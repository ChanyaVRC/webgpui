//! Platform abstraction layer for webgpui.
//!
//! This crate defines the traits and types that decouple the engine from any
//! specific windowing / OS library.  Concrete implementations (e.g. winit)
//! live in separate crates (`webgpui-platform-winit`).

use thiserror::Error;
use webgpui_geometry::Size;
use webgpui_input::InputEvent;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("failed to create window: {0}")]
    WindowCreation(String),
    #[error("event loop error: {0}")]
    EventLoop(String),
}

pub type PlatformResult<T> = Result<T, PlatformError>;

// ---------------------------------------------------------------------------
// WindowConfig
// ---------------------------------------------------------------------------

/// Parameters for creating a new window.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("webgpui"),
            width: 800,
            height: 600,
            resizable: true,
            vsync: true,
        }
    }
}

impl WindowConfig {
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self { title: title.into(), width, height, ..Default::default() }
    }
}

// ---------------------------------------------------------------------------
// PlatformEvent
// ---------------------------------------------------------------------------

/// Events produced by the platform event loop.
#[derive(Debug, Clone)]
pub enum PlatformEvent {
    /// The window contents should be redrawn.
    RedrawRequested,
    /// The window was resized to a new physical pixel size.
    Resized { physical_size: Size },
    /// DPI scaling factor changed.
    ScaleFactorChanged { scale_factor: f64 },
    /// The user requested the window to be closed.
    CloseRequested,
    /// An input event arrived.
    Input(InputEvent),
    /// No more events for now; the app may go idle.
    Idle,
}

// ---------------------------------------------------------------------------
// WindowHandle – opaque surface handle
// ---------------------------------------------------------------------------

/// An opaque, cloneable handle that identifies a platform window.
///
/// Back-ends downcast this to their concrete window type when needed.
pub trait WindowHandle: Send + Sync + std::fmt::Debug {
    /// Returns the current inner size in *physical* pixels.
    fn physical_size(&self) -> Size;
    /// Returns the DPI scale factor (logical pixels per physical pixel).
    fn scale_factor(&self) -> f64;
    /// Requests that the window is redrawn on the next opportunity.
    fn request_redraw(&self);
    /// Returns the title of the window.
    fn title(&self) -> &str;
}

// ---------------------------------------------------------------------------
// EventHandler – callback interface
// ---------------------------------------------------------------------------

/// The application implements this trait to receive platform events.
pub trait EventHandler {
    /// Called for every [`PlatformEvent`] dispatched by the event loop.
    fn on_event(&mut self, event: PlatformEvent, window: &dyn WindowHandle);
}

// ---------------------------------------------------------------------------
// Platform – factory + runner
// ---------------------------------------------------------------------------

/// Abstraction over a native event loop.
///
/// The implementation creates the window, then drives the event loop until
/// the user closes the window, calling `handler.on_event` for each event.
pub trait Platform {
    /// Runs the event loop, blocking until the window is closed.
    ///
    /// `config` describes the initial window.  `handler` receives all events.
    fn run(config: WindowConfig, handler: Box<dyn EventHandler>) -> PlatformResult<()>
    where
        Self: Sized;
}
