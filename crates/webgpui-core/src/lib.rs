//! UI node tree for webgpui.
//!
//! The tree is an arena-backed hierarchy of [`Node`]s.  Each node carries a
//! [`NodeStyle`] (visual properties) and a [`webgpui_layout::LayoutStyle`]
//! (layout properties).  Mutation marks nodes dirty so that the renderer can
//! skip unchanged subtrees.

use std::collections::HashSet;
use webgpui_geometry::{BorderRadius, Color, Insets, Rect, Size};
use webgpui_layout::LayoutStyle;

// ---------------------------------------------------------------------------
// NodeId
// ---------------------------------------------------------------------------

/// A stable, unique identifier for a node in the [`NodeTree`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

impl NodeId {
    pub const ROOT: Self = Self(0);
    const TOMBSTONE: Self = Self(u64::MAX);
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeId({})", self.0)
    }
}

// ---------------------------------------------------------------------------
// NodeKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeKind {
    #[default]
    Container,
    Text,
    Image,
}

// ---------------------------------------------------------------------------
// NodeStyle  (visual properties)
// ---------------------------------------------------------------------------

/// Visual appearance of a node.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeStyle {
    /// Background fill colour.  Transparent by default.
    pub background: Color,
    /// Border colour.
    pub border_color: Color,
    /// Border width on each side.
    pub border: Insets,
    /// Corner radii for rounded rectangles.
    pub border_radius: BorderRadius,
    /// Opacity in `[0.0, 1.0]`.
    pub opacity: f32,
    /// Text content (only meaningful when `kind == NodeKind::Text`).
    pub text: String,
    /// Text colour.
    pub text_color: Color,
    /// Font size in logical pixels.
    pub font_size: f32,
    /// Whether the node and its subtree are visible.
    pub visible: bool,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
            border_color: Color::TRANSPARENT,
            border: Insets::ZERO,
            border_radius: BorderRadius::ZERO,
            opacity: 1.0,
            text: String::new(),
            text_color: Color::BLACK,
            font_size: 14.0,
            visible: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub style: NodeStyle,
    pub layout: LayoutStyle,
    /// Indices of child nodes in the same [`NodeTree`] arena.
    children: Vec<usize>,
    /// Index of the parent node; `None` for the root.
    parent: Option<usize>,
    /// Whether this node's style / layout has changed since the last frame.
    dirty: bool,
}

impl Node {
    fn new(id: NodeId, kind: NodeKind) -> Self {
        Self {
            id,
            kind,
            style: NodeStyle::default(),
            layout: LayoutStyle::default(),
            children: Vec::new(),
            parent: None,
            dirty: true,
        }
    }

    fn is_tombstone(&self) -> bool {
        self.id == NodeId::TOMBSTONE
    }
}

// ---------------------------------------------------------------------------
// NodeTree
// ---------------------------------------------------------------------------

/// Arena-based UI node tree.
///
/// Nodes are stored in a flat `Vec` and referenced by arena index.
/// `NodeId` values are stable identifiers that never change.
pub struct NodeTree {
    /// Flat arena storage.
    nodes: Vec<Node>,
    /// Maps `NodeId` → arena index.
    id_to_index: std::collections::HashMap<NodeId, usize>,
    /// Next `NodeId` to hand out.
    next_id: u64,
}

impl NodeTree {
    /// Creates a new tree with a single root node.
    pub fn new() -> Self {
        let root = Node::new(NodeId::ROOT, NodeKind::Container);
        let mut id_to_index = std::collections::HashMap::new();
        id_to_index.insert(NodeId::ROOT, 0);
        Self {
            nodes: vec![root],
            id_to_index,
            next_id: 1,
        }
    }

