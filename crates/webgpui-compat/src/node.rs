//! Node tree manipulation — MUST-tier (api-mapping.md §3, §13.4).

use webgpui_core::NodeKind as CoreKind;
use webgpui_core::NodeStyle;
use webgpui_layout::LayoutStyle;

use crate::state::{with_state, StagedNode};
use crate::types::{CompatError, CompatResult, NodeId, NodeKind};

fn to_core_kind(kind: NodeKind) -> CoreKind {
    match kind {
        NodeKind::Container => CoreKind::Container,
        NodeKind::Text => CoreKind::Text,
        NodeKind::Image => CoreKind::Image,
    }
}

/// Creates a new detached node of the given kind and returns its id.
///
/// The node lives in a staging area until it is attached via [`node_append`]
/// and its subtree root is mounted via [`crate::app_mount`].
pub fn node_create(kind: NodeKind) -> CompatResult<NodeId> {
    with_state(|s| {
        let id = s.alloc_compat_id();
        s.staged.insert(
            id,
            StagedNode {
                kind: to_core_kind(kind),
                children: Vec::new(),
                style: NodeStyle::default(),
                layout: LayoutStyle::default(),
            },
        );
        Ok(NodeId(id))
    })
}

/// Appends `child` as the last child of `parent`.
///
/// If `parent` is already mounted the child is placed into the core tree
/// immediately.  If `parent` is still staged the relationship is recorded and
/// flushed on [`crate::app_mount`].
pub fn node_append(parent: NodeId, child: NodeId) -> CompatResult<()> {
    with_state(|s| {
        let child_staged = s.staged.contains_key(&child.0);
        let child_mounted = s.id_map.contains_key(&child.0);
        if !child_staged && !child_mounted {
            return Err(CompatError::InvalidNode);
        }

        if let Some(parent_core) = s.core_id_of(parent.0) {
            if child_staged {
                if !s.flush_staged_under(child.0, parent_core) {
                    return Err(CompatError::InternalError(
                        "flush_staged_under failed".to_string(),
                    ));
                }
            }
            Ok(())
        } else if let Some(staged) = s.staged.get_mut(&parent.0) {
            if !child_staged {
                return Err(CompatError::InvalidNode);
            }
            staged.children.push(child.0);
            Ok(())
        } else {
            Err(CompatError::InvalidNode)
        }
    })
}

/// Removes `child` from the tree and invalidates its `NodeId`.
pub fn node_remove(_parent: NodeId, child: NodeId) -> CompatResult<()> {
    with_state(|s| {
        if s.staged.remove(&child.0).is_some() {
            return Ok(());
        }
        if let Some(core_id) = s.id_map.remove(&child.0) {
            if s.tree.remove_node(core_id) {
                return Ok(());
            }
        }
        Err(CompatError::InvalidNode)
    })
}

/// Marks `node` as needing re-evaluation.
///
/// For MVP the `patch` string is accepted but not applied; use the `style_*`
/// functions for style mutations.
pub fn node_update(node: NodeId, _patch: &str) -> CompatResult<()> {
    with_state(|s| {
        if s.id_map.contains_key(&node.0) || s.staged.contains_key(&node.0) {
            Ok(())
        } else {
            Err(CompatError::InvalidNode)
        }
    })
}
