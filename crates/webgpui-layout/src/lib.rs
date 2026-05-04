//! Layout engine for webgpui.
//!
//! Supports:
//! - **Absolute** – positioned at explicit `(x, y)` relative to the parent
//!   content area.
//! - **Stack** – children laid out along the `direction` axis.
//!   - `Direction::Column` (default): top-to-bottom.
//!   - `Direction::Row`: left-to-right.
//! - **`flex_grow`** – proportionally distributes remaining main-axis space
//!   after fixed-size children are placed.
//! - **Text nodes** – auto-sized via the pluggable [`TextMeasure`] trait.
//!
//! Margin and padding are fully respected.  `width` / `height` may be an
//! explicit pixel value or `None` (fill / shrink-wrap).

use webgpui_geometry::{Insets, Point, Rect, Size};

// ---------------------------------------------------------------------------
// Direction
// ---------------------------------------------------------------------------

/// Main axis for stack layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Children stacked top-to-bottom (default).
    #[default]
    Column,
    /// Children stacked left-to-right.
    Row,
}

// ---------------------------------------------------------------------------
// PositionType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PositionType {
    /// Positioned at `(x, y)` relative to the parent content area.
    Absolute,
    /// Participates in the parent's stack flow.
    #[default]
    Stack,
}

// ---------------------------------------------------------------------------
// LayoutStyle
// ---------------------------------------------------------------------------

/// Per-node layout properties.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutStyle {
    pub position: PositionType,
    /// Stack direction for children.
    pub direction: Direction,
    /// Explicit x-offset for `Absolute` nodes.
    pub x: f32,
    /// Explicit y-offset for `Absolute` nodes.
    pub y: f32,
    /// Explicit width. `None` means "fill available width" (Column) or
    /// "shrink-wrap / grow" (Row).
    pub width: Option<f32>,
    /// Explicit height. `None` means "fill available height" (Row) or
    /// "shrink-wrap children" (Column).
    pub height: Option<f32>,
    pub margin: Insets,
    pub padding: Insets,
    /// Gap between consecutive stack children.
    pub gap: f32,
    /// Proportion of remaining main-axis space to absorb (like CSS flex-grow).
    /// `0.0` means the node does not grow.
    pub flex_grow: f32,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            position: PositionType::Stack,
            direction: Direction::Column,
            x: 0.0,
            y: 0.0,
            width: None,
            height: None,
            margin: Insets::ZERO,
            padding: Insets::ZERO,
            gap: 0.0,
            flex_grow: 0.0,
        }
    }
}

