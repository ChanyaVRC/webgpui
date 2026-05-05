//! Application lifecycle stubs — MUST-tier (api-mapping.md §13).

use crate::types::{CompatError, CompatResult, NodeId};

pub fn app_mount(_root: NodeId) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

pub fn render_request() -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

pub fn render_vsync(_enabled: bool) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

pub fn viewport_resize(_w: u32, _h: u32) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}
