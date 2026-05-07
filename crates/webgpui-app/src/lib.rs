#![warn(missing_docs)]
//! Top-level application integration crate for webgpui.
//!
//! Provides a simple, ergonomic API for building GPU-rendered UI applications:
//!
//! ```no_run
//! use webgpui_app::{App, AppBuilder, DrawContext};
//! use webgpui_geometry::{Color, Rect};
//!
//! let app = AppBuilder::new()
//!     .title("Hello webgpui")
//!     .size(800, 600)
//!     .background(Color::from_rgb_u8(30, 30, 30))
//!     .build();
//!
//! app.run(|ctx| {
//!     ctx.fill_rect(Rect::new(50.0, 50.0, 200.0, 100.0), Color::BLUE);
//! });
//! ```

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use thiserror::Error;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};

use webgpui_core::{DirtyTracker, NodeTree};
use webgpui_geometry::{Color, Point, Rect, Size};
use webgpui_input::{FocusManager, InputEvent, InputState, Modifiers};

// Re-export types that application code commonly needs.
pub use webgpui_input::{KeyCode, MouseButton};
use webgpui_profiler::FrameTimer;
pub use webgpui_render::BackendSelector;
pub use webgpui_render::ImageHandle;
use webgpui_render::{DrawList, RenderError, Renderer};
use webgpui_render_graph::ClearColor;
use webgpui_render_wgpu::{PendingImage, WgpuContext, WgpuRenderer};

// ---------------------------------------------------------------------------
// BackendSwitcher
// ---------------------------------------------------------------------------

/// Handle for switching the rendering backend at runtime.
///
/// Create with [`BackendSwitcher::new`], pass a clone to
/// [`AppBuilder::backend_switcher`], and call [`switch_to`][BackendSwitcher::switch_to]
/// from your frame callback.
pub struct BackendSwitcher {
    state: Arc<Mutex<BackendState>>,
}

impl std::fmt::Debug for BackendSwitcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.state.lock().unwrap();
        f.debug_struct("BackendSwitcher")
            .field("current", &state.current)
            .field("pending", &state.pending)
            .finish()
    }
}

struct BackendState {
    current: BackendSelector,
    pending: Option<BackendSelector>,
}

impl Clone for BackendSwitcher {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

impl BackendSwitcher {
    /// Creates a new switcher with the given initial backend.
    pub fn new(initial: BackendSelector) -> Self {
        Self {
            state: Arc::new(Mutex::new(BackendState {
                current: initial,
                pending: None,
            })),
        }
    }

    /// Requests a switch to `backend` on the next frame.
    pub fn switch_to(&self, backend: BackendSelector) {
        self.state.lock().unwrap().pending = Some(backend);
    }

    /// Returns the currently active backend.
    pub fn current(&self) -> BackendSelector {
        self.state.lock().unwrap().current
    }

