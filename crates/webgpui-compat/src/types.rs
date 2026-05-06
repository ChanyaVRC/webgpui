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
    Container,
    Text,
    Image,
}

/// Style property key — MUST-tier only (api-mapping.md §13.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleProp {
    Background,
    BorderWidth,
    BorderColor,
    Opacity,
    X,
    Y,
    Width,
    Height,
    MarginLeft,
    MarginTop,
    MarginRight,
    MarginBottom,
    PaddingLeft,
    PaddingTop,
    PaddingRight,
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
    Click,
    PointerMove,
    PointerDown,
    PointerUp,
    Scroll,
    KeyDown,
    KeyUp,
    Focus,
    FocusLost,
}

impl EventType {
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    pub fn prevent_default(&mut self) {
        self.default_prevented = true;
    }

    pub fn is_propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }

    pub fn is_default_prevented(&self) -> bool {
        self.default_prevented
    }
}

/// Errors returned by compat API calls (api-mapping.md §13.3).
#[derive(Debug, Error)]
pub enum CompatError {
    #[error("invalid node id")]
    InvalidNode,
    #[error("invalid listener id")]
    InvalidListener,
    #[error("style parse error: {0}")]
    StyleParseError(String),
    #[error("internal error: {0}")]
    InternalError(String),
}

pub type CompatResult<T> = Result<T, CompatError>;
