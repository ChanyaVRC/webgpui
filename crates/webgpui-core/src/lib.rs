#![warn(missing_docs)]
//! UI node tree for webgpui.
//!
//! The tree is an arena-backed hierarchy of [`Node`]s.  Each node carries a
//! [`NodeStyle`] (visual properties) and a [`webgpui_layout::LayoutStyle`]
//! (layout properties).  Mutation marks nodes dirty so that the renderer can
//! skip unchanged subtrees.

mod widget;
pub use widget::{
    Button, ContextMenu, CursorMove, Dialog, Label, ScrollView, TabBar, TextAlign, TextInput,
    Toolbar, WidgetState,
};

use webgpui_geometry::{BorderRadius, Color, Insets, Rect, Size};
use webgpui_layout::LayoutStyle;

// ---------------------------------------------------------------------------
// Focus ring
// ---------------------------------------------------------------------------

/// Standard focus ring stroke width in logical pixels.
pub const FOCUS_RING_WIDTH: f32 = 2.0;

/// Standard focus ring colour (blue accent, consistent across all widgets).
pub const FOCUS_RING_COLOR: Color = Color {
    r: 0.35,
    g: 0.7,
    b: 1.0,
    a: 1.0,
};

/// Standard focus ring colour. Prefer the [`FOCUS_RING_COLOR`] constant for
/// const contexts; this function exists for backwards compatibility.
pub fn focus_ring_color() -> Color {
    FOCUS_RING_COLOR
}

// ---------------------------------------------------------------------------
// NodeRole
// ---------------------------------------------------------------------------

/// Accessibility role for a [`Node`].
///
/// Assigned when widgets are wired into the node tree.  Consumers (screen
/// readers, keyboard managers) use this to understand widget semantics without
/// inspecting visual properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeRole {
    /// No specific role.
    #[default]
    None,
    /// Interactive button.
    Button,
    /// Single-line or multi-line text input.
    TextBox,
    /// A tab within a tab bar.
    Tab,
    /// Modal dialog container.
    Dialog,
    /// Dropdown or context menu container.
    Menu,
}

// ---------------------------------------------------------------------------
// NodeId
// ---------------------------------------------------------------------------

/// A stable, unique identifier for a node in the [`NodeTree`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

