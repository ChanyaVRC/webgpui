#![warn(missing_docs)]
//! Compatibility layer for legacy WebUI engine consumers.
//!
//! Maps legacy string-based WebUI calls to the typed `webgpui-core` API.
//! See `docs/ja/api-mapping.md §13` for the full mapping table.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use webgpui_compat::{self as compat, types::{EventType, NodeKind}};
//!
//! let root = compat::node_create(NodeKind::Container).unwrap();
//! compat::style_background(root, "#1e1e2e").unwrap();
//! compat::style_size(root, Some(800.0), Some(600.0)).unwrap();
//! compat::app_mount(root).unwrap();
//! ```

mod state;

#[cfg(test)]
mod tests;

pub mod app;
pub mod event;
pub mod node;
pub mod style;
pub mod types;

pub use app::{app_mount, app_unmount, render_request, render_vsync, viewport_resize};
pub use event::{event_on, event_stop_propagation, focus_set};
pub use node::{node_append, node_create, node_remove, node_update};
pub use state::with_tree;
pub use style::{
    style_background, style_border, style_margin, style_opacity, style_padding, style_position,
    style_set, style_set_many, style_size,
};
pub use types::{
    CompatError, CompatResult, EventContext, EventType, ListenerId, NodeId, NodeKind, StyleProp,
};