impl LayoutStyle {
    pub fn absolute(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            position: PositionType::Absolute,
            x,
            y,
            width: Some(width),
            height: Some(height),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// TextMeasure
// ---------------------------------------------------------------------------

/// Measures a text string given a font size and a maximum available width.
///
/// Implementations are object-safe so they can be passed as `&dyn TextMeasure`.
pub trait TextMeasure {
    /// Returns the `(width, height)` bounding box of `text` rendered at
    /// `font_size` and wrapped to `max_width` (pass `f32::INFINITY` to disable
    /// wrapping).
    fn measure(&self, text: &str, font_size: f32, max_width: f32) -> Size;
}

// ---------------------------------------------------------------------------
// DefaultTextMeasure
// ---------------------------------------------------------------------------

/// Pixel-font implementation (`FONT_W = 5`, `FONT_H = 7`, 1 px advance gap).
///
/// Scale is derived from `font_size / 14.0` (the default `NodeStyle::font_size`).
pub struct DefaultTextMeasure;

const FONT_W: f32 = 5.0;
const FONT_H: f32 = 7.0;
const BASE_FONT_SIZE: f32 = 14.0;

impl TextMeasure for DefaultTextMeasure {
    fn measure(&self, text: &str, font_size: f32, max_width: f32) -> Size {
        let chars: Vec<char> = text.chars().collect();
        let total = chars.len();
        if total == 0 {
            return Size::new(0.0, 0.0);
        }
        let scale = font_size / BASE_FONT_SIZE;
        let char_advance = (FONT_W + 1.0) * scale; // advance including gap
        let line_h = FONT_H * scale;

        let mut lines: u32 = 0;
        let mut max_w: f32 = 0.0;
        let mut line_start = 0;

        while line_start < total {
            // Extend line until it would exceed max_width.
            let mut end = line_start + 1; // always include at least one char
            while end < total {
                // Width of chars [line_start, end+1): n chars → n*advance - scale (no trailing gap)
                let w = char_advance * (end - line_start + 1) as f32 - scale;
                if max_width.is_finite() && w > max_width {
                    break;
                }
                end += 1;
            }
            let line_w = char_advance * (end - line_start) as f32 - scale;
            max_w = max_w.max(line_w);
            lines += 1;
            line_start = end;
        }

        Size::new(max_w, line_h * lines as f32)
    }
}

// ---------------------------------------------------------------------------
// LayoutNode  (input)
// ---------------------------------------------------------------------------

/// A node fed into the layout engine.
#[derive(Debug)]
pub struct LayoutNode {
    pub id: u32,
    pub style: LayoutStyle,
    /// Indices into the same flat array of layout nodes (children).
    pub children: Vec<usize>,
    /// Text content.  Non-empty signals that this node should be auto-sized
    /// via the [`TextMeasure`] provided to [`LayoutEngine::compute_with`].
    pub text: String,
    /// Font size in logical pixels used when measuring `text`.
    pub font_size: f32,
}

impl LayoutNode {
    pub fn new(id: u32, style: LayoutStyle) -> Self {
        Self {
            id,
            style,
            children: Vec::new(),
            text: String::new(),
            font_size: 14.0,
        }
    }
}

// ---------------------------------------------------------------------------
// LayoutResult  (output)
// ---------------------------------------------------------------------------

/// Computed layout for a single node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutResult {
    /// Outer border box.
    pub border_box: Rect,
    /// Inner content area (border_box minus padding).
    pub content_box: Rect,
}

impl LayoutResult {
    fn from_border(border: Rect, padding: Insets) -> Self {
        Self {
            border_box: border,
            content_box: border.shrink(padding),
        }
    }

    fn zero() -> Self {
        Self {
            border_box: Rect::ZERO,
            content_box: Rect::ZERO,
        }
    }
}

// ---------------------------------------------------------------------------
// LayoutEngine
// ---------------------------------------------------------------------------

/// Computes layout for a flat array of [`LayoutNode`]s.
///
/// The root node is at index `0`.  After calling [`LayoutEngine::compute`]
/// or [`LayoutEngine::compute_with`] the results are available via
/// [`LayoutEngine::result`].
pub struct LayoutEngine {
    results: Vec<LayoutResult>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Computes layout using the [`DefaultTextMeasure`].
    pub fn compute(&mut self, nodes: &[LayoutNode], viewport: Size) {
        self.compute_with(nodes, viewport, &DefaultTextMeasure);
    }

    /// Computes layout using a custom `text_measure` implementation.
    pub fn compute_with(
        &mut self,
        nodes: &[LayoutNode],
        viewport: Size,
        text_measure: &dyn TextMeasure,
    ) {
        self.results = vec![LayoutResult::zero(); nodes.len()];
        if nodes.is_empty() {
            return;
        }
        let available = Rect::from_origin_size(Point::ZERO, viewport);
        self.layout_node(nodes, 0, available, text_measure);
    }

    /// Returns the computed result for the node at arena **index**, or `None`.
    pub fn result(&self, index: usize) -> Option<LayoutResult> {
        self.results.get(index).copied()
    }

    // ------------------------------------------------------------------
    // Internal layout
    // ------------------------------------------------------------------

