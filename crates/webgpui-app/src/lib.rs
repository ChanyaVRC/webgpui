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

use webgpui_core::{DirtyTracker, NodeStyle, NodeTree, TransitionConfig};
use webgpui_geometry::{Color, Point, Rect, Size};
use webgpui_input::{FocusManager, InputEvent, InputState, Modifiers};

// Re-export types that application code commonly needs.
pub use webgpui_core::NodeId;
pub use webgpui_input::{KeyCode, MouseButton};
use webgpui_profiler::FrameTimer;
pub use webgpui_render::BackendSelector;
pub use webgpui_render::ImageHandle;
use webgpui_render::{DrawList, RenderError, Renderer};
use webgpui_render_graph::ClearColor;
use webgpui_render_wgpu::{PendingImage, WgpuContext, WgpuRenderer};

// ---------------------------------------------------------------------------
// Animation
// ---------------------------------------------------------------------------

/// Easing curve for an [`Animation`].
///
/// [`Easing::sample`] maps a normalized time `t ∈ [0, 1]` to an output
/// value in roughly `[0, 1]`.
///
/// # Example
///
/// ```
/// use webgpui_app::Easing;
///
/// assert_eq!(Easing::Linear.sample(0.5), 0.5);
/// assert!(Easing::EaseIn.sample(0.5) < 0.5);
/// assert!(Easing::EaseOut.sample(0.5) > 0.5);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Easing {
    /// Constant-rate interpolation.
    Linear,
    /// Slow start, fast end (cubic).
    EaseIn,
    /// Fast start, slow end (cubic).
    EaseOut,
    /// Slow start and end, fast middle (cubic).
    EaseInOut,
    /// Custom cubic bézier with control points `(x1, y1)` and `(x2, y2)`,
    /// matching the CSS `cubic-bezier()` function.
    CubicBezier(f32, f32, f32, f32),
}

impl Easing {
    /// Samples the easing curve at normalized time `t ∈ [0.0, 1.0]`.
    ///
    /// Values outside `[0, 1]` are clamped before sampling.
    pub fn sample(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t * t,
            Self::EaseOut => 1.0 - (1.0 - t).powi(3),
            Self::EaseInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0_f32).powi(3) / 2.0
                }
            }
            Self::CubicBezier(x1, y1, x2, y2) => cubic_bezier_sample(*x1, *y1, *x2, *y2, t),
        }
    }
}

