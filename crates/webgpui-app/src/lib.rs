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
use webgpui_render::{DrawList, RenderError, Renderer};
use webgpui_render_graph::ClearColor;
use webgpui_render_wgpu::{WgpuContext, WgpuRenderer};

// ---------------------------------------------------------------------------
// BackendSwitcher
// ---------------------------------------------------------------------------

/// Handle for switching the rendering backend at runtime.
///
/// Create with [`BackendSwitcher::new`], pass a clone to
/// [`AppBuilder::backend_switcher`], and call [`switch_to`][BackendSwitcher::switch_to]
/// from your frame callback.
#[derive(Clone, Debug)]
pub struct BackendSwitcher {
    current: Arc<Mutex<BackendSelector>>,
    pending: Arc<Mutex<Option<BackendSelector>>>,
}

impl BackendSwitcher {
    /// Creates a new switcher with the given initial backend.
    pub fn new(initial: BackendSelector) -> Self {
        Self {
            current: Arc::new(Mutex::new(initial)),
            pending: Arc::new(Mutex::new(None)),
        }
    }

    /// Requests a switch to `backend` on the next frame.
    pub fn switch_to(&self, backend: BackendSelector) {
        *self.pending.lock().unwrap() = Some(backend);
    }

    /// Returns the currently active backend.
    pub fn current(&self) -> BackendSelector {
        *self.current.lock().unwrap()
    }

    /// Takes the pending switch (if any), applies it, and returns it.
    fn take_pending(&self) -> Option<BackendSelector> {
        let mut pending = self.pending.lock().unwrap();
        if let Some(backend) = pending.take() {
            *self.current.lock().unwrap() = backend;
            return Some(backend);
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum AppError {
    #[error("event loop error: {0}")]
    EventLoop(String),
    #[error("window creation failed: {0}")]
    WindowCreation(String),
    #[error("renderer initialisation failed: {0}")]
    RendererInit(String),
    #[error("render error: {0}")]
    Render(#[from] RenderError),
}

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
}

impl<'a> DrawContext<'a> {
    fn new(
        draw_list: &'a mut DrawList,
        viewport: Size,
        input: &'a InputState,
        current_backend: BackendSelector,
    ) -> Self {
        Self {
            draw_list,
            viewport,
            input,
            current_backend,
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
}

// ---------------------------------------------------------------------------
// AppConfig
// ---------------------------------------------------------------------------

/// Configuration for the application window and renderer.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub vsync: bool,
    /// Optional application-level frame cap.
    ///
    /// * `Some(n)`: cap redraw scheduling to `n` FPS.
    /// * `None`: no app-side cap (redraw as often as possible).
    pub target_fps: Option<u32>,
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.config.resizable = resizable;
        self
    }

    pub fn vsync(mut self, vsync: bool) -> Self {
        self.config.vsync = vsync;
        self
    }

    /// Sets an application-side redraw cap.
    ///
    /// This is independent from GPU present-mode vsync.
    /// Set to `None` for uncapped redraw scheduling.
    pub fn target_fps(mut self, target_fps: Option<u32>) -> Self {
        self.config.target_fps = target_fps;
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.config.background = color;
        self
    }

    /// Attaches a [`BackendSwitcher`] enabling runtime backend switching.
    pub fn backend_switcher(mut self, switcher: BackendSwitcher) -> Self {
        self.config.backend_switcher = Some(switcher);
        self
    }

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
        let _node_tree = NodeTree::new();
        let _dirty_tracker = DirtyTracker::new();
        let _focus = FocusManager::new();
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
                            );
                            frame_fn(&mut ctx);

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
        Key::Character(s) => match s.as_str() {
            "a" | "A" => KeyCode::A,
            "b" | "B" => KeyCode::B,
            "c" | "C" => KeyCode::C,
            "d" | "D" => KeyCode::D,
            "e" | "E" => KeyCode::E,
            "f" | "F" => KeyCode::F,
            "g" | "G" => KeyCode::G,
            "h" | "H" => KeyCode::H,
            "i" | "I" => KeyCode::I,
            "j" | "J" => KeyCode::J,
            "k" | "K" => KeyCode::K,
            "l" | "L" => KeyCode::L,
            "m" | "M" => KeyCode::M,
            "n" | "N" => KeyCode::N,
            "o" | "O" => KeyCode::O,
            "p" | "P" => KeyCode::P,
            "q" | "Q" => KeyCode::Q,
            "r" | "R" => KeyCode::R,
            "s" | "S" => KeyCode::S,
            "t" | "T" => KeyCode::T,
            "u" | "U" => KeyCode::U,
            "v" | "V" => KeyCode::V,
            "w" | "W" => KeyCode::W,
            "x" | "X" => KeyCode::X,
            "y" | "Y" => KeyCode::Y,
            "z" | "Z" => KeyCode::Z,
            "0" => KeyCode::Digit0,
            "1" => KeyCode::Digit1,
            "2" => KeyCode::Digit2,
            "3" => KeyCode::Digit3,
            "4" => KeyCode::Digit4,
            "5" => KeyCode::Digit5,
            "6" => KeyCode::Digit6,
            "7" => KeyCode::Digit7,
            "8" => KeyCode::Digit8,
            "9" => KeyCode::Digit9,
            _ => KeyCode::Unknown,
        },
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
