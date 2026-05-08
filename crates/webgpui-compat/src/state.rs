//! Process-global compat state: node tree, listener registry, focus, and
//! lifecycle flags.  Access via [`with_state`] or [`with_tree`].

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use webgpui_core::{NodeId as CoreId, NodeKind as CoreKind, NodeStyle, NodeTree};
use webgpui_layout::LayoutStyle;

use crate::types::EventType;

// ---------------------------------------------------------------------------
// StagedNode
// ---------------------------------------------------------------------------

/// A node that has been created (via `node_create`) but not yet placed into
/// the core `NodeTree`.  All style mutations before `app_mount` are cached
/// here and flushed when `flush_staged` is called.
pub(crate) struct StagedNode {
    pub kind: CoreKind,
    pub children: Vec<u64>,
    pub style: NodeStyle,
    pub layout: LayoutStyle,
}

// ---------------------------------------------------------------------------
// Listener
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) struct Listener {
    pub id: u64,
    pub event_type: EventType,
    pub callback: Box<dyn Fn() + Send + Sync + 'static>,
}

// ---------------------------------------------------------------------------
// CompatState
// ---------------------------------------------------------------------------

pub(crate) struct CompatState {
    /// Live node tree (contains only mounted nodes).
    pub tree: NodeTree,
    /// Nodes created but not yet placed in the core tree.
    pub staged: HashMap<u64, StagedNode>,
    /// Compat node ID → core `NodeId` (live, mounted nodes only).
    pub id_map: HashMap<u64, CoreId>,
    pub next_compat_id: u64,
    /// Per-node event listeners, keyed by core `NodeId`.
    pub listeners: HashMap<CoreId, Vec<Listener>>,
    pub next_listener_id: u64,
    pub focus: Option<CoreId>,
    pub render_requested: bool,
    pub vsync: bool,
    pub viewport: (u32, u32),
    pub mounted: bool,
}

impl CompatState {
    fn new() -> Self {
        Self {
            tree: NodeTree::new(),
            staged: HashMap::new(),
            id_map: HashMap::new(),
            next_compat_id: 1,
            listeners: HashMap::new(),
            next_listener_id: 1,
            focus: None,
            render_requested: false,
            vsync: true,
            viewport: (0, 0),
            mounted: false,
        }
    }

    pub fn alloc_compat_id(&mut self) -> u64 {
        let id = self.next_compat_id;
        self.next_compat_id += 1;
        id
    }

    pub fn core_id_of(&self, compat: u64) -> Option<CoreId> {
        self.id_map.get(&compat).copied()
    }

    /// Recursively flush a staged subtree into the core tree under `parent_core`.
    fn flush_recursive(
        staged: &mut HashMap<u64, StagedNode>,
        id_map: &mut HashMap<u64, CoreId>,
        tree: &mut NodeTree,
        compat_id: u64,
        parent_core: CoreId,
    ) {
        let node = match staged.remove(&compat_id) {
            Some(n) => n,
            None => return,
        };
        let core_id = match tree.add_node(parent_core, node.kind) {
            Some(id) => id,
            None => return,
        };
        tree.set_style(core_id, node.style);
        tree.set_layout(core_id, node.layout);
        id_map.insert(compat_id, core_id);
        for child_compat in node.children {
            Self::flush_recursive(staged, id_map, tree, child_compat, core_id);
        }
    }

    /// Flush the staged subtree rooted at `root_compat` into the core tree
    /// under `NodeId::ROOT`.  Returns `false` if `root_compat` is not staged.
    pub fn flush_staged(&mut self, root_compat: u64) -> bool {
        self.flush_staged_under(root_compat, CoreId::ROOT)
    }

    /// Flush the staged subtree rooted at `compat_id` into the core tree
    /// under `parent_core`.  Returns `false` if `compat_id` is not staged.
    pub fn flush_staged_under(&mut self, compat_id: u64, parent_core: CoreId) -> bool {
        if !self.staged.contains_key(&compat_id) {
            return false;
        }
        Self::flush_recursive(
            &mut self.staged,
            &mut self.id_map,
            &mut self.tree,
            compat_id,
            parent_core,
        );
        true
    }
}

// ---------------------------------------------------------------------------
// Global accessor
// ---------------------------------------------------------------------------

static GLOBAL: OnceLock<Mutex<CompatState>> = OnceLock::new();

/// Runs `f` with exclusive access to the process-global [`CompatState`].
pub(crate) fn with_state<R>(f: impl FnOnce(&mut CompatState) -> R) -> R {
    let m = GLOBAL.get_or_init(|| Mutex::new(CompatState::new()));
    f(&mut m.lock().unwrap_or_else(|p| p.into_inner()))
}

/// Runs `f` with shared read access to the process-global [`NodeTree`].
pub fn with_tree<R>(f: impl FnOnce(&NodeTree) -> R) -> R {
    let m = GLOBAL.get_or_init(|| Mutex::new(CompatState::new()));
    f(&m.lock().unwrap_or_else(|p| p.into_inner()).tree)
}

/// Replaces the global state with a fresh instance.
///
/// Only available in test builds; use the `test_lock!` macro to also
/// serialize concurrent tests before calling this.
#[cfg(test)]
pub(crate) fn reset_for_test() {
    let m = GLOBAL.get_or_init(|| Mutex::new(CompatState::new()));
    *m.lock().unwrap_or_else(|p| p.into_inner()) = CompatState::new();
}
