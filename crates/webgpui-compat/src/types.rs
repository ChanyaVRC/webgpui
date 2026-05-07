//! Shared types for the compat API: node handles, style keys, event types, and error variants.

use thiserror::Error;

/// Opaque node handle.  Invalidated after `node_remove`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

/// Opaque listener handle returned by `event_on`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerId(pub u64);

/// Node type — mirrors the legacy `"container"` / `"text"` / `"image"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// Generic layout container.
    Container,
    /// Text-rendering leaf node.
    Text,
    /// Image-rendering leaf node.
    Image,
}

/// Style property key — MUST-tier only (api-mapping.md §13.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleProp {
    /// `background` / `background-color`.
    Background,
    /// `border-width`.
    BorderWidth,
    /// `border-color`.
    BorderColor,
    /// `opacity` in `[0.0, 1.0]`.
    Opacity,
    /// `x` / `left` position.
    X,
    /// `y` / `top` position.
    Y,
    /// Element width.
    Width,
    /// Element height.
    Height,
    /// `margin-left`.
    MarginLeft,
    /// `margin-top`.
    MarginTop,
    /// `margin-right`.
    MarginRight,
    /// `margin-bottom`.
    MarginBottom,
    /// `padding-left`.
    PaddingLeft,
    /// `padding-top`.
    PaddingTop,
    /// `padding-right`.
    PaddingRight,
    /// `padding-bottom`.
    PaddingBottom,
}

impl StyleProp {
    /// Parses a CSS-like key string into a `StyleProp`.
    pub fn from_key(s: &str) -> Option<Self> {
        match s {
            "background" | "background-color" => Some(Self::Background),
            "border-width" => Some(Self::BorderWidth),
            "border-color" => Some(Self::BorderColor),
            "opacity" => Some(Self::Opacity),
            "x" | "left" => Some(Self::X),
            "y" | "top" => Some(Self::Y),
            "width" => Some(Self::Width),
            "height" => Some(Self::Height),
            "margin-left" => Some(Self::MarginLeft),
            "margin-top" => Some(Self::MarginTop),
            "margin-right" => Some(Self::MarginRight),
            "margin-bottom" => Some(Self::MarginBottom),
            "padding-left" => Some(Self::PaddingLeft),
            "padding-top" => Some(Self::PaddingTop),
            "padding-right" => Some(Self::PaddingRight),
            "padding-bottom" => Some(Self::PaddingBottom),
            _ => None,
        }
    }
}

/// Event type for `event_on` (api-mapping.md §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// Mouse / touch primary-button click.
    Click,
    /// Pointer moved over the node.
    PointerMove,
    /// Pointer button pressed.
    PointerDown,
    /// Pointer button released.
    PointerUp,
    /// Scroll-wheel or touch-scroll gesture.
    Scroll,
    /// Key pressed while the node has focus.
    KeyDown,
    /// Key released while the node has focus.
    KeyUp,
    /// Node received input focus.
    Focus,
    /// Node lost input focus.
    FocusLost,
}

impl EventType {
    /// Returns the canonical string name used in the legacy WebUI API.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::PointerMove => "pointermove",
            Self::PointerDown => "pointerdown",
            Self::PointerUp => "pointerup",
            Self::Scroll => "scroll",
            Self::KeyDown => "keydown",
            Self::KeyUp => "keyup",
            Self::Focus => "focus",
            Self::FocusLost => "focuslost",
        }
    }
}

/// Payload passed to event handlers.  Exposes `stop_propagation` and
/// `prevent_default` (api-mapping.md §13.3).
#[derive(Default)]
pub struct EventContext {
    propagation_stopped: bool,
    default_prevented: bool,
}

impl EventContext {
    /// Creates a fresh context with no flags set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Prevents the event from bubbling to ancestor nodes.
    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    /// Suppresses the default platform action for this event.
    pub fn prevent_default(&mut self) {
        self.default_prevented = true;
    }

    /// Returns `true` if [`stop_propagation`][Self::stop_propagation] was called.
    pub fn is_propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }

    /// Returns `true` if [`prevent_default`][Self::prevent_default] was called.
    pub fn is_default_prevented(&self) -> bool {
        self.default_prevented
    }
}

/// Errors returned by compat API calls (api-mapping.md §13.3).
#[derive(Debug, Error)]
pub enum CompatError {
    /// The supplied `NodeId` does not exist in the current tree.
    #[error("invalid node id")]
    InvalidNode,
    /// The supplied `ListenerId` does not match any registered listener.
    #[error("invalid listener id")]
    InvalidListener,
    /// A style value string could not be parsed (e.g. malformed hex colour).
    #[error("style parse error: {0}")]
    StyleParseError(String),
    /// An unexpected internal failure (bug in the compat layer).
    #[error("internal error: {0}")]
    InternalError(String),
}

/// Convenience alias for `Result<T, CompatError>`.
pub type CompatResult<T> = Result<T, CompatError>;
