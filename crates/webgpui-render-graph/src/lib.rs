//! Render graph for webgpui.
//!
//! Manages named render passes, their dependencies, and topological ordering.
//! The graph drives the sequence of GPU work each frame.
//!
//! # MVP passes
//! 1. `clear`   – clears the surface to the background colour.
//! 2. `ui`      – draws all UI batches.
//! 3. `overlay` – draws debug/profiler overlays (optional).

use std::collections::HashMap;
use webgpui_batching::DrawBatch;
use webgpui_geometry::Color;

// ---------------------------------------------------------------------------
// PassId
// ---------------------------------------------------------------------------

/// A stable identifier for a render pass within the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PassId(pub u32);

impl std::fmt::Display for PassId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pass({})", self.0)
    }
}

// ---------------------------------------------------------------------------
// PassKind
// ---------------------------------------------------------------------------

/// Categorises what a render pass does; drives how the renderer handles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassKind {
    /// Clears the render target.
    Clear,
    /// Draws UI geometry.
    Ui,
    /// Draws debug overlays on top of everything else.
    Overlay,
}

// ---------------------------------------------------------------------------
// ClearColor
// ---------------------------------------------------------------------------

/// RGBA clear colour for the [`PassKind::Clear`] pass, using `f64` components in `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy)]
pub struct ClearColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl ClearColor {
    /// Opaque black `(0, 0, 0, 1)`.
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// Opaque white `(1, 1, 1, 1)`.
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    /// Fully transparent `(0, 0, 0, 0)`.
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    /// Constructs a colour from individual `f64` components in `[0.0, 1.0]`.
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }
}

impl Default for ClearColor {
    fn default() -> Self {
        Self::BLACK
    }
}

impl From<Color> for ClearColor {
    fn from(c: Color) -> Self {
        Self {
            r: c.r as f64,
            g: c.g as f64,
            b: c.b as f64,
            a: c.a as f64,
        }
    }
}

// ---------------------------------------------------------------------------
// RenderPass
// ---------------------------------------------------------------------------

/// A single render pass in the graph.
pub struct RenderPass {
    pub id: PassId,
    pub name: &'static str,
    pub kind: PassKind,
    /// Pass IDs that must complete before this pass executes.
    pub depends_on: Vec<PassId>,
    /// Whether the pass is active this frame.
    pub enabled: bool,
    /// Clear colour (only used when `kind == PassKind::Clear`).
    pub clear_color: ClearColor,
    /// Batches assigned to this pass (only meaningful for `Ui` / `Overlay`).
    pub batches: Vec<DrawBatch>,
}

