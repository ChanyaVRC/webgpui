//! Compatibility layer for legacy WebUI engine consumers.
//!
//! Provides the MUST-tier API surface mapping legacy string-based WebUI
//! calls to the typed `webgpui-core` API.  See `docs/api-mapping.md §13`
//! for the full compatibility table.
//!
//! All functions currently return [`CompatError::NotSupported`]; they are
//! stubs to be filled in during M4.

pub mod app;
pub mod event;
pub mod node;
pub mod style;
pub mod types;

pub use app::{app_mount, render_request, render_vsync, viewport_resize};
pub use event::{event_on, event_stop_propagation};
pub use node::{node_append, node_create, node_remove, node_update};
pub use style::{
    style_background, style_border, style_margin, style_opacity, style_padding, style_position,
    style_set, style_set_many, style_size,
};
pub use types::{CompatError, CompatResult, ListenerId, NodeId, NodeKind};
