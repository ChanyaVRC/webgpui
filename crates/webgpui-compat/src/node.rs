//! Node tree manipulation — MUST-tier stubs (api-mapping.md §13).

use crate::types::{CompatError, CompatResult, NodeId, NodeKind};

/// Creates a new node of the given kind and returns its id.
pub fn node_create(_kind: NodeKind) -> CompatResult<NodeId> {
    Err(CompatError::NotSupported)
}

/// Appends `child` as the last child of `parent`.
pub fn node_append(_parent: NodeId, _child: NodeId) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

/// Removes `child` from `parent` and invalidates `child`.
pub fn node_remove(_parent: NodeId, _child: NodeId) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

/// Applies a patch to `node` and marks it dirty.
pub fn node_update(_node: NodeId, _patch: &str) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}
