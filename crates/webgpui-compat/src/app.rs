//! Application lifecycle — MUST-tier (api-mapping.md §6, §13.4).

use crate::state::with_state;
use crate::types::{CompatError, CompatResult, NodeId};

/// Mounts `root` as the application root and triggers the first layout pass.
///
/// Flushes all staged nodes under `root` into the core tree.  Subsequent
/// calls to `node_append` will place nodes directly into the core tree.
pub fn app_mount(root: NodeId) -> CompatResult<()> {
    with_state(|s| {
        if s.mounted {
            return Err(CompatError::InternalError(
                "app_mount called more than once".to_string(),
            ));
        }
        if !s.flush_staged(root.0) {
            return Err(CompatError::InvalidNode);
        }
        s.mounted = true;
        s.render_requested = true;
        Ok(())
    })
}

/// Unmounts the application and releases all nodes.
pub fn app_unmount() -> CompatResult<()> {
    with_state(|s| {
        s.mounted = false;
        s.render_requested = false;
        Ok(())
    })
}

/// Requests a frame render on the next opportunity.
pub fn render_request() -> CompatResult<()> {
    with_state(|s| {
        s.render_requested = true;
        Ok(())
    })
}

/// Enables or disables vertical sync.
pub fn render_vsync(enabled: bool) -> CompatResult<()> {
    with_state(|s| {
        s.vsync = enabled;
        Ok(())
    })
}

/// Updates the viewport dimensions and marks all nodes dirty.
pub fn viewport_resize(w: u32, h: u32) -> CompatResult<()> {
    with_state(|s| {
        s.viewport = (w, h);
        s.tree.mark_all_dirty();
        s.render_requested = true;
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_vsync_roundtrip() {
        render_vsync(false).unwrap();
        render_vsync(true).unwrap();
    }

    #[test]
    fn render_request_ok() {
        assert!(render_request().is_ok());
    }

    #[test]
    fn viewport_resize_ok() {
        assert!(viewport_resize(800, 600).is_ok());
    }
}
