//! Event handling stubs — MUST-tier (api-mapping.md §13).

use crate::types::{CompatError, CompatResult, ListenerId, NodeId};

pub fn event_on(
    _node: NodeId,
    _event_type: &str,
    _callback: Box<dyn Fn() + Send + Sync + 'static>,
) -> CompatResult<ListenerId> {
    Err(CompatError::NotSupported)
}

pub fn event_stop_propagation() -> CompatResult<()> {
    Err(CompatError::NotSupported)
}
