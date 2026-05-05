use webgpui_geometry::{Insets, Rect};

use crate::style::LayoutStyle;

/// A node fed into the layout engine.
#[derive(Debug)]
pub struct LayoutNode {
    pub id: u32,
    pub style: LayoutStyle,
    /// Indices into the same flat array of layout nodes (children).
    pub children: Vec<usize>,
    /// Text content.  Non-empty signals that this node should be auto-sized
    /// via the [`TextMeasure`](crate::measure::TextMeasure) provided to
    /// [`LayoutEngine::compute_with`](crate::engine::LayoutEngine::compute_with).
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

/// Computed layout for a single node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutResult {
    /// Outer border box.
    pub border_box: Rect,
    /// Inner content area (border_box minus padding).
    pub content_box: Rect,
}

impl LayoutResult {
    pub(crate) fn from_border(border: Rect, padding: Insets) -> Self {
        Self {
            border_box: border,
            content_box: border.shrink(padding),
        }
    }

    pub(crate) fn zero() -> Self {
        Self {
            border_box: Rect::ZERO,
            content_box: Rect::ZERO,
        }
    }
}
