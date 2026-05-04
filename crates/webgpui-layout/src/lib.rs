//! MVP layout engine for webgpui.
//!
//! Supports two layout modes:
//! - **Absolute** – the node is positioned at an explicit (`x`, `y`) relative
//!   to its parent's content box.
//! - **Stack** – children are stacked vertically (top-to-bottom) inside the
//!   parent's content box.
//!
//! Margin and padding are fully respected.  Width / height may be given as an
//! explicit pixel value or left as `None` to fill the available space.

use webgpui_geometry::{Insets, Point, Rect, Size};

// ---------------------------------------------------------------------------
// PositionType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PositionType {
    /// The node is placed at (`x`, `y`) relative to its parent's content area.
    Absolute,
    /// The node participates in the parent's stack flow.
    #[default]
    Stack,
}

// ---------------------------------------------------------------------------
// LayoutStyle
// ---------------------------------------------------------------------------

/// Per-node layout properties.
#[derive(Debug, Clone)]
pub struct LayoutStyle {
    pub position: PositionType,
    /// Explicit x-offset for `Absolute` nodes.
    pub x: f32,
    /// Explicit y-offset for `Absolute` nodes.
    pub y: f32,
    /// Explicit width.  `None` means "fill the available width".
    pub width: Option<f32>,
    /// Explicit height. `None` means "shrink-wrap children".
    pub height: Option<f32>,
    pub margin: Insets,
    pub padding: Insets,
    /// Gap between stacked children.
    pub gap: f32,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            position: PositionType::Stack,
            x: 0.0,
            y: 0.0,
            width: None,
            height: None,
            margin: Insets::ZERO,
            padding: Insets::ZERO,
            gap: 0.0,
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
// LayoutNode  (input to the engine)
// ---------------------------------------------------------------------------

/// A node fed into the layout engine.
#[derive(Debug)]
pub struct LayoutNode {
    pub id: u32,
    pub style: LayoutStyle,
    /// Indices into the same flat array of layout nodes (for the children).
    pub children: Vec<usize>,
}

impl LayoutNode {
    pub fn new(id: u32, style: LayoutStyle) -> Self {
        Self {
            id,
            style,
            children: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// LayoutResult  (output)
// ---------------------------------------------------------------------------

/// Computed layout for a single node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutResult {
    /// Outer border box (includes margin only in flow position; excludes
    /// margin in absolute positioning).
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
}

// ---------------------------------------------------------------------------
// LayoutEngine
// ---------------------------------------------------------------------------

/// Computes layout for a flat array of [`LayoutNode`]s.
///
/// The root node is at index `0`.  After calling [`LayoutEngine::compute`]
/// the results are available via [`LayoutEngine::result`].
pub struct LayoutEngine {
    results: Vec<LayoutResult>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Compute layout for `nodes`, given a viewport of `viewport` size.
    ///
    /// Results are keyed by `LayoutNode::id`, not by index.
    /// Use [`LayoutEngine::result`] to query them.
    pub fn compute(&mut self, nodes: &[LayoutNode], viewport: Size) {
        self.results = vec![
            LayoutResult {
                border_box: Rect::ZERO,
                content_box: Rect::ZERO,
            };
            nodes.len()
        ];
        if nodes.is_empty() {
            return;
        }
        let available = Rect::from_origin_size(Point::ZERO, viewport);
        self.layout_node(nodes, 0, available);
    }

    /// Returns the computed result for the node at arena **index**, or `None`.
    ///
    /// Note: results are keyed by arena index, not by `LayoutNode::id`.
    pub fn result(&self, index: usize) -> Option<LayoutResult> {
        self.results.get(index).copied()
    }

    // ------------------------------------------------------------------
    // Internal recursive layout
    // ------------------------------------------------------------------

    fn layout_node(&mut self, nodes: &[LayoutNode], idx: usize, parent_content: Rect) {
        let node = &nodes[idx];
        let style = &node.style;

        let margin = style.margin;
        let padding = style.padding;

        // Resolve this node's border box.
        let border_box = match style.position {
            PositionType::Absolute => {
                let w = style.width.unwrap_or(parent_content.size.width);
                let h = style.height.unwrap_or(0.0);
                Rect::new(
                    parent_content.origin.x + style.x,
                    parent_content.origin.y + style.y,
                    w,
                    h,
                )
            }
            PositionType::Stack => {
                // Width fills parent minus horizontal margin.
                let w = style
                    .width
                    .unwrap_or(parent_content.size.width - margin.horizontal());
                // Height will be finalised after children are laid out.
                let h = style.height.unwrap_or(0.0);
                Rect::new(
                    parent_content.origin.x + margin.left,
                    parent_content.origin.y + margin.top,
                    w,
                    h,
                )
            }
        };

        let content_box = border_box.shrink(padding);
        self.results[idx] = LayoutResult::from_border(border_box, padding);

        // Layout children, accumulating total stacked height.
        let mut cursor_y = content_box.origin.y;
        for (child_idx, &ci) in node.children.iter().enumerate() {
            let child_available = Rect::from_origin_size(
                Point::new(content_box.origin.x, cursor_y),
                Size::new(content_box.size.width, content_box.size.height),
            );

            match nodes[ci].style.position {
                PositionType::Absolute => {
                    self.layout_node(nodes, ci, content_box);
                }
                PositionType::Stack => {
                    self.layout_node(nodes, ci, child_available);
                    let child_result = self.results[ci];
                    cursor_y = child_result.border_box.max_y() + nodes[ci].style.margin.bottom;
                    // Add gap after all but the last child.
                    if child_idx + 1 < node.children.len() {
                        cursor_y += nodes[idx].style.gap;
                    }
                }
            }
        }

        // If height was not explicitly set, shrink-wrap around children.
        if nodes[idx].style.height.is_none() {
            let children_height = cursor_y - content_box.origin.y;
            let new_h = children_height + padding.vertical();
            self.results[idx].border_box.size.height = new_h.max(0.0);
            self.results[idx].content_box = self.results[idx].border_box.shrink(padding);
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

    #[test]
    fn single_absolute_node() {
        let mut engine = LayoutEngine::new();
        let nodes = vec![LayoutNode {
            id: 0,
            style: LayoutStyle::absolute(10.0, 20.0, 100.0, 50.0),
            children: vec![],
        }];
        engine.compute(&nodes, Size::new(800.0, 600.0));
        let r = engine.result(0).unwrap();
        assert_eq!(r.border_box, Rect::new(10.0, 20.0, 100.0, 50.0));
    }

    #[test]
    fn stacked_children() {
        let mut engine = LayoutEngine::new();
        let mut root = LayoutNode {
            id: 0,
            style: LayoutStyle {
                width: Some(200.0),
                ..Default::default()
            },
            children: vec![1, 2],
        };
        root.children = vec![1, 2];
        let child1 = LayoutNode {
            id: 1,
            style: LayoutStyle {
                height: Some(30.0),
                ..Default::default()
            },
            children: vec![],
        };
        let child2 = LayoutNode {
            id: 2,
            style: LayoutStyle {
                height: Some(40.0),
                ..Default::default()
            },
            children: vec![],
        };
        let nodes = vec![root, child1, child2];
        engine.compute(&nodes, Size::new(800.0, 600.0));
        let r0 = engine.result(0).unwrap();
        // Root shrink-wraps children: 30 + 40 = 70
        assert_eq!(r0.border_box.size.height, 70.0);
    }
}