impl NodeId {
    /// The root node, always present in every [`NodeTree`].
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

/// The structural type of a node — drives layout and rendering treatment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeKind {
    /// A generic layout container (default).
    #[default]
    Container,
    /// A leaf node that renders a text string.
    Text,
    /// A leaf node that renders a raster image.
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

/// A single node in the [`NodeTree`] arena.
pub struct Node {
    /// Stable unique identifier.
    pub id: NodeId,
    /// Structural type (container, text, image).
    pub kind: NodeKind,
    /// Visual style properties.
    pub style: NodeStyle,
    /// Layout constraints and positioning.
    pub layout: LayoutStyle,
    /// Accessibility role.
    pub role: NodeRole,
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
            role: NodeRole::None,
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
    /// Number of live (non-tombstone) nodes; maintained incrementally.
    live_count: usize,
    /// Arena indices of nodes currently marked dirty. Kept in sync with `node.dirty`.
    dirty_indices: Vec<usize>,
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
            live_count: 1,
            dirty_indices: vec![0], // root node starts dirty at index 0
        }
    }

    // ------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------

    /// Returns the number of live (non-removed) nodes in the tree.
    pub fn len(&self) -> usize {
        self.live_count
    }

    /// Returns `true` only if the arena contains no nodes at all.
    ///
    /// Note: a freshly-created [`NodeTree`] always contains the root node, so
    /// this returns `false` in all normal circumstances.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns a shared reference to the node with `id`, or `None` if not found.
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        let idx = self.id_to_index.get(&id)?;
        self.nodes.get(*idx)
    }

    /// Returns an exclusive reference to the node with `id`, or `None` if not found.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        let idx = *self.id_to_index.get(&id)?;
        self.nodes.get_mut(idx)
    }

    // ------------------------------------------------------------------
    // Mutation
    // ------------------------------------------------------------------

    /// Adds a new child node under `parent_id` and returns its `NodeId`.
    ///
    /// Returns `None` if `parent_id` does not exist in the tree.
    pub fn add_node(&mut self, parent_id: NodeId, kind: NodeKind) -> Option<NodeId> {
        let parent_index = *self.id_to_index.get(&parent_id)?;
        let new_id = NodeId(self.next_id);
        self.next_id += 1;
        let new_index = self.nodes.len();
        let mut node = Node::new(new_id, kind);
        node.parent = Some(parent_index);
        self.nodes.push(node);
        self.nodes[parent_index].children.push(new_index);
        self.id_to_index.insert(new_id, new_index);
        self.live_count += 1;
        self.dirty_indices.push(new_index);
        Some(new_id)
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
        let removed_count = to_remove.len();
        for ri in to_remove {
            self.id_to_index.remove(&self.nodes[ri].id);
            // Mark slot as removed by resetting to a placeholder.
            self.nodes[ri].id = NodeId::TOMBSTONE;
        }
        self.live_count -= removed_count;
        true
    }

    fn mark_dirty_at(&mut self, idx: usize) {
        if !self.nodes[idx].dirty {
            self.nodes[idx].dirty = true;
            self.dirty_indices.push(idx);
        }
    }

    /// Updates the visual style of a node and marks it dirty.
    pub fn set_style(&mut self, id: NodeId, style: NodeStyle) -> bool {
        let Some(&idx) = self.id_to_index.get(&id) else {
            return false;
        };
        if self.nodes[idx].style != style {
            self.nodes[idx].style = style;
            self.mark_dirty_at(idx);
        }
        true
    }

    /// Sets the accessibility role of a node.
    pub fn set_role(&mut self, id: NodeId, role: NodeRole) -> bool {
        let Some(&idx) = self.id_to_index.get(&id) else {
            return false;
        };
        if self.nodes[idx].role != role {
            self.nodes[idx].role = role;
            self.mark_dirty_at(idx);
        }
        true
    }

    /// Updates the layout style of a node and marks it dirty.
    pub fn set_layout(&mut self, id: NodeId, layout: LayoutStyle) -> bool {
        let Some(idx) = self.id_to_index.get(&id).copied() else {
            return false;
        };
        if self.nodes[idx].layout != layout {
            self.nodes[idx].layout = layout;
            self.mark_dirty_at(idx);
        }
        true
    }

    // ------------------------------------------------------------------
    // Dirty tracking
    // ------------------------------------------------------------------

    /// Collects all nodes that are currently marked dirty and clears the
    /// dirty flag.
    pub fn flush_dirty(&mut self) -> Vec<NodeId> {
        self.dirty_indices
            .drain(..)
            .filter_map(|i| {
                let node = &mut self.nodes[i];
                if node.dirty && !node.is_tombstone() {
                    node.dirty = false;
                    Some(node.id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Marks every node dirty (useful after a resize).
    pub fn mark_all_dirty(&mut self) {
        self.dirty_indices.clear();
        for (i, node) in self.nodes.iter_mut().enumerate() {
            if !node.is_tombstone() {
                node.dirty = true;
                self.dirty_indices.push(i);
            }
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

    /// Iterates over the children of `id` without allocating.
    ///
    /// Callers that need an owned `Vec<NodeId>` can call `.collect()` on the
    /// returned iterator. Prefer this over [`children_of`][Self::children_of]
    /// when only iteration is needed.
    pub fn children_iter(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let idx = self.id_to_index.get(&id).copied();
        idx.into_iter().flat_map(move |i| {
            self.nodes[i]
                .children
                .iter()
                .map(move |&ci| self.nodes[ci].id)
        })
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
        let mut old_to_new: Vec<Option<usize>> = vec![None; old_len];

        // Pass 1: build old→new index map and move live nodes simultaneously.
        let old_nodes = std::mem::take(&mut self.nodes);
        let mut new_nodes: Vec<Node> = Vec::new();
        for (i, node) in old_nodes.into_iter().enumerate() {
            if !node.is_tombstone() {
                old_to_new[i] = Some(new_nodes.len());
                new_nodes.push(node);
            }
        }
        if new_nodes.len() == old_len {
            self.nodes = new_nodes;
            return;
        }

        // Pass 2: remap arena indices and rebuild id→index map in one sweep.
        self.id_to_index.clear();
        for (new_idx, node) in new_nodes.iter_mut().enumerate() {
            node.parent = node.parent.and_then(|p| old_to_new[p]);
            node.children = node
                .children
                .iter()
                .filter_map(|&c| old_to_new[c])
                .collect();
            self.id_to_index.insert(node.id, new_idx);
        }

        self.live_count = new_nodes.len();
        self.nodes = new_nodes;
        self.dirty_indices = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| if n.dirty { Some(i) } else { None })
            .collect();
    }
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
    /// Creates an empty tracker with no dirty regions.
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

    /// Returns the effective redraw area given the `viewport` size, or `None`
    /// if nothing is dirty.
    ///
    /// Returns `Some(full_viewport)` when a full-screen redraw was requested,
    /// `Some(union)` when individual rects were marked, and `None` when the
    /// tracker is clean.
    pub fn effective_area(&self, viewport: Size) -> Option<Rect> {
        if self.full_invalidate {
            Some(Rect::from_origin_size(
                webgpui_geometry::Point::ZERO,
                viewport,
            ))
        } else {
            self.dirty_union()
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
        let id = tree.add_node(NodeId::ROOT, NodeKind::Container).unwrap();
        assert!(tree.get(id).is_some());
    }

    #[test]
    fn remove_node() {
        let mut tree = NodeTree::new();
        let id = tree.add_node(NodeId::ROOT, NodeKind::Container).unwrap();
        assert!(tree.remove_node(id));
        assert!(tree.get(id).is_none());
    }

    #[test]
    fn compact_removes_tombstones() {
        let mut tree = NodeTree::new();
        let a = tree.add_node(NodeId::ROOT, NodeKind::Container).unwrap();
        let b = tree.add_node(NodeId::ROOT, NodeKind::Container).unwrap();
        let c = tree.add_node(a, NodeKind::Text).unwrap();
        // Four live nodes: root + a + b + c
        assert_eq!(tree.len(), 4);
        tree.remove_node(b);
        // b is gone; three live nodes remain
        assert_eq!(tree.len(), 3);
        tree.compact();
        // Still three live nodes after compaction
        assert_eq!(tree.len(), 3);
        assert!(tree.get(a).is_some());
        assert!(tree.get(c).is_some());
        assert!(tree.get(b).is_none());
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

    // ---- Before fix: renderer skips first frame after backend switch (#112) ----

    #[test]
    fn before_fix_renderer_skips_redraw_after_backend_switch() {
        // Before the fix, mark_all() was not called after a backend switch.
        // This test drives DirtyTracker the same way the app event loop does:
        // 1. Frame N renders normally: mark the full viewport, then clear.
        // 2. Backend switch happens (old code: no mark_all).
        // 3. Renderer asks "do I need to redraw?" via overlaps(viewport).
        // Result: overlaps() returns false — renderer silently skips frame N+1.
        let mut tracker = DirtyTracker::new();
        let viewport = Rect::new(0.0, 0.0, 1280.0, 720.0);

        // Frame N: normal render cycle.
        tracker.mark(viewport);
        tracker.clear(); // renderer consumed the dirty state

        // Backend switch — old code did NOT call mark_all() here.
        // Renderer now queries whether anything needs drawing.
        assert!(
            !tracker.overlaps(viewport),
            "bug: renderer sees no dirty area and skips the first frame on the new backend"
        );
    }

    // ---- After fix: renderer performs full redraw after backend switch ----

    #[test]
    fn after_fix_renderer_redraws_full_viewport_after_backend_switch() {
        // After the fix, mark_all() is called immediately after the backend switch.
        // Drive through the same public API the renderer uses.
        let mut tracker = DirtyTracker::new();
        let viewport = Rect::new(0.0, 0.0, 1280.0, 720.0);

        // Frame N: normal render cycle.
        tracker.mark(viewport);
        tracker.clear();

        // Backend switch — new code calls mark_all().
        tracker.mark_all();

        // Renderer queries: does anything need drawing?
        assert!(
            tracker.overlaps(viewport),
            "renderer must see the full viewport as dirty after backend switch"
        );
        assert_eq!(
            tracker.effective_area(Size::new(1280.0, 720.0)),
            Some(viewport),
            "effective_area must cover the entire viewport"
        );
    }

    // ---- NodeRole --------------------------------------------------------

    #[test]
    fn node_default_role_is_none() {
        let tree = NodeTree::new();
        assert_eq!(tree.get(NodeId::ROOT).unwrap().role, NodeRole::None);
    }

    #[test]
    fn set_role_readable_via_get() {
        let mut tree = NodeTree::new();
        let id = tree.add_node(NodeId::ROOT, NodeKind::Container).unwrap();
        assert!(tree.set_role(id, NodeRole::Button));
        assert_eq!(tree.get(id).unwrap().role, NodeRole::Button);
    }

    #[test]
    fn widget_roles_match_expected() {
        assert_eq!(Button::role(), NodeRole::Button);
        assert_eq!(TextInput::role(), NodeRole::TextBox);
        assert_eq!(TabBar::role(), NodeRole::Tab);
        assert_eq!(Dialog::role(), NodeRole::Dialog);
        assert_eq!(ContextMenu::role(), NodeRole::Menu);
        assert_eq!(Label::role(), NodeRole::None);
    }

    // ---- len() counts live nodes only (#74) ---
    #[test]
    fn len_excludes_tombstones() {
        let mut tree = NodeTree::new();
        assert_eq!(tree.len(), 1); // root only
        let a = tree.add_node(NodeId::ROOT, NodeKind::Container).unwrap();
        assert_eq!(tree.len(), 2);
        tree.remove_node(a);
        assert_eq!(tree.len(), 1); // tombstone not counted
    }

    // ---- mark_all_dirty skips tombstones (#75) ---
    #[test]
    fn mark_all_dirty_skips_tombstones() {
        let mut tree = NodeTree::new();
        let a = tree.add_node(NodeId::ROOT, NodeKind::Container).unwrap();
        tree.remove_node(a);
        // Should not panic or include tombstone in dirty set.
        tree.mark_all_dirty();
        let dirty = tree.flush_dirty();
        assert!(!dirty.contains(&a)); // tombstone must not appear
        assert!(dirty.contains(&NodeId::ROOT));
    }

    // ---- set_role marks dirty (#77) ---
    #[test]
    fn set_role_marks_dirty() {
        let mut tree = NodeTree::new();
        // Flush initial dirty state.
        tree.flush_dirty();
        // Setting a new role must mark the node dirty.
        assert!(tree.set_role(NodeId::ROOT, NodeRole::Button));
        let dirty = tree.flush_dirty();
        assert!(dirty.contains(&NodeId::ROOT));
    }

    #[test]
    fn set_role_no_dirty_on_same_value() {
        let mut tree = NodeTree::new();
        tree.flush_dirty();
        // Setting the same role (None → None) must not mark dirty.
        assert!(tree.set_role(NodeId::ROOT, NodeRole::None));
        let dirty = tree.flush_dirty();
        assert!(!dirty.contains(&NodeId::ROOT));
    }

    #[test]
    fn add_node_returns_none_for_missing_parent() {
        let mut tree = NodeTree::new();
        let fake_id = NodeId(9999);
        let result = tree.add_node(fake_id, NodeKind::Container);
        assert!(result.is_none());
        // next_id must not have been consumed
        let real = tree.add_node(NodeId::ROOT, NodeKind::Container).unwrap();
        assert_eq!(real.0, 1); // first real allocation still gets id=1
    }

    // ---- Focus ring ------------------------------------------------------

    #[test]
    fn focus_ring_width_is_two() {
        assert_eq!(FOCUS_RING_WIDTH, 2.0);
    }

    #[test]
    fn focus_ring_color_const_matches_fn() {
        assert_eq!(FOCUS_RING_COLOR.r, focus_ring_color().r);
        assert_eq!(FOCUS_RING_COLOR.a, 1.0);
    }
}