    fn layout_node(
        &mut self,
        nodes: &[LayoutNode],
        idx: usize,
        parent_content: Rect,
        tm: &dyn TextMeasure,
    ) {
        let node = &nodes[idx];
        let style = &node.style;
        let margin = style.margin;
        let padding = style.padding;

        // --- Absolute positioning ---
        if style.position == PositionType::Absolute {
            let w = style.width.unwrap_or(parent_content.size.width);
            let h = style.height.unwrap_or(0.0);
            let border = Rect::new(
                parent_content.origin.x + style.x,
                parent_content.origin.y + style.y,
                w,
                h,
            );
            self.results[idx] = LayoutResult::from_border(border, padding);
            let content = self.results[idx].content_box;
            for &ci in &nodes[idx].children {
                self.layout_node(nodes, ci, content, tm);
            }
            return;
        }

        // --- Stack (Column / Row) ---
        let dir = style.direction;

        // Resolve initial border box dimensions.
        // A flex_grow child gets a definite main-axis size from its parent's
        // allocation (passed as parent_content); non-grow nodes shrink-wrap.
        let (init_w, init_h) = match dir {
            Direction::Column => {
                let w = style
                    .width
                    .unwrap_or(parent_content.size.width - margin.horizontal());
                let h = style.height.unwrap_or_else(|| {
                    if style.flex_grow > 0.0 {
                        (parent_content.size.height - margin.vertical()).max(0.0)
                    } else {
                        0.0 // finalised after children (shrink-wrap)
                    }
                });
                (w, h)
            }
            Direction::Row => {
                let w = style.width.unwrap_or_else(|| {
                    if style.flex_grow > 0.0 {
                        (parent_content.size.width - margin.horizontal()).max(0.0)
                    } else {
                        0.0 // finalised after children (shrink-wrap)
                    }
                });
                let h = style
                    .height
                    .unwrap_or(parent_content.size.height - margin.vertical());
                (w, h)
            }
        };

        let border = Rect::new(
            parent_content.origin.x + margin.left,
            parent_content.origin.y + margin.top,
            init_w,
            init_h,
        );
        self.results[idx] = LayoutResult::from_border(border, padding);

        // --- Text auto-size (leaf node with content) ---
        if !node.text.is_empty() && node.children.is_empty() {
            let avail_w = self.results[idx].content_box.size.width;
            let measured = tm.measure(&node.text, node.font_size, avail_w);
            // Override dimensions not explicitly set.
            if style.width.is_none() {
                self.results[idx].border_box.size.width = measured.width + padding.horizontal();
                self.results[idx].content_box = self.results[idx].border_box.shrink(padding);
            }
            if style.height.is_none() {
                self.results[idx].border_box.size.height = measured.height + padding.vertical();
                self.results[idx].content_box = self.results[idx].border_box.shrink(padding);
            }
            return;
        }

        if node.children.is_empty() {
            return;
        }

        let content = self.results[idx].content_box;

        // --- Phase 1: layout fixed children, collect grow children ---
        let mut main_used: f32 = 0.0; // space consumed by fixed + margins
        let mut total_grow: f32 = 0.0;
        let gap = style.gap;
        let child_count = node.children.len();

        for (child_ord, &ci) in node.children.iter().enumerate() {
            let child_style = &nodes[ci].style;
            if child_style.position == PositionType::Absolute {
                self.layout_node(nodes, ci, content, tm);
                continue;
            }
            if child_style.flex_grow > 0.0 {
                total_grow += child_style.flex_grow;
                // Reserve margin space.
                match dir {
                    Direction::Column => {
                        main_used += child_style.margin.vertical();
                    }
                    Direction::Row => {
                        main_used += child_style.margin.horizontal();
                    }
                }
                if child_ord + 1 < child_count {
                    main_used += gap;
                }
                continue;
            }

            // Fixed child: layout with current cursor.
            let child_available = self.child_available(content, dir, main_used);
            self.layout_node(nodes, ci, child_available, tm);
            let cr = self.results[ci];
            match dir {
                Direction::Column => {
                    main_used += cr.border_box.size.height
                        + child_style.margin.bottom
                        + if child_ord + 1 < child_count {
                            gap
                        } else {
                            0.0
                        };
                }
                Direction::Row => {
                    main_used += cr.border_box.size.width
                        + child_style.margin.right
                        + if child_ord + 1 < child_count {
                            gap
                        } else {
                            0.0
                        };
                }
            }
        }

        // --- Phase 2: distribute remaining space to grow children ---
        let main_content_size = match dir {
            Direction::Column => content.size.height,
            Direction::Row => content.size.width,
        };
        // For auto-sized parents main_content_size may be 0; use parent's.
        let main_avail = if main_content_size > 0.0 {
            main_content_size
        } else {
            match dir {
                Direction::Column => parent_content.size.height,
                Direction::Row => parent_content.size.width,
            }
        };
        let remaining = (main_avail - main_used).max(0.0);

        // --- Phase 3: second pass – place all children in order ---
        let mut cursor: f32 = match dir {
            Direction::Column => content.origin.y,
            Direction::Row => content.origin.x,
        };

        for (child_ord, &ci) in node.children.iter().enumerate() {
            let child_style = &nodes[ci].style;
            if child_style.position == PositionType::Absolute {
                continue; // already placed
            }

            let child_main_size = if child_style.flex_grow > 0.0 && total_grow > 0.0 {
                remaining * (child_style.flex_grow / total_grow)
                    - match dir {
                        Direction::Column => child_style.margin.vertical(),
                        Direction::Row => child_style.margin.horizontal(),
                    }
            } else {
                // Already laid out; just re-position.
                match dir {
                    Direction::Column => self.results[ci].border_box.size.height,
                    Direction::Row => self.results[ci].border_box.size.width,
                }
            };

            // Build available rect for this child at current cursor.
            let child_avail = match dir {
                Direction::Column => {
                    let cm = child_style.margin;
                    Rect::from_origin_size(
                        Point::new(content.origin.x, cursor + cm.top),
                        Size::new(content.size.width, child_main_size.max(0.0)),
                    )
                }
                Direction::Row => {
                    let cm = child_style.margin;
                    Rect::from_origin_size(
                        Point::new(cursor + cm.left, content.origin.y),
                        Size::new(child_main_size.max(0.0), content.size.height),
                    )
                }
            };

            if child_style.flex_grow > 0.0 {
                // Grow child: layout now with its allocated size.
                self.layout_node(nodes, ci, child_avail, tm);
            } else {
                // Fixed child: re-position to current cursor without re-layout.
                self.reposition(ci, child_avail, dir);
            }

            let cr = self.results[ci];
            match dir {
                Direction::Column => {
                    let cm = child_style.margin;
                    cursor = cr.border_box.max_y()
                        + cm.bottom
                        + if child_ord + 1 < child_count {
                            gap
                        } else {
                            0.0
                        };
                }
                Direction::Row => {
                    let cm = child_style.margin;
                    cursor = cr.border_box.max_x()
                        + cm.right
                        + if child_ord + 1 < child_count {
                            gap
                        } else {
                            0.0
                        };
                }
            }
        }

        // --- Finalise parent size if auto (shrink-wrap, non-grow only) ---
        match dir {
            Direction::Column => {
                if style.height.is_none() && style.flex_grow == 0.0 {
                    let children_h = cursor - content.origin.y;
                    self.results[idx].border_box.size.height =
                        (children_h + padding.vertical()).max(0.0);
                    self.results[idx].content_box = self.results[idx].border_box.shrink(padding);
                }
            }
            Direction::Row => {
                if style.width.is_none() && style.flex_grow == 0.0 {
                    let children_w = cursor - content.origin.x;
                    self.results[idx].border_box.size.width =
                        (children_w + padding.horizontal()).max(0.0);
                    self.results[idx].content_box = self.results[idx].border_box.shrink(padding);
                }
            }
        }
    }

