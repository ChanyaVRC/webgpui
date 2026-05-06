//! Event handling — MUST-tier (api-mapping.md §5, §13.4).

use crate::state::{with_state, Listener};
use crate::types::{CompatError, CompatResult, EventType, ListenerId, NodeId};

/// Registers a callback for `event_type` on `node`.
///
/// Returns a [`ListenerId`] that can be passed to `event_off` (SHOULD-tier,
/// not yet implemented) to remove the listener.
///
/// The callback signature is `Fn()` for MVP; a future milestone will add an
/// `EventContext` parameter for capture/bubble control.
pub fn event_on(
    node: NodeId,
    event_type: EventType,
    callback: Box<dyn Fn() + Send + Sync + 'static>,
) -> CompatResult<ListenerId> {
    with_state(|s| {
        let core_id = s
            .core_id_of(node.0)
            .or_else(|| {
                // Accept staged nodes too — listeners are stored by compat ID
                // until the node is mounted and a core ID is assigned.
                s.staged.contains_key(&node.0).then_some({
                    // Use a synthetic core ID derived from the compat ID.
                    // Real dispatch will use id_map once mounted.
                    webgpui_core::NodeId(node.0)
                })
            })
            .ok_or(CompatError::InvalidNode)?;

        let lid = s.next_listener_id;
        s.next_listener_id += 1;
        s.listeners.entry(core_id).or_default().push(Listener {
            id: lid,
            event_type,
            callback,
        });
        Ok(ListenerId(lid))
    })
}

/// Signals that event propagation should stop at the current node.
///
/// For MVP this is a no-op placeholder; full capture/bubble support is M1.
pub fn event_stop_propagation() -> CompatResult<()> {
    Ok(())
}

/// Moves keyboard focus to `node`.
pub fn focus_set(node: NodeId) -> CompatResult<()> {
    with_state(|s| {
        if s.id_map.contains_key(&node.0) || s.staged.contains_key(&node.0) {
            s.focus = s.core_id_of(node.0);
            Ok(())
        } else {
            Err(CompatError::InvalidNode)
        }
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_stop_propagation_is_ok() {
        assert!(event_stop_propagation().is_ok());
    }

    #[test]
    fn focus_set_invalid_node_errors() {
        assert!(matches!(
            focus_set(NodeId(u64::MAX - 1)),
            Err(CompatError::InvalidNode)
        ));
    }
}