/// Samples a CSS cubic-bézier easing curve.
///
/// Solves `x(s) = t` via 16-iteration binary search (sufficient for 60 fps),
/// then evaluates `y(s)`.
fn cubic_bezier_sample(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let bx = |s: f32| 3.0 * x1 * s * (1.0 - s).powi(2) + 3.0 * x2 * s * s * (1.0 - s) + s * s * s;
    let by = |s: f32| 3.0 * y1 * s * (1.0 - s).powi(2) + 3.0 * y2 * s * s * (1.0 - s) + s * s * s;
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for _ in 0..16 {
        let mid = (lo + hi) * 0.5;
        if bx(mid) < t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    by((lo + hi) * 0.5)
}

/// The style property targeted by an [`Animation`].
#[derive(Debug, Clone, PartialEq)]
pub enum AnimatedProperty {
    /// Animate [`NodeStyle::opacity`] from `from` to `to`.
    Opacity {
        /// Starting opacity.
        from: f32,
        /// Target opacity.
        to: f32,
    },
    /// Animate [`NodeStyle::translate_x`] from `from` to `to` logical pixels.
    TranslateX {
        /// Starting X offset.
        from: f32,
        /// Target X offset.
        to: f32,
    },
    /// Animate [`NodeStyle::translate_y`] from `from` to `to` logical pixels.
    TranslateY {
        /// Starting Y offset.
        from: f32,
        /// Target Y offset.
        to: f32,
    },
}

/// A one-shot animation that interpolates a node's style property over time.
///
/// Construct with one of the factory methods, then chain `.duration_ms` and
/// `.easing` before passing to [`DrawContext::start_animation`].
///
/// # Example
///
/// ```no_run
/// use webgpui_app::{Animation, Easing, NodeId};
///
/// let anim = Animation::opacity(NodeId::ROOT, 0.0, 1.0)
///     .duration_ms(400.0)
///     .easing(Easing::EaseOut);
/// ```
#[derive(Debug, Clone)]
pub struct Animation {
    pub(crate) node_id: NodeId,
    pub(crate) property: AnimatedProperty,
    pub(crate) duration_ms: f64,
    pub(crate) easing: Easing,
}

impl Animation {
    /// Animates [`NodeStyle::opacity`] from `from` to `to`.
    pub fn opacity(node_id: NodeId, from: f32, to: f32) -> Self {
        Self {
            node_id,
            property: AnimatedProperty::Opacity { from, to },
            duration_ms: 300.0,
            easing: Easing::Linear,
        }
    }

    /// Animates [`NodeStyle::translate_x`] from `from` to `to` logical pixels.
    pub fn translate_x(node_id: NodeId, from: f32, to: f32) -> Self {
        Self {
            node_id,
            property: AnimatedProperty::TranslateX { from, to },
            duration_ms: 300.0,
            easing: Easing::Linear,
        }
    }

    /// Animates [`NodeStyle::translate_y`] from `from` to `to` logical pixels.
    pub fn translate_y(node_id: NodeId, from: f32, to: f32) -> Self {
        Self {
            node_id,
            property: AnimatedProperty::TranslateY { from, to },
            duration_ms: 300.0,
            easing: Easing::Linear,
        }
    }

    /// Sets the animation duration in milliseconds (default: 300).
    pub fn duration_ms(mut self, ms: f64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Sets the easing curve (default: [`Easing::Linear`]).
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

// Internal record of a running animation.
struct RunningAnimation {
    node_id: NodeId,
    property: AnimatedProperty,
    easing: Easing,
    start: Instant,
    duration: Duration,
}

/// Manages all active animations for an [`App`].
///
/// Owned by `App::run` and passed by mutable reference to [`DrawContext`].
/// Users interact with it via [`DrawContext::start_animation`] and
/// [`DrawContext::set_style`].
pub(crate) struct AnimationTimeline {
    active: Vec<RunningAnimation>,
}

impl AnimationTimeline {
    fn new() -> Self {
        Self { active: Vec::new() }
    }

    /// Enqueues an animation to start immediately.
    pub(crate) fn start(&mut self, anim: Animation) {
        self.active.push(RunningAnimation {
            node_id: anim.node_id,
            property: anim.property,
            easing: anim.easing,
            start: Instant::now(),
            duration: Duration::from_secs_f64(anim.duration_ms / 1000.0),
        });
    }

    /// Returns `true` if at least one animation is still running.
    pub(crate) fn has_active(&self) -> bool {
        !self.active.is_empty()
    }

    /// Advances all animations, writes interpolated values into `node_tree`,
    /// and calls `dirty.mark_all()` for every frame that has active animations.
    ///
    /// Completed animations are removed from the active list.
    pub(crate) fn tick(&mut self, node_tree: &mut NodeTree, dirty: &mut DirtyTracker) {
        if self.active.is_empty() {
            return;
        }
        let now = Instant::now();
        let mut i = 0;
        while i < self.active.len() {
            // Extract what we need without keeping a borrow on self.active.
            let (node_id, v, done) = {
                let anim = &self.active[i];
                let t = if anim.duration.is_zero() {
                    1.0_f32
                } else {
                    (now.duration_since(anim.start).as_secs_f64() / anim.duration.as_secs_f64())
                        .min(1.0) as f32
                };
                (anim.node_id, anim.easing.sample(t), t >= 1.0)
            };
            // Clone the current style and apply the interpolated value.
            let style_opt = node_tree.get(node_id).map(|n| n.style.clone());
            if let Some(mut style) = style_opt {
                match &self.active[i].property {
                    AnimatedProperty::Opacity { from, to } => {
                        style.opacity = from + (to - from) * v;
                    }
                    AnimatedProperty::TranslateX { from, to } => {
                        style.translate_x = from + (to - from) * v;
                    }
                    AnimatedProperty::TranslateY { from, to } => {
                        style.translate_y = from + (to - from) * v;
                    }
                }
                node_tree.set_style(node_id, style);
                // Mark full frame dirty so the renderer repaints this frame.
                // P2 integration: replace with per-node rect when layout rects
                // are queryable from the animation system.
                dirty.mark_all();
            }
            if done {
                self.active.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Creates implicit transition animations for any animatable properties
    /// that differ between `old` and `new`.
    pub(crate) fn create_transitions(
        &mut self,
        node_id: NodeId,
        old: &NodeStyle,
        new: &NodeStyle,
        config: &TransitionConfig,
    ) {
        let dur = config.duration_ms;
        if (old.opacity - new.opacity).abs() > 1e-4 {
            self.start(
                Animation::opacity(node_id, old.opacity, new.opacity)
                    .duration_ms(dur)
                    .easing(Easing::EaseInOut),
            );
        }
        if (old.translate_x - new.translate_x).abs() > 1e-4 {
            self.start(
                Animation::translate_x(node_id, old.translate_x, new.translate_x)
                    .duration_ms(dur)
                    .easing(Easing::EaseInOut),
            );
        }
        if (old.translate_y - new.translate_y).abs() > 1e-4 {
            self.start(
                Animation::translate_y(node_id, old.translate_y, new.translate_y)
                    .duration_ms(dur)
                    .easing(Easing::EaseInOut),
            );
        }
    }
}

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
    /// Failed to load or rasterize an SVG file.
    #[error("SVG load error: {0}")]
    SvgLoad(String),
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

    /// Loads and rasterizes an SVG from `path` at `width × height` pixels.
    ///
    /// The result is cached by `(path, width, height)` — subsequent calls with
    /// the same arguments return the cached handle without re-rasterizing.
    pub fn load_svg(
        &mut self,
        path: impl AsRef<std::path::Path>,
        width: u32,
        height: u32,
    ) -> Result<ImageHandle, AppError> {
        let key = format!(
            "svg:{}@{}x{}",
            path.as_ref().to_string_lossy(),
            width,
            height
        );
        if let Some(&handle) = self.loaded.get(&key) {
            return Ok(handle);
        }
        let pixels = rasterize_svg(path.as_ref(), width, height)?;
        let id = self.next_id;
        self.next_id += 1;
        self.pending.push(PendingImage {
            id,
            pixels,
            width,
            height,
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

/// Rasterizes an SVG file to raw RGBA8 pixels at the given dimensions.
fn rasterize_svg(path: &std::path::Path, width: u32, height: u32) -> Result<Vec<u8>, AppError> {
    let data =
        std::fs::read(path).map_err(|e| AppError::SvgLoad(format!("{}: {}", path.display(), e)))?;
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&data, &options)
        .map_err(|e| AppError::SvgLoad(e.to_string()))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| AppError::SvgLoad(format!("invalid dimensions {}x{}", width, height)))?;
    let sx = width as f32 / tree.size().width();
    let sy = height as f32 / tree.size().height();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(sx, sy),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap.data().to_vec())
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
    timeline: &'a mut AnimationTimeline,
}

impl<'a> DrawContext<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        draw_list: &'a mut DrawList,
        viewport: Size,
        input: &'a InputState,
        current_backend: BackendSelector,
        node_tree: &'a mut NodeTree,
        dirty: &'a mut DirtyTracker,
        focus: &'a mut FocusManager,
        images: &'a mut ImageRegistry,
        timeline: &'a mut AnimationTimeline,
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
            timeline,
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

    /// Loads, rasterizes, and draws an SVG file scaled to fill `rect`.
    ///
    /// The rasterized texture is cached by `(path, width, height)` and
    /// re-used on subsequent frames without re-rasterizing.
    pub fn draw_svg(
        &mut self,
        rect: Rect,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), AppError> {
        let w = rect.size.width.round() as u32;
        let h = rect.size.height.round() as u32;
        if w == 0 || h == 0 {
            return Ok(());
        }
        let handle = self.images.load_svg(path, w, h)?;
        self.draw_list.draw_image(rect, handle);
        Ok(())
    }

    /// Starts a one-shot animation on a node's style property.
    ///
    /// The animation begins on the current frame and runs until its duration
    /// elapses.  The animation timeline applies interpolated values to the
    /// node's style each frame before the user callback is called.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use webgpui_app::{Animation, DrawContext, Easing, NodeId};
    ///
    /// fn frame(ctx: &mut DrawContext<'_>) {
    ///     ctx.start_animation(
    ///         Animation::opacity(NodeId::ROOT, 0.0, 1.0)
    ///             .duration_ms(500.0)
    ///             .easing(Easing::EaseOut),
    ///     );
    /// }
    /// ```
    pub fn start_animation(&mut self, animation: Animation) {
        self.timeline.start(animation);
    }

    /// Updates the style of a node, automatically creating transition
    /// animations for changed properties when the node has a
    /// [`TransitionConfig`].
    ///
    /// Prefer this over `self.node_tree.set_style(id, style)` when implicit
    /// transitions should be honoured.
    pub fn set_style(&mut self, node_id: NodeId, new_style: NodeStyle) {
        if let Some(transition) = new_style.transition.clone() {
            let old_opt = self.node_tree.get(node_id).map(|n| n.style.clone());
            if let Some(old) = old_opt {
                self.timeline
                    .create_transitions(node_id, &old, &new_style, &transition);
            }
        }
        self.node_tree.set_style(node_id, new_style);
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
        let mut timeline = AnimationTimeline::new();
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

                            // Advance active animations before the user callback so
                            // that node_tree already holds the interpolated values
                            // when frame_fn runs.
                            timeline.tick(&mut node_tree, &mut dirty_tracker);

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
                                &mut timeline,
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tmp_png(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        image::RgbaImage::new(4, 4).save(&path).unwrap();
        path
    }

    #[test]
    fn image_registry_same_path_returns_cached_handle() {
        let path = write_tmp_png("webgpui_test_cache.png");
        let mut reg = ImageRegistry::new();
        let h1 = reg.load(&path).unwrap();
        let h2 = reg.load(&path).unwrap();
        assert_eq!(h1, h2);
        // Second call must not enqueue another upload.
        assert_eq!(reg.take_pending().len(), 1);
    }

    #[test]
    fn image_registry_different_paths_get_different_handles() {
        let p1 = write_tmp_png("webgpui_test_diff1.png");
        let p2 = write_tmp_png("webgpui_test_diff2.png");
        let mut reg = ImageRegistry::new();
        let h1 = reg.load(&p1).unwrap();
        let h2 = reg.load(&p2).unwrap();
        assert_ne!(h1, h2);
        assert_eq!(reg.take_pending().len(), 2);
    }

    #[test]
    fn image_registry_invalid_path_returns_error() {
        let mut reg = ImageRegistry::new();
        assert!(reg.load("nonexistent_webgpui_image.png").is_err());
    }

    #[test]
    fn image_registry_take_pending_drains_queue() {
        let path = write_tmp_png("webgpui_test_drain.png");
        let mut reg = ImageRegistry::new();
        reg.load(&path).unwrap();
        let first = reg.take_pending();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].width, 4);
        assert_eq!(first[0].height, 4);
        // After draining, pending is empty.
        assert!(reg.take_pending().is_empty());
    }

    // ---- SVG ----

    fn write_tmp_svg(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(
            &path,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">
                <rect width="16" height="16" fill="red"/>
            </svg>"#,
        )
        .unwrap();
        path
    }

    #[test]
    fn svg_rasterize_produces_correct_dimensions() {
        let path = write_tmp_svg("webgpui_test_svg_dims.svg");
        let pixels = rasterize_svg(&path, 32, 32).unwrap();
        assert_eq!(pixels.len(), (32 * 32 * 4) as usize);
    }

    #[test]
    fn svg_load_same_path_size_returns_cached_handle() {
        let path = write_tmp_svg("webgpui_test_svg_cache.svg");
        let mut reg = ImageRegistry::new();
        let h1 = reg.load_svg(&path, 16, 16).unwrap();
        let h2 = reg.load_svg(&path, 16, 16).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(reg.take_pending().len(), 1);
    }

    #[test]
    fn svg_load_different_sizes_get_different_handles() {
        let path = write_tmp_svg("webgpui_test_svg_sizes.svg");
        let mut reg = ImageRegistry::new();
        let h1 = reg.load_svg(&path, 16, 16).unwrap();
        let h2 = reg.load_svg(&path, 32, 32).unwrap();
        assert_ne!(h1, h2);
        assert_eq!(reg.take_pending().len(), 2);
    }

    #[test]
    fn svg_load_invalid_path_returns_error() {
        let mut reg = ImageRegistry::new();
        assert!(reg.load_svg("nonexistent.svg", 16, 16).is_err());
    }

    #[test]
    fn svg_load_invalid_svg_returns_error() {
        let path = std::env::temp_dir().join("webgpui_test_bad.svg");
        std::fs::write(&path, b"not valid svg content").unwrap();
        let mut reg = ImageRegistry::new();
        assert!(reg.load_svg(&path, 16, 16).is_err());
    }

    // ---- Easing ----

    #[test]
    fn easing_linear_endpoints() {
        assert_eq!(Easing::Linear.sample(0.0), 0.0);
        assert_eq!(Easing::Linear.sample(1.0), 1.0);
        assert!((Easing::Linear.sample(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn easing_ease_in_slow_start() {
        // Cubic ease-in: output at t=0.5 should be less than 0.5.
        assert!(Easing::EaseIn.sample(0.0) < 1e-6);
        assert!((Easing::EaseIn.sample(1.0) - 1.0).abs() < 1e-6);
        assert!(Easing::EaseIn.sample(0.5) < 0.5);
    }

    #[test]
    fn easing_ease_out_fast_start() {
        // Cubic ease-out: output at t=0.5 should be greater than 0.5.
        assert!(Easing::EaseOut.sample(0.0) < 1e-6);
        assert!((Easing::EaseOut.sample(1.0) - 1.0).abs() < 1e-6);
        assert!(Easing::EaseOut.sample(0.5) > 0.5);
    }

    #[test]
    fn easing_ease_in_out_symmetry() {
        let e = Easing::EaseInOut;
        assert!(e.sample(0.0) < 1e-6);
        assert!((e.sample(1.0) - 1.0).abs() < 1e-6);
        // Symmetric: sample(0.5) ≈ 0.5
        assert!((e.sample(0.5) - 0.5).abs() < 1e-5);
        // Slow start: below linear at t=0.25
        assert!(e.sample(0.25) < 0.25);
        // Slow end: above linear at t=0.75
        assert!(e.sample(0.75) > 0.75);
    }

    #[test]
    fn easing_cubic_bezier_endpoints() {
        let e = Easing::CubicBezier(0.25, 0.1, 0.25, 1.0); // CSS "ease"
        assert!(e.sample(0.0) < 1e-4);
        assert!((e.sample(1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn easing_clamps_out_of_range() {
        assert_eq!(Easing::Linear.sample(-1.0), 0.0);
        assert_eq!(Easing::Linear.sample(2.0), 1.0);
    }

    // ---- Animation opacity keyframes (5-point check per exit criteria) ----

    #[test]
    fn opacity_fade_keyframes_linear() {
        let (from, to) = (0.0_f32, 1.0_f32);
        for (t, expected) in [
            (0.0, 0.0),
            (0.25, 0.25),
            (0.5, 0.5),
            (0.75, 0.75),
            (1.0, 1.0),
        ] {
            let v = Easing::Linear.sample(t);
            let actual = from + (to - from) * v;
            assert!(
                (actual - expected).abs() < 1e-5,
                "at t={t}: expected {expected} got {actual}"
            );
        }
    }

    // ---- Animation translate keyframes ----

    #[test]
    fn translate_slide_keyframes_ease_out() {
        let (from, to) = (-30.0_f32, 0.0_f32);
        // Endpoints
        let v0 = Easing::EaseOut.sample(0.0);
        let v1 = Easing::EaseOut.sample(1.0);
        assert!((from + (to - from) * v0 - from).abs() < 1e-4);
        assert!((from + (to - from) * v1 - to).abs() < 1e-4);
        // Midpoint: ease-out reaches more than halfway by t=0.5
        let v_mid = Easing::EaseOut.sample(0.5);
        let actual_mid = from + (to - from) * v_mid;
        // Linear midpoint would be -15.0; ease-out should be closer to 0 (> -15)
        assert!(
            actual_mid > -15.0 && actual_mid <= 0.0,
            "ease-out midpoint should be > -15.0, got {actual_mid}"
        );
    }

    // ---- AnimationTimeline ----

    #[test]
    fn animation_tick_no_dirty_when_empty() {
        let mut timeline = AnimationTimeline::new();
        let mut tree = webgpui_core::NodeTree::new();
        let mut dirty = webgpui_core::DirtyTracker::new();
        assert!(!timeline.has_active());
        timeline.tick(&mut tree, &mut dirty);
        assert!(!dirty.is_dirty(), "empty timeline must not mark dirty");
    }

    #[test]
    fn animation_tick_marks_dirty_when_active() {
        let mut timeline = AnimationTimeline::new();
        let mut tree = webgpui_core::NodeTree::new();
        let mut dirty = webgpui_core::DirtyTracker::new();
        // Long duration so it stays active on the first tick.
        timeline.start(Animation::opacity(NodeId::ROOT, 0.0, 1.0).duration_ms(100_000.0));
        assert!(timeline.has_active());
        timeline.tick(&mut tree, &mut dirty);
        assert!(
            dirty.is_dirty(),
            "active animation must mark dirty every tick"
        );
    }

    #[test]
    fn animation_zero_duration_completes_immediately() {
        let mut timeline = AnimationTimeline::new();
        let mut tree = webgpui_core::NodeTree::new();
        let mut dirty = webgpui_core::DirtyTracker::new();
        timeline.start(Animation::opacity(NodeId::ROOT, 0.0, 1.0).duration_ms(0.0));
        timeline.tick(&mut tree, &mut dirty);
        // After one tick with zero duration the animation should be gone.
        assert!(!timeline.has_active());
        // The node's opacity should be the target value.
        let opacity = tree
            .get(NodeId::ROOT)
            .map(|n| n.style.opacity)
            .unwrap_or(0.0);
        assert!(
            (opacity - 1.0).abs() < 1e-5,
            "opacity should reach target, got {opacity}"
        );
    }

    #[test]
    fn animation_translate_applied_to_node() {
        let mut timeline = AnimationTimeline::new();
        let mut tree = webgpui_core::NodeTree::new();
        let mut dirty = webgpui_core::DirtyTracker::new();
        // Zero duration so tick applies the final value immediately.
        timeline.start(Animation::translate_y(NodeId::ROOT, -40.0, 0.0).duration_ms(0.0));
        timeline.tick(&mut tree, &mut dirty);
        let ty = tree
            .get(NodeId::ROOT)
            .map(|n| n.style.translate_y)
            .unwrap_or(-1.0);
        assert!(
            (ty - 0.0).abs() < 1e-5,
            "translate_y should reach 0.0, got {ty}"
        );
    }

    #[test]
    fn transition_creates_implicit_animation() {
        use webgpui_core::{NodeStyle, TransitionConfig};
        let mut timeline = AnimationTimeline::new();
        let mut tree = webgpui_core::NodeTree::new();
        // Give root node a starting style and a transition config.
        let mut old = NodeStyle::default();
        old.opacity = 1.0;
        old.transition = Some(TransitionConfig { duration_ms: 400.0 });
        tree.set_style(NodeId::ROOT, old.clone());
        // New style: different opacity.
        let mut new = old.clone();
        new.opacity = 0.0;
        timeline.create_transitions(NodeId::ROOT, &old, &new, old.transition.as_ref().unwrap());
        assert!(
            timeline.has_active(),
            "transition must create an active animation"
        );
    }
}