    /// Returns the available `Rect` for the next child at `cursor` offset
    /// along the main axis.
    fn child_available(&self, content: Rect, dir: Direction, cursor_offset: f32) -> Rect {
        match dir {
            Direction::Column => Rect::from_origin_size(
                Point::new(content.origin.x, content.origin.y + cursor_offset),
                content.size,
            ),
            Direction::Row => Rect::from_origin_size(
                Point::new(content.origin.x + cursor_offset, content.origin.y),
                content.size,
            ),
        }
    }

    /// Moves a previously-laid-out child to start at `avail` without
    /// recomputing its size.
    fn reposition(&mut self, ci: usize, avail: Rect, dir: Direction) {
        let r = &mut self.results[ci];
        match dir {
            Direction::Column => {
                let dy = avail.origin.y - r.border_box.origin.y;
                r.border_box.origin.y += dy;
                r.content_box.origin.y += dy;
            }
            Direction::Row => {
                let dx = avail.origin.x - r.border_box.origin.x;
                r.border_box.origin.x += dx;
                r.content_box.origin.x += dx;
            }
        }
    }
}

impl Default for LayoutEngine {
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

    fn viewport() -> Size {
        Size::new(800.0, 600.0)
    }

    // ---- existing tests (unchanged behaviour) ----------------------------