    // ------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` only if the arena contains no nodes at all.
    ///
    /// Note: a freshly-created [`NodeTree`] always contains the root node, so
    /// this returns `false` in all normal circumstances.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        let idx = self.id_to_index.get(&id)?;
        self.nodes.get(*idx)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        let idx = *self.id_to_index.get(&id)?;
        self.nodes.get_mut(idx)
    }

    // ------------------------------------------------------------------
    // Mutation
    // ------------------------------------------------------------------

    /// Adds a new child node under `parent_id` and returns its `NodeId`.
    pub fn add_node(&mut self, parent_id: NodeId, kind: NodeKind) -> NodeId {
        let new_id = NodeId(self.next_id);
        self.next_id += 1;
        let new_index = self.nodes.len();
        let mut node = Node::new(new_id, kind);
        let parent_index = *self.id_to_index.get(&parent_id).expect("parent not found");
        node.parent = Some(parent_index);
        self.nodes.push(node);
        self.nodes[parent_index].children.push(new_index);
        self.id_to_index.insert(new_id, new_index);
        new_id
    }

    /// Removes a node and its entire subtree from the tree.
    ///
    /// The root node cannot be removed.
    pub fn remove_node(&mut self, id: NodeId) -> bool {
        if id == NodeId::ROOT {
            return false;
        }
        let Some(&idx) = self.id_to_index.get(&id) else {
            return false;
        };

        // Collect all descendant indices (DFS).
        let mut to_remove: Vec<usize> = Vec::new();
        let mut stack = vec![idx];
        while let Some(i) = stack.pop() {
            to_remove.push(i);
            for &ci in &self.nodes[i].children {
                stack.push(ci);
            }
        }

        // Detach from parent.
        if let Some(parent_idx) = self.nodes[idx].parent {
            self.nodes[parent_idx].children.retain(|&c| c != idx);
        }

        // Invalidate id_to_index entries.
        // The arena is not compacted here; call compact() explicitly when needed.
        for ri in to_remove {
            self.id_to_index.remove(&self.nodes[ri].id);
            // Mark slot as removed by resetting to a placeholder.
            self.nodes[ri].id = NodeId::TOMBSTONE;
        }
        true
    }

    /// Updates the visual style of a node and marks it dirty.
    pub fn set_style(&mut self, id: NodeId, style: NodeStyle) -> bool {
        let Some(node) = self.get_mut(id) else {
            return false;
        };
        if node.style != style {
            node.style = style;
            node.dirty = true;
        }
        true
    }

    /// Updates the layout style of a node and marks it dirty.
    pub fn set_layout(&mut self, id: NodeId, layout: LayoutStyle) -> bool {
        let Some(idx) = self.id_to_index.get(&id).copied() else {
            return false;
        };
        if self.nodes[idx].layout != layout {
            node_set_layout(&mut self.nodes[idx], layout);
        }
        true
    }

    // ------------------------------------------------------------------
    // Dirty tracking
    // ------------------------------------------------------------------

    /// Collects all nodes that are currently marked dirty and clears the
    /// dirty flag.
    pub fn flush_dirty(&mut self) -> HashSet<NodeId> {
        let mut dirty = HashSet::new();
        for node in &mut self.nodes {
            if node.dirty && !node.is_tombstone() {
                dirty.insert(node.id);
                node.dirty = false;
            }
        }
        dirty
    }

    /// Marks every node dirty (useful after a resize).
    pub fn mark_all_dirty(&mut self) {
        for node in &mut self.nodes {
            node.dirty = true;
        }
    }

    // ------------------------------------------------------------------
    // Layout integration helpers
    // ------------------------------------------------------------------

    /// Returns children of `id` in order.
    pub fn children_of(&self, id: NodeId) -> Vec<NodeId> {
        let Some(&idx) = self.id_to_index.get(&id) else {
            return vec![];
        };
        self.nodes[idx]
            .children
            .iter()
            .map(|&ci| self.nodes[ci].id)
            .collect()
    }

    /// Iterates over all valid nodes in arena order.
    pub fn iter(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter(|n| !n.is_tombstone())
    }

    /// Compacts the backing arena by removing tombstone slots.
    ///
    /// After many [`remove_node`] calls the arena grows monotonically.
    /// Call this method to reclaim memory. All live `NodeId`s remain valid;
    /// only internal arena indices change (they are not part of the public API).
    pub fn compact(&mut self) {
        let old_len = self.nodes.len();

        // Build old-index → new-index mapping (None = tombstone).
        let mut old_to_new: Vec<Option<usize>> = vec![None; old_len];
        let mut new_len = 0usize;
        for (i, node) in self.nodes.iter().enumerate() {
            if !node.is_tombstone() {
                old_to_new[i] = Some(new_len);
                new_len += 1;
            }
        }
        if new_len == old_len {
            return;
        }

        // Move live nodes into a fresh vec (no Clone required).
        let old_nodes = std::mem::take(&mut self.nodes);
        let mut new_nodes: Vec<Node> = Vec::with_capacity(new_len);
        for (i, node) in old_nodes.into_iter().enumerate() {
            if old_to_new[i].is_some() {
                new_nodes.push(node);
            }
        }

        // Remap parent / children arena indices.
        for node in &mut new_nodes {
            node.parent = node.parent.and_then(|p| old_to_new[p]);
            node.children = node
                .children
                .iter()
                .filter_map(|&c| old_to_new[c])
                .collect();
        }

        // Rebuild the id → index map.
        self.id_to_index.clear();
        for (idx, node) in new_nodes.iter().enumerate() {
            self.id_to_index.insert(node.id, idx);
        }

        self.nodes = new_nodes;
    }
}

