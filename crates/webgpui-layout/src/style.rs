use webgpui_geometry::Insets;

use crate::direction::Direction;

/// Controls how a node is positioned within its parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PositionType {
    /// Positioned at `(x, y)` relative to the parent content area.
    Absolute,
    /// Participates in the parent's stack flow.
    #[default]
    Stack,
}

/// Per-node layout properties.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutStyle {
    /// How the node is placed within its parent (stack flow or absolute).
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
    /// Space outside the border box.
    pub margin: Insets,
    /// Space between the border box and the content area.
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
    /// Shorthand for an absolutely-positioned node with explicit position and size.
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

    /// Explicit size along the main axis (`height` for Column, `width` for Row).
    pub(crate) fn main_size(&self, dir: Direction) -> Option<f32> {
        match dir {
            Direction::Column => self.height,
            Direction::Row => self.width,
        }
    }

    /// Explicit size along the cross axis (`width` for Column, `height` for Row).
    pub(crate) fn cross_size(&self, dir: Direction) -> Option<f32> {
        match dir {
            Direction::Column => self.width,
            Direction::Row => self.height,
        }
    }
}