    #[test]
    fn single_absolute_node() {
        let mut engine = LayoutEngine::new();
        let nodes = vec![LayoutNode {
            id: 0,
            style: LayoutStyle::absolute(10.0, 20.0, 100.0, 50.0),
            children: vec![],
            text: String::new(),
            font_size: 14.0,
        }];
        engine.compute(&nodes, viewport());
        let r = engine.result(0).unwrap();
        assert_eq!(r.border_box, Rect::new(10.0, 20.0, 100.0, 50.0));
    }

    #[test]
    fn stacked_children() {
        let mut engine = LayoutEngine::new();
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    width: Some(200.0),
                    ..Default::default()
                },
                children: vec![1, 2],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle {
                    height: Some(30.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 2,
                style: LayoutStyle {
                    height: Some(40.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        let r0 = engine.result(0).unwrap();
        assert_eq!(r0.border_box.size.height, 70.0);
    }

    // ---- Direction::Row --------------------------------------------------

    #[test]
    fn row_stacks_horizontally() {
        let mut engine = LayoutEngine::new();
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    direction: Direction::Row,
                    height: Some(50.0),
                    ..Default::default()
                },
                children: vec![1, 2],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle {
                    width: Some(60.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 2,
                style: LayoutStyle {
                    width: Some(80.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        let r1 = engine.result(1).unwrap();
        let r2 = engine.result(2).unwrap();
        // Child 1 starts at x=0, child 2 at x=60.
        assert_eq!(r1.border_box.origin.x, 0.0);
        assert_eq!(r2.border_box.origin.x, 60.0);
        // Row auto-sizes width: 60+80=140.
        let r0 = engine.result(0).unwrap();
        assert_eq!(r0.border_box.size.width, 140.0);
    }

    #[test]
    fn row_gap() {
        let mut engine = LayoutEngine::new();
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    direction: Direction::Row,
                    height: Some(40.0),
                    gap: 10.0,
                    ..Default::default()
                },
                children: vec![1, 2],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle {
                    width: Some(30.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 2,
                style: LayoutStyle {
                    width: Some(50.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        let r2 = engine.result(2).unwrap();
        // Child 2 starts after child 1 + gap: 30 + 10 = 40.
        assert_eq!(r2.border_box.origin.x, 40.0);
    }

    // ---- flex_grow -------------------------------------------------------

    #[test]
    fn flex_grow_single_column() {
        let mut engine = LayoutEngine::new();
        // Root: explicit 100h column. Child 1: 20h fixed. Child 2: grows.
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    width: Some(200.0),
                    height: Some(100.0),
                    ..Default::default()
                },
                children: vec![1, 2],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle {
                    height: Some(20.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 2,
                style: LayoutStyle {
                    flex_grow: 1.0,
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        let r2 = engine.result(2).unwrap();
        // Remaining after child1: 100 - 20 = 80, all goes to grow child.
        assert_eq!(r2.border_box.size.height, 80.0);
        assert_eq!(r2.border_box.origin.y, 20.0);
    }

    #[test]
    fn flex_grow_proportional_row() {
        let mut engine = LayoutEngine::new();
        // Root: explicit 120w row. Child 1 grow=1, Child 2 grow=2.
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    direction: Direction::Row,
                    width: Some(120.0),
                    height: Some(40.0),
                    ..Default::default()
                },
                children: vec![1, 2],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle {
                    flex_grow: 1.0,
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 2,
                style: LayoutStyle {
                    flex_grow: 2.0,
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        let r1 = engine.result(1).unwrap();
        let r2 = engine.result(2).unwrap();
        // grow=1 gets 40px, grow=2 gets 80px.
        assert_eq!(r1.border_box.size.width, 40.0);
        assert_eq!(r2.border_box.size.width, 80.0);
        assert_eq!(r2.border_box.origin.x, 40.0);
    }

    // ---- margin & padding -----------------------------------------------

    #[test]
    fn column_margin_and_padding() {
        let mut engine = LayoutEngine::new();
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    width: Some(200.0),
                    padding: Insets::all(10.0),
                    ..Default::default()
                },
                children: vec![1],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle {
                    height: Some(30.0),
                    margin: Insets::all(5.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        let r0 = engine.result(0).unwrap();
        let r1 = engine.result(1).unwrap();
        // Child x = padding.left + margin.left = 10 + 5 = 15
        // But stack fills content_box width: w = 200 - 2*10 - 2*5 = 170
        assert_eq!(r1.border_box.origin.x, 15.0);
        assert_eq!(r1.border_box.origin.y, 15.0); // padding.top + margin.top
                                                  // Root height: padding(20) + margin(10) + child(30) + margin(10) = 50... actually:
                                                  // children_h = cursor - content.origin.y = (15 + 30 + 5) - 10 = 40
                                                  // border_h = 40 + 20 (padding.vertical) = 60
        assert_eq!(r0.border_box.size.height, 60.0);
    }

    // ---- absolute inside stack ------------------------------------------

    #[test]
    fn absolute_inside_stack() {
        let mut engine = LayoutEngine::new();
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    width: Some(200.0),
                    height: Some(200.0),
                    ..Default::default()
                },
                children: vec![1, 2],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle::absolute(5.0, 10.0, 50.0, 20.0),
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 2,
                style: LayoutStyle {
                    height: Some(30.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        let r1 = engine.result(1).unwrap();
        let r2 = engine.result(2).unwrap();
        // Absolute child at (5,10).
        assert_eq!(r1.border_box, Rect::new(5.0, 10.0, 50.0, 20.0));
        // Stack child starts at y=0 (absolute siblings don't consume flow space).
        assert_eq!(r2.border_box.origin.y, 0.0);
    }

    // ---- TextMeasure / DefaultTextMeasure --------------------------------

    #[test]
    fn default_text_measure_empty() {
        let tm = DefaultTextMeasure;
        assert_eq!(tm.measure("", 14.0, f32::INFINITY), Size::new(0.0, 0.0));
    }

    #[test]
    fn default_text_measure_single_line() {
        let tm = DefaultTextMeasure;
        // "ABC": 3 chars, scale=1, width = 6*3 - 1 = 17, height = 7
        let s = tm.measure("ABC", 14.0, f32::INFINITY);
        assert_eq!(s.width, 17.0);
        assert_eq!(s.height, 7.0);
    }

    #[test]
    fn default_text_measure_wraps() {
        let tm = DefaultTextMeasure;
        // max_width=10: 2 chars → 6*2-1=11 > 10 → 1 char per line.
        // "ABC" → 3 lines, each width=5, height=7*3=21.
        let s = tm.measure("ABC", 14.0, 10.0);
        assert_eq!(s.width, 5.0);
        assert_eq!(s.height, 21.0);
    }

    #[test]
    fn default_text_measure_scale() {
        let tm = DefaultTextMeasure;
        // font_size=28 → scale=2; "AB": width = (6*2-1)*2 = 22, height = 7*2 = 14
        let s = tm.measure("AB", 28.0, f32::INFINITY);
        assert_eq!(s.width, 22.0);
        assert_eq!(s.height, 14.0);
    }

    // ---- Text node auto-sizing ------------------------------------------

    #[test]
    fn text_node_auto_sizes() {
        let mut engine = LayoutEngine::new();
        // Root: 100w column. Child: text node with "HI" at 14px.
        // "HI": width = 6*2-1 = 11, height = 7.
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    width: Some(100.0),
                    ..Default::default()
                },
                children: vec![1],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle::default(),
                children: vec![],
                text: "HI".to_string(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        let r1 = engine.result(1).unwrap();
        assert_eq!(r1.border_box.size.width, 11.0);
        assert_eq!(r1.border_box.size.height, 7.0);
    }

    #[test]
    fn text_node_wraps_in_narrow_parent() {
        let mut engine = LayoutEngine::new();
        // Root: 10w column. Child: text node with "ABC".
        // max_width = 10 → 1 char per line → 3 lines, height=21.
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    width: Some(10.0),
                    ..Default::default()
                },
                children: vec![1],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle::default(),
                children: vec![],
                text: "ABC".to_string(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        let r1 = engine.result(1).unwrap();
        assert_eq!(r1.border_box.size.height, 21.0);
    }

    #[test]
    fn text_node_in_row() {
        let mut engine = LayoutEngine::new();
        // Root: row, 200w 50h. Child: text "OK" — should sit at x=0.
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    direction: Direction::Row,
                    width: Some(200.0),
                    height: Some(50.0),
                    ..Default::default()
                },
                children: vec![1],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle::default(),
                children: vec![],
                text: "OK".to_string(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        let r1 = engine.result(1).unwrap();
        assert_eq!(r1.border_box.origin.x, 0.0);
        // "OK": width = 6*2-1 = 11.
        assert_eq!(r1.border_box.size.width, 11.0);
    }

    // ---- gap in column ---------------------------------------------------

    #[test]
    fn column_gap() {
        let mut engine = LayoutEngine::new();
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    width: Some(100.0),
                    gap: 8.0,
                    ..Default::default()
                },
                children: vec![1, 2],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle {
                    height: Some(20.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 2,
                style: LayoutStyle {
                    height: Some(15.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        let r2 = engine.result(2).unwrap();
        // Child 2 starts after child1 + gap = 20 + 8 = 28.
        assert_eq!(r2.border_box.origin.y, 28.0);
        let r0 = engine.result(0).unwrap();
        // Root height: 20 + 8 + 15 = 43.
        assert_eq!(r0.border_box.size.height, 43.0);
    }

    // ---- nested column inside row ----------------------------------------

    #[test]
    fn nested_column_in_row() {
        let mut engine = LayoutEngine::new();
        // Root: row 300w 100h.
        // Child 0: column 100w, 2 stacked children (20h + 30h).
        // Child 1: fixed 80w.
        let nodes = vec![
            LayoutNode {
                id: 0,
                style: LayoutStyle {
                    direction: Direction::Row,
                    width: Some(300.0),
                    height: Some(100.0),
                    ..Default::default()
                },
                children: vec![1, 4],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 1,
                style: LayoutStyle {
                    direction: Direction::Column,
                    width: Some(100.0),
                    ..Default::default()
                },
                children: vec![2, 3],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 2,
                style: LayoutStyle {
                    height: Some(20.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 3,
                style: LayoutStyle {
                    height: Some(30.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
            LayoutNode {
                id: 4,
                style: LayoutStyle {
                    width: Some(80.0),
                    ..Default::default()
                },
                children: vec![],
                text: String::new(),
                font_size: 14.0,
            },
        ];
        engine.compute(&nodes, viewport());
        // Inner column auto-height = 50.
        let r1 = engine.result(1).unwrap();
        assert_eq!(r1.border_box.size.height, 50.0);
        // Child 4 starts at x=100 (after the column).
        let r4 = engine.result(4).unwrap();
        assert_eq!(r4.border_box.origin.x, 100.0);
    }
}