    /// Takes the pending switch (if any), applies it, and returns it.
    fn take_pending(&self) -> Option<BackendSelector> {
        let mut state = self.state.lock().unwrap();
        if let Some(backend) = state.pending.take() {
            state.current = backend;
            Some(backend)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur while building or running an application.
#[derive(Debug, Error)]
pub enum AppError {
    /// The OS event loop failed to start or dispatch.
    #[error("event loop error: {0}")]
    EventLoop(String),
    /// The platform window could not be created.
    #[error("window creation failed: {0}")]
    WindowCreation(String),
    /// The GPU renderer failed to initialise.
    #[error("renderer initialisation failed: {0}")]
    RendererInit(String),
    /// A per-frame render error from the backend.
    #[error("render error: {0}")]
    Render(#[from] RenderError),
    /// Failed to load or decode an image file.
    #[error("image load error: {0}")]
    ImageLoad(String),
}

// ---------------------------------------------------------------------------
// ImageRegistry
// ---------------------------------------------------------------------------

/// Per-application image loading and caching registry.
///
/// Obtained via [`DrawContext::images`]. Call [`load`][Self::load] to decode a
/// PNG/JPEG file and obtain an [`ImageHandle`] that can be passed to
/// [`DrawContext::draw_image`].
pub struct ImageRegistry {
    next_id: u32,
    /// Already-registered paths → handle (avoids re-decoding across frames).
    loaded: std::collections::HashMap<String, ImageHandle>,
    /// Newly decoded images waiting for GPU upload.
    pending: Vec<PendingImage>,
}

impl ImageRegistry {
    fn new() -> Self {
        Self {
            next_id: 0,
            loaded: std::collections::HashMap::new(),
            pending: Vec::new(),
        }
    }

    /// Loads a PNG or JPEG image from `path` and returns a reusable [`ImageHandle`].
    ///
    /// If the same path has been loaded before, the cached handle is returned
    /// without re-reading the file.
    pub fn load(&mut self, path: impl AsRef<std::path::Path>) -> Result<ImageHandle, AppError> {
        let key = path.as_ref().to_string_lossy().into_owned();
        if let Some(&handle) = self.loaded.get(&key) {
            return Ok(handle);
        }
        let img = image::open(path.as_ref())
            .map_err(|e| AppError::ImageLoad(e.to_string()))?
            .into_rgba8();
        let (w, h) = img.dimensions();
        let id = self.next_id;
        self.next_id += 1;
        self.pending.push(PendingImage {
            id,
            pixels: img.into_raw(),
            width: w,
            height: h,
        });
        let handle = ImageHandle(id);
        self.loaded.insert(key, handle);
        Ok(handle)
    }

    /// Drains all pending (not yet GPU-uploaded) images.
    fn take_pending(&mut self) -> Vec<PendingImage> {
        std::mem::take(&mut self.pending)
    }
}

/// Convenience alias for `Result<T, AppError>`.
pub type AppResult<T> = Result<T, AppError>;

// ---------------------------------------------------------------------------
// DrawContext – per-frame drawing API
// ---------------------------------------------------------------------------

/// A per-frame drawing context handed to the user callback.
///
/// Wrap the underlying [`DrawList`] with convenience methods.
pub struct DrawContext<'a> {
    draw_list: &'a mut DrawList,
    /// Current viewport size in logical pixels.
    pub viewport: Size,
    /// Snapshot of the current input state.
    pub input: &'a InputState,
    /// The currently active rendering backend.
    pub current_backend: BackendSelector,
    /// The application node tree.
    pub node_tree: &'a mut NodeTree,
    /// Dirty-region tracker for the current frame.
    pub dirty: &'a mut DirtyTracker,
    /// Focus state manager.
    pub focus: &'a mut FocusManager,
    /// Image loading and caching registry.
    pub images: &'a mut ImageRegistry,
}

impl<'a> DrawContext<'a> {
    fn new(
        draw_list: &'a mut DrawList,
        viewport: Size,
        input: &'a InputState,
        current_backend: BackendSelector,
        node_tree: &'a mut NodeTree,
        dirty: &'a mut DirtyTracker,
        focus: &'a mut FocusManager,
        images: &'a mut ImageRegistry,
    ) -> Self {
        Self {
            draw_list,
            viewport,
            input,
            current_backend,
            node_tree,
            dirty,
            focus,
            images,
        }
    }

    /// Fills `rect` with a solid `color`.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.draw_list.fill_rect(rect, color);
    }

    /// Fills a rectangle with rounded corners.
    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.draw_list
            .fill_rounded_rect(rect, webgpui_geometry::BorderRadius::all(radius), color);
    }

    /// Draws a rectangular border.
    pub fn draw_border(&mut self, rect: Rect, color: Color, width: f32) {
        self.draw_list.draw_border(rect, color, width);
    }

    /// Fills a rectangle spanning the entire viewport.
    pub fn fill_background(&mut self, color: Color) {
        self.fill_rect(
            Rect::from_origin_size(webgpui_geometry::Point::ZERO, self.viewport),
            color,
        );
    }

    /// Returns the underlying draw list for advanced usage.
    pub fn draw_list(&mut self) -> &mut DrawList {
        self.draw_list
    }

    /// Loads a PNG or JPEG image from `path` and returns a reusable [`ImageHandle`].
    ///
    /// Delegates to [`ImageRegistry::load`]; returns a cached handle on subsequent calls.
    pub fn load_image(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<ImageHandle, AppError> {
        self.images.load(path)
    }

    /// Draws a previously loaded image filling `rect`.
    pub fn draw_image(&mut self, rect: Rect, handle: ImageHandle) {
        self.draw_list.draw_image(rect, handle);
    }
}

// ---------------------------------------------------------------------------
// AppConfig
// ---------------------------------------------------------------------------

/// Configuration for the application window and renderer.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Window title shown in the OS title bar.
    pub title: String,
    /// Initial window width in logical pixels.
    pub width: u32,
    /// Initial window height in logical pixels.
    pub height: u32,
    /// Whether the user can resize the window.
    pub resizable: bool,
    /// Enable GPU present-mode vsync.
    pub vsync: bool,
    /// Optional application-level frame cap.
    ///
    /// * `Some(n)`: cap redraw scheduling to `n` FPS.
    /// * `None`: no app-side cap (redraw as often as possible).
    pub target_fps: Option<u32>,
    /// Default background clear colour.
    pub background: Color,
    /// Optional backend switcher for runtime backend switching.
    pub backend_switcher: Option<BackendSwitcher>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: String::from("webgpui"),
            width: 800,
            height: 600,
            resizable: true,
            vsync: true,
            target_fps: Some(60),
            background: Color::BLACK,
            backend_switcher: None,
        }
    }
}

