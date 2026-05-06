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
    #[inline(always)]
    fn sel<T>(self, if_col: T, if_row: T) -> T {
        match self {
            Direction::Column => if_col,
            Direction::Row => if_row,
        }
    }

    pub(crate) fn main_of(self, s: Size) -> f32 {
        self.sel(s.height, s.width)
    }
    pub(crate) fn cross_of(self, s: Size) -> f32 {
        self.sel(s.width, s.height)
    }
    pub(crate) fn main_insets(self, ins: Insets) -> f32 {
        self.sel(ins.vertical(), ins.horizontal())
    }
    pub(crate) fn cross_insets(self, ins: Insets) -> f32 {
        self.sel(ins.horizontal(), ins.vertical())
    }
    pub(crate) fn main_leading(self, ins: Insets) -> f32 {
        self.sel(ins.top, ins.left)
    }
    pub(crate) fn main_trailing(self, ins: Insets) -> f32 {
        self.sel(ins.bottom, ins.right)
    }
    pub(crate) fn main_origin(self, p: Point) -> f32 {
        self.sel(p.y, p.x)
    }
    pub(crate) fn cross_origin(self, p: Point) -> f32 {
        self.sel(p.x, p.y)
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

    pub(crate) fn size(self, main: f32, cross: f32) -> Size {
        self.sel(Size::new(cross, main), Size::new(main, cross))
    }
    pub(crate) fn point(self, main: f32, cross: f32) -> Point {
        self.sel(Point::new(cross, main), Point::new(main, cross))
    }
    pub(crate) fn main_max(self, r: Rect) -> f32 {
        self.sel(r.max_y(), r.max_x())
    }
}
