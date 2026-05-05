use webgpui_geometry::{Insets, Point, Rect, Size};

/// Main axis for stack layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Children stacked top-to-bottom (default).
    #[default]
    Column,
    /// Children stacked left-to-right.
    Row,
}

impl Direction {
    pub(crate) fn main_of(self, s: Size) -> f32 {
        match self {
            Direction::Column => s.height,
            Direction::Row => s.width,
        }
    }
    pub(crate) fn cross_of(self, s: Size) -> f32 {
        match self {
            Direction::Column => s.width,
            Direction::Row => s.height,
        }
    }
    pub(crate) fn main_insets(self, ins: Insets) -> f32 {
        match self {
            Direction::Column => ins.vertical(),
            Direction::Row => ins.horizontal(),
        }
    }
    pub(crate) fn cross_insets(self, ins: Insets) -> f32 {
        match self {
            Direction::Column => ins.horizontal(),
            Direction::Row => ins.vertical(),
        }
    }
    pub(crate) fn main_leading(self, ins: Insets) -> f32 {
        match self {
            Direction::Column => ins.top,
            Direction::Row => ins.left,
        }
    }
    pub(crate) fn main_trailing(self, ins: Insets) -> f32 {
        match self {
            Direction::Column => ins.bottom,
            Direction::Row => ins.right,
        }
    }
    pub(crate) fn main_origin(self, p: Point) -> f32 {
        match self {
            Direction::Column => p.y,
            Direction::Row => p.x,
        }
    }
    pub(crate) fn cross_origin(self, p: Point) -> f32 {
        match self {
            Direction::Column => p.x,
            Direction::Row => p.y,
        }
    }
    pub(crate) fn main_origin_mut(self, p: &mut Point) -> &mut f32 {
        match self {
            Direction::Column => &mut p.y,
            Direction::Row => &mut p.x,
        }
    }
    pub(crate) fn main_size_mut(self, s: &mut Size) -> &mut f32 {
        match self {
            Direction::Column => &mut s.height,
            Direction::Row => &mut s.width,
        }
    }
    /// Build a `Size` from (main, cross) components.
    pub(crate) fn size(self, main: f32, cross: f32) -> Size {
        match self {
            Direction::Column => Size::new(cross, main),
            Direction::Row => Size::new(main, cross),
        }
    }
    /// Build a `Point` from (main, cross) components.
    pub(crate) fn point(self, main: f32, cross: f32) -> Point {
        match self {
            Direction::Column => Point::new(cross, main),
            Direction::Row => Point::new(main, cross),
        }
    }
    /// max_y (Column) or max_x (Row) of a rect.
    pub(crate) fn main_max(self, r: Rect) -> f32 {
        match self {
            Direction::Column => r.max_y(),
            Direction::Row => r.max_x(),
        }
    }
}