// ---------------------------------------------------------------------------
// AppBuilder
// ---------------------------------------------------------------------------

/// Fluent builder for [`App`].
#[derive(Default)]
pub struct AppBuilder {
    config: AppConfig,
}

impl AppBuilder {
    /// Creates a builder with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the window title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    /// Sets the initial window size in logical pixels.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    /// Controls whether the window can be resized by the user.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.config.resizable = resizable;
        self
    }

    /// Enables or disables GPU present-mode vsync.
    pub fn vsync(mut self, vsync: bool) -> Self {
        self.config.vsync = vsync;
        self
    }

    /// Sets an application-side redraw cap.
    ///
    /// This is independent from GPU present-mode vsync.
    /// Set to `None` for uncapped redraw scheduling.
    pub fn target_fps(mut self, target_fps: Option<u32>) -> Self {
        if let Some(fps) = target_fps {
            assert!(
                fps > 0,
                "target_fps must be > 0; use None for uncapped rendering"
            );
        }
        self.config.target_fps = target_fps;
        self
    }

    /// Sets the default background clear colour.
    pub fn background(mut self, color: Color) -> Self {
        self.config.background = color;
        self
    }

    /// Attaches a [`BackendSwitcher`] enabling runtime backend switching.
    pub fn backend_switcher(mut self, switcher: BackendSwitcher) -> Self {
        self.config.backend_switcher = Some(switcher);
        self
    }

    /// Consumes the builder and returns a configured [`App`].
    pub fn build(self) -> App {
        App {
            config: self.config,
        }
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

/// The top-level application.
///
/// Call [`App::run`] with a frame callback to start the event loop.
pub struct App {
    config: AppConfig,
}

impl App {
    /// Starts the application event loop.
    ///
    /// `frame_fn` is called every frame with a [`DrawContext`].  It should
    /// issue draw commands; the engine handles the rest.
    pub fn run<F>(self, mut frame_fn: F) -> AppResult<()>
    where
        F: FnMut(&mut DrawContext<'_>) + 'static,
    {
        let _ = env_logger::try_init();

        let event_loop = EventLoop::new().map_err(|e| AppError::EventLoop(e.to_string()))?;

        let window_title = self.config.title.clone();

        let window = Arc::new(
            WindowBuilder::new()
                .with_title(&window_title)
                .with_inner_size(LogicalSize::new(self.config.width, self.config.height))
                .with_resizable(self.config.resizable)
                .build(&event_loop)
                .map_err(|e| AppError::WindowCreation(e.to_string()))?,
        );

        let bg = self.config.background;
        let backend_switcher = self.config.backend_switcher;
        let mut current_backend = backend_switcher
            .as_ref()
            .map(|s| s.current())
            .unwrap_or(BackendSelector::Wgpu);

        let ctx = WgpuContext::new(
            Arc::clone(&window),
            self.config.width.max(1),
            self.config.height.max(1),
            self.config.vsync,
        )
        .map_err(AppError::RendererInit)?;

        let cpu_fallback = matches!(ctx.adapter_info.device_type, wgpu::DeviceType::Cpu);
        if cpu_fallback {
            window.set_title(&format!("{} [CPU Fallback Active]", window_title));
            log::warn!(
                "[app] CPU fallback active: adapter='{}' backend={:?} driver='{}'",
                ctx.adapter_info.name,
                ctx.adapter_info.backend,
                ctx.adapter_info.driver,
            );
            eprintln!(
                "[webgpui-app] CPU fallback active: adapter='{}' backend={:?} driver='{}'",
                ctx.adapter_info.name, ctx.adapter_info.backend, ctx.adapter_info.driver,
            );
        }

        let mut renderer = WgpuRenderer::new(ctx);
        renderer.render_graph_mut().set_clear_color(ClearColor::new(
            bg.r as f64,
            bg.g as f64,
            bg.b as f64,
            bg.a as f64,
        ));

        // Side renderer for non-wgpu backends (CPU, CUDA placeholder).
        let mut side_renderer: Option<Box<dyn Renderer>> = create_side_renderer(
            current_backend,
            self.config.width.max(1),
            self.config.height.max(1),
        );

        let mut draw_list = DrawList::new();
        let mut input_state = InputState::new();
        let mut frame_timer = FrameTimer::new(120);
        let mut node_tree = NodeTree::new();
        let mut dirty_tracker = DirtyTracker::new();
        let mut focus = FocusManager::new();
        let mut image_registry = ImageRegistry::new();
        let mut cursor_pos = Point::ZERO;
        let frame_interval = self.config.target_fps.and_then(|fps| {
            if fps == 0 {
                None
            } else {
                Some(Duration::from_secs_f64(1.0 / fps as f64))
            }
        });
        let mut last_perf_warn_at = Instant::now();

        // Track whether we actually entered rendering and whether the close was
        // user-initiated; this lets us classify ExitFailure(1) more accurately.
        let has_rendered_frame = Arc::new(AtomicBool::new(false));
        let close_requested = Arc::new(AtomicBool::new(false));
        let has_rendered_frame_inner = Arc::clone(&has_rendered_frame);
        let close_requested_inner = Arc::clone(&close_requested);

        if let Some(interval) = frame_interval {
            event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + interval));
        } else {
            event_loop.set_control_flow(ControlFlow::Poll);
        }
        let run_result = event_loop.run(move |event, elwt| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => {
                            close_requested_inner.store(true, Ordering::Relaxed);
                            elwt.exit();
                        }
                        WindowEvent::Resized(ps) => {
                            renderer.resize(ps.width.max(1), ps.height.max(1));
                            if let Some(ref mut sr) = side_renderer {
                                sr.resize(ps.width.max(1), ps.height.max(1));
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            has_rendered_frame_inner.store(true, Ordering::Relaxed);
                            frame_timer.begin_frame();

                            // Apply any pending backend switch.
                            if let Some(ref sw) = backend_switcher {
                                if let Some(new_backend) = sw.take_pending() {
                                    current_backend = new_backend;
                                    let title =
                                        format!("{} [{}]", window_title, current_backend.name());
                                    window.set_title(&title);
                                    let (w, h) = renderer.surface_size();
                                    side_renderer = create_side_renderer(current_backend, w, h);
                                    frame_timer.reset();
                                    dirty_tracker.mark_all();
                                    eprintln!(
                                        "[webgpui-app] backend switched to: {}",
                                        current_backend.name()
                                    );
                                }
                            }

                            let (sw, sh) = renderer.surface_size();
                            let viewport = Size::new(sw as f32, sh as f32);
                            draw_list.clear();
                            let mut ctx = DrawContext::new(
                                &mut draw_list,
                                viewport,
                                &input_state,
                                current_backend,
                                &mut node_tree,
                                &mut dirty_tracker,
                                &mut focus,
                                &mut image_registry,
                            );
                            frame_fn(&mut ctx);

                            // Upload any newly decoded images to the GPU.
                            let pending = image_registry.take_pending();
                            if !pending.is_empty() {
                                renderer.upload_images(pending);
                            }

                            // Run the active side renderer (validation / side effects).
                            if let Some(ref mut sr) = side_renderer {
                                let _ = sr.render(&draw_list);
                            }

                            match renderer.render(&draw_list) {
                                Ok(()) => {}
                                Err(RenderError::SurfaceLost) => {
                                    let (w, h) = renderer.surface_size();
                                    renderer.resize(w, h);
                                }
                                Err(e) => {
                                    log::error!("[app] render error: {}", e);
                                }
                            }

                            if let Some(ms) = frame_timer.end_frame() {
                                if ms > 20.0
                                    && last_perf_warn_at.elapsed() >= Duration::from_secs(1)
                                {
                                    log::warn!("[app] slow frame: {:.2}ms", ms);
                                    frame_timer.check_thresholds(16.6, 20.0);
                                    last_perf_warn_at = Instant::now();
                                }
                            }
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            let sf = window.scale_factor();
                            let lp = position.to_logical::<f32>(sf);
                            cursor_pos = Point::new(lp.x, lp.y);
                            input_state.apply(&InputEvent::MouseMoved {
                                position: cursor_pos,
                            });
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            let mb = convert_mouse_button(button);
                            let ev = match state {
                                ElementState::Pressed => InputEvent::MousePressed {
                                    button: mb,
                                    position: cursor_pos,
                                    modifiers: Modifiers::none(),
                                },
                                ElementState::Released => InputEvent::MouseReleased {
                                    button: mb,
                                    position: cursor_pos,
                                    modifiers: Modifiers::none(),
                                },
                            };
                            input_state.apply(&ev);
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            let (dx, dy) = match delta {
                                MouseScrollDelta::LineDelta(x, y) => (*x * 20.0, *y * 20.0),
                                MouseScrollDelta::PixelDelta(p) => {
                                    let sf = window.scale_factor();
                                    let lp = p.to_logical::<f32>(sf);
                                    (lp.x, lp.y)
                                }
                            };
                            input_state.apply(&InputEvent::MouseScrolled {
                                position: cursor_pos,
                                delta_x: dx,
                                delta_y: dy,
                                modifiers: Modifiers::none(),
                            });
                        }
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    logical_key,
                                    state,
                                    text,
                                    ..
                                },
                            ..
                        } => {
                            let key = convert_key(logical_key);
                            let ev = match state {
                                ElementState::Pressed => InputEvent::KeyPressed {
                                    key,
                                    modifiers: Modifiers::none(),
                                },
                                ElementState::Released => InputEvent::KeyReleased {
                                    key,
                                    modifiers: Modifiers::none(),
                                },
                            };
                            input_state.apply(&ev);
                            if *state == ElementState::Pressed {
                                if let Some(s) = text {
                                    for ch in s.chars() {
                                        input_state.apply(&InputEvent::CharInput { ch });
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    // Optionally pace redraw to avoid saturating CPU fallback adapters.
                    if let Some(interval) = frame_interval {
                        elwt.set_control_flow(ControlFlow::WaitUntil(Instant::now() + interval));
                    } else {
                        elwt.set_control_flow(ControlFlow::Poll);
                    }
                    window.request_redraw();
                }
                _ => {}
            }
        });

        match run_result {
            Ok(()) => Ok(()),
            Err(e) => {
                let msg = e.to_string();
                let is_exit_failure = msg.contains("Exit Failure") || msg.contains("ExitFailure");

                // Normal path on Linux/Wayland when user closes the window.
                if close_requested.load(Ordering::Relaxed) && is_exit_failure {
                    return Ok(());
                }

                // If we never reached a redraw and immediately got ExitFailure,
                // report this as a display-backend disconnect for easier diagnosis.
                if !has_rendered_frame.load(Ordering::Relaxed) && is_exit_failure {
                    return Err(AppError::EventLoop(
                        "display backend disconnected before first frame (e.g. WSLg/Wayland pipe closed)".to_string(),
                    ));
                }

                Err(AppError::EventLoop(msg))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Key / button conversion helpers
// ---------------------------------------------------------------------------

fn convert_mouse_button(button: &winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Other(n) => MouseButton::Other(*n),
        _ => MouseButton::Other(0),
    }
}

fn convert_key(key: &Key) -> KeyCode {
    match key {
        Key::Named(named) => convert_named_key(named),
        Key::Character(s) => {
            let Some(ch) = s.chars().next() else {
                return KeyCode::Unknown;
            };
            let upper = ch.to_ascii_uppercase();
            if upper.is_ascii_alphabetic() {
                const LETTERS: [KeyCode; 26] = [
                    KeyCode::A,
                    KeyCode::B,
                    KeyCode::C,
                    KeyCode::D,
                    KeyCode::E,
                    KeyCode::F,
                    KeyCode::G,
                    KeyCode::H,
                    KeyCode::I,
                    KeyCode::J,
                    KeyCode::K,
                    KeyCode::L,
                    KeyCode::M,
                    KeyCode::N,
                    KeyCode::O,
                    KeyCode::P,
                    KeyCode::Q,
                    KeyCode::R,
                    KeyCode::S,
                    KeyCode::T,
                    KeyCode::U,
                    KeyCode::V,
                    KeyCode::W,
                    KeyCode::X,
                    KeyCode::Y,
                    KeyCode::Z,
                ];
                LETTERS[(upper as u8 - b'A') as usize]
            } else if ch.is_ascii_digit() {
                const DIGITS: [KeyCode; 10] = [
                    KeyCode::Digit0,
                    KeyCode::Digit1,
                    KeyCode::Digit2,
                    KeyCode::Digit3,
                    KeyCode::Digit4,
                    KeyCode::Digit5,
                    KeyCode::Digit6,
                    KeyCode::Digit7,
                    KeyCode::Digit8,
                    KeyCode::Digit9,
                ];
                DIGITS[(ch as u8 - b'0') as usize]
            } else {
                KeyCode::Unknown
            }
        }
        _ => KeyCode::Unknown,
    }
}

/// Creates a side renderer for non-wgpu backends, or `None` for wgpu (which is always
/// the primary display renderer).
fn create_side_renderer(backend: BackendSelector, w: u32, h: u32) -> Option<Box<dyn Renderer>> {
    #[cfg(feature = "backend-cpu")]
    if backend == BackendSelector::Cpu {
        if let Ok(r) = webgpui_render_cpu::CpuRenderer::new(w, h) {
            return Some(Box::new(r));
        }
    }
    let _ = (backend, w, h);
    None
}

fn convert_named_key(key: &NamedKey) -> KeyCode {
    match key {
        NamedKey::Space => KeyCode::Space,
        NamedKey::Enter => KeyCode::Enter,
        NamedKey::Tab => KeyCode::Tab,
        NamedKey::Backspace => KeyCode::Backspace,
        NamedKey::Escape => KeyCode::Escape,
        NamedKey::Delete => KeyCode::Delete,
        NamedKey::ArrowLeft => KeyCode::ArrowLeft,
        NamedKey::ArrowRight => KeyCode::ArrowRight,
        NamedKey::ArrowUp => KeyCode::ArrowUp,
        NamedKey::ArrowDown => KeyCode::ArrowDown,
        NamedKey::Home => KeyCode::Home,
        NamedKey::End => KeyCode::End,
        NamedKey::PageUp => KeyCode::PageUp,
        NamedKey::PageDown => KeyCode::PageDown,
        NamedKey::Shift => KeyCode::Shift,
        NamedKey::Control => KeyCode::Control,
        NamedKey::Alt => KeyCode::Alt,
        NamedKey::Meta => KeyCode::Meta,
        NamedKey::F1 => KeyCode::F1,
        NamedKey::F2 => KeyCode::F2,
        NamedKey::F3 => KeyCode::F3,
        NamedKey::F4 => KeyCode::F4,
        NamedKey::F5 => KeyCode::F5,
        NamedKey::F6 => KeyCode::F6,
        NamedKey::F7 => KeyCode::F7,
        NamedKey::F8 => KeyCode::F8,
        NamedKey::F9 => KeyCode::F9,
        NamedKey::F10 => KeyCode::F10,
        NamedKey::F11 => KeyCode::F11,
        NamedKey::F12 => KeyCode::F12,
        _ => KeyCode::Unknown,
    }
}