fn node_set_layout(node: &mut Node, layout: LayoutStyle) {
    node.layout = layout;
    node.dirty = true;
}

impl Default for NodeTree {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// DirtyRect tracker
// ---------------------------------------------------------------------------

/// Accumulates dirty screen regions across frames.
///
/// The engine marks regions when nodes change; the renderer uses the union
/// of all dirty rects to decide which area to re-render.
#[derive(Debug, Default)]
pub struct DirtyTracker {
    rects: Vec<Rect>,
    /// Whether a full-screen redraw has been requested.
    full_invalidate: bool,
}

impl DirtyTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks a screen region as dirty.
    pub fn mark(&mut self, rect: Rect) {
        if !rect.is_empty() {
            self.rects.push(rect);
        }
    }

    /// Requests a full-frame redraw.
    pub fn mark_all(&mut self) {
        self.full_invalidate = true;
        self.rects.clear();
    }

    /// Returns whether a full redraw is needed.
    pub fn needs_full_redraw(&self) -> bool {
        self.full_invalidate
    }

    /// Returns true if any dirty region has been recorded.
    pub fn is_dirty(&self) -> bool {
        self.full_invalidate || !self.rects.is_empty()
    }

    /// Computes the union of all recorded dirty rects.
    ///
    /// Returns `None` in two distinct cases:
    /// * No rects have been recorded and `needs_full_redraw()` is `false`.
    /// * `needs_full_redraw()` is `true` — the caller must use the full
    ///   viewport instead (see [`effective_area`][Self::effective_area]).
    ///
    /// Prefer [`effective_area`][Self::effective_area] for the common rendering
    /// loop, which handles both cases automatically.
    pub fn dirty_union(&self) -> Option<Rect> {
        if self.full_invalidate {
            return None; // Caller should use the full viewport.
        }
        self.rects.iter().copied().reduce(|a, b| a.union(b))
    }

    /// Clears all dirty regions after they have been processed.
    pub fn clear(&mut self) {
        self.rects.clear();
        self.full_invalidate = false;
    }

    /// Checks whether any dirty rect overlaps `query`.
    pub fn overlaps(&self, query: Rect) -> bool {
        if self.full_invalidate {
            return true;
        }
        self.rects.iter().any(|r| r.intersect(query).is_some())
    }

    /// Returns the effective redraw area given the `viewport` size.
    pub fn effective_area(&self, viewport: Size) -> Rect {
        if self.full_invalidate {
            Rect::from_origin_size(webgpui_geometry::Point::ZERO, viewport)
        } else {
            self.dirty_union().unwrap_or(Rect::ZERO)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_find_node() {
        let mut tree = NodeTree::new();
        let id = tree.add_node(NodeId::ROOT, NodeKind::Container);
        assert!(tree.get(id).is_some());
    }

    #[test]
    fn remove_node() {
        let mut tree = NodeTree::new();
        let id = tree.add_node(NodeId::ROOT, NodeKind::Container);
        assert!(tree.remove_node(id));
        assert!(tree.get(id).is_none());
    }

    #[test]
    fn compact_removes_tombstones() {
        let mut tree = NodeTree::new();
        let a = tree.add_node(NodeId::ROOT, NodeKind::Container);
        let b = tree.add_node(NodeId::ROOT, NodeKind::Container);
        let c = tree.add_node(a, NodeKind::Text);
        let len_before = tree.len();
        tree.remove_node(b);
        assert_eq!(tree.len(), len_before); // arena still same size
        tree.compact();
        assert!(tree.len() < len_before); // tombstone removed
                                          // live nodes still accessible
        assert!(tree.get(a).is_some());
        assert!(tree.get(c).is_some());
        assert!(tree.get(b).is_none());
        // c is still a child of a
        assert!(tree.children_of(a).contains(&c));
    }

    #[test]
    fn dirty_tracking() {
        let mut tracker = DirtyTracker::new();
        assert!(!tracker.is_dirty());
        tracker.mark(Rect::new(0.0, 0.0, 100.0, 100.0));
        assert!(tracker.is_dirty());
        tracker.clear();
        assert!(!tracker.is_dirty());
    }
}