impl RenderPass {
    fn new(id: PassId, name: &'static str, kind: PassKind) -> Self {
        Self {
            id,
            name,
            kind,
            depends_on: Vec::new(),
            enabled: true,
            clear_color: ClearColor::default(),
            batches: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// RenderGraph
// ---------------------------------------------------------------------------

/// Manages the set of render passes and their execution order.
///
/// Pass ordering is computed via Kahn's topological sort and cached lazily.
/// `topo_dirty` is set whenever the graph structure changes; `execution_order()`
/// recomputes `topo_order` when the cache is stale.
pub struct RenderGraph {
    passes: HashMap<PassId, RenderPass>,
    next_id: u32,
    /// Index from pass name to [`PassId`] for O(1) `pass_by_name` lookups.
    name_to_id: HashMap<&'static str, PassId>,
    /// Cached result of the last topological sort.  Rebuilt when `topo_dirty` is true.
    topo_order: Vec<PassId>,
    /// Set to `true` whenever passes are added or dependencies change, triggering
    /// a re-sort on the next `execution_order()` call.
    topo_dirty: bool,
}

impl RenderGraph {
    /// Creates a default graph with `clear`, `ui`, and `overlay` passes.
    pub fn new() -> Self {
        let mut graph = Self {
            passes: HashMap::new(),
            next_id: 0,
            name_to_id: HashMap::new(),
            topo_order: Vec::new(),
            topo_dirty: true,
        };
        // Bootstrap the three standard passes.
        let clear_id = graph.add_pass("clear", PassKind::Clear);
        let ui_id = graph.add_pass("ui", PassKind::Ui);
        let overlay_id = graph.add_pass("overlay", PassKind::Overlay);
        graph.add_dependency(ui_id, clear_id);
        graph.add_dependency(overlay_id, ui_id);
        // Disable overlay by default.
        graph.passes.get_mut(&overlay_id).unwrap().enabled = false;
        graph
    }

    /// Adds a new render pass and returns its [`PassId`].
    pub fn add_pass(&mut self, name: &'static str, kind: PassKind) -> PassId {
        let id = PassId(self.next_id);
        self.next_id += 1;
        self.passes.insert(id, RenderPass::new(id, name, kind));
        self.name_to_id.insert(name, id);
        self.topo_dirty = true;
        id
    }

    /// Declares that `pass` must run after `after`.
    ///
    /// Returns `false` if `pass` does not exist in the graph (the call is a no-op in that case).
    pub fn add_dependency(&mut self, pass: PassId, after: PassId) -> bool {
        if let Some(p) = self.passes.get_mut(&pass) {
            if !p.depends_on.contains(&after) {
                p.depends_on.push(after);
                self.topo_dirty = true;
            }
            true
        } else {
            false
        }
    }

    /// Returns a mutable reference to the pass with the given `id`, or `None` if it does not exist.
    pub fn pass_mut(&mut self, id: PassId) -> Option<&mut RenderPass> {
        self.passes.get_mut(&id)
    }

    /// Returns a shared reference to the pass with the given `id`, or `None` if it does not exist.
    pub fn pass(&self, id: PassId) -> Option<&RenderPass> {
        self.passes.get(&id)
    }

    /// Returns a pass by well-known name.
    pub fn pass_by_name(&self, name: &str) -> Option<&RenderPass> {
        self.name_to_id
            .get(name)
            .and_then(|&id| self.passes.get(&id))
    }

    /// Returns a mutable reference to a pass by well-known name.
    pub fn pass_by_name_mut(&mut self, name: &str) -> Option<&mut RenderPass> {
        let id = *self.name_to_id.get(name)?;
        self.passes.get_mut(&id)
    }

    /// Sets the background clear colour.
    pub fn set_clear_color(&mut self, color: ClearColor) {
        if let Some(clear) = self.pass_by_name_mut("clear") {
            clear.clear_color = color;
        }
    }

    /// Enables or disables the overlay pass.
    pub fn set_overlay_enabled(&mut self, enabled: bool) {
        if let Some(p) = self.pass_by_name_mut("overlay") {
            p.enabled = enabled;
        }
    }

    /// Assigns batches to the `ui` pass for this frame.
    pub fn set_ui_batches(&mut self, batches: Vec<DrawBatch>) {
        if let Some(ui) = self.pass_by_name_mut("ui") {
            ui.batches = batches;
        }
    }

    /// Returns the passes in topological execution order, skipping disabled passes.
    pub fn execution_order(&mut self) -> Vec<&RenderPass> {
        if self.topo_dirty {
            self.topo_order = self.topological_sort();
            self.topo_dirty = false;
        }
        self.topo_order
            .iter()
            .filter_map(|id| self.passes.get(id))
            .filter(|p| p.enabled)
            .collect()
    }

    // ------------------------------------------------------------------
    // Kahn's algorithm for topological sort
    // ------------------------------------------------------------------

    fn topological_sort(&self) -> Vec<PassId> {
        let ids: Vec<PassId> = self.passes.keys().copied().collect();
        let mut in_degree: HashMap<PassId, usize> = ids.iter().map(|&id| (id, 0)).collect();
        let mut adj: HashMap<PassId, Vec<PassId>> = HashMap::new();

        for (&id, pass) in &self.passes {
            for &dep in &pass.depends_on {
                *in_degree.entry(id).or_insert(0) += 1;
                adj.entry(dep).or_default().push(id);
            }
        }

        let mut queue: std::collections::VecDeque<PassId> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::new();
        while let Some(id) = queue.pop_front() {
            order.push(id);
            if let Some(dependents) = adj.get(&id) {
                for &dep in dependents {
                    let count = in_degree.get_mut(&dep).unwrap();
                    *count -= 1;
                    if *count == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }
        debug_assert_eq!(
            order.len(),
            ids.len(),
            "RenderGraph cycle detected; {} pass(es) were skipped",
            ids.len() - order.len()
        );
        order
    }
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(debug_assertions))]
    fn topological_sort_no_infinite_loop_on_cycle() {
        // Build a two-pass cycle: A depends on B, B depends on A.
        let mut graph = RenderGraph::new();
        let a = graph.add_pass("cycle_a", super::PassKind::Ui);
        let b = graph.add_pass("cycle_b", super::PassKind::Ui);
        graph.add_dependency(a, b);
        graph.add_dependency(b, a);
        // In debug builds the assert fires; in release it silently drops the cycle.
        // Either way, the sort must terminate without an infinite loop.
        let _ = graph.execution_order();
    }

    #[test]
    fn add_dependency_returns_false_for_missing_pass() {
        let mut graph = RenderGraph::new();
        let missing = super::PassId(9999);
        let ui_id = graph.pass_by_name("ui").unwrap().id;
        assert!(!graph.add_dependency(missing, ui_id));
    }

    #[test]
    fn add_dependency_no_spurious_dirty() {
        let mut graph = RenderGraph::new();
        // Populate topo cache.
        let _ = graph.execution_order();
        // Adding a dependency to a non-existent pass must not dirty the cache.
        let missing = super::PassId(9999);
        let ui_id = graph.pass_by_name("ui").unwrap().id;
        graph.add_dependency(missing, ui_id);
        assert!(!graph.topo_dirty);
    }

    #[test]
    fn pass_by_name_o1_lookup() {
        let mut graph = RenderGraph::new();
        let extra = graph.add_pass("extra", PassKind::Ui);
        assert_eq!(graph.pass_by_name("extra").unwrap().id, extra);
        assert!(graph.pass_by_name("nonexistent").is_none());
    }

    #[test]
    fn default_passes_exist() {
        let mut graph = RenderGraph::new();
        assert!(graph.pass_by_name("clear").is_some());
        assert!(graph.pass_by_name("ui").is_some());
        let order = graph.execution_order();
        // clear must come before ui.
        let clear_pos = order.iter().position(|p| p.name == "clear").unwrap();
        let ui_pos = order.iter().position(|p| p.name == "ui").unwrap();
        assert!(clear_pos < ui_pos);
    }

    #[test]
    fn clear_color_from_color() {
        use webgpui_geometry::Color;
        let c = ClearColor::from(Color::WHITE);
        assert!((c.r - 1.0).abs() < 1e-9);
        assert!((c.g - 1.0).abs() < 1e-9);
        assert!((c.b - 1.0).abs() < 1e-9);
        assert!((c.a - 1.0).abs() < 1e-9);
    }
}
