//! Core geometric primitives used throughout webgpui.

use bytemuck::{Pod, Zeroable};

// ---------------------------------------------------------------------------
// Point
// ---------------------------------------------------------------------------

/// A two-dimensional point in logical pixel space.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn distance_to(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

impl std::ops::Add for Point {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Add<Size> for Point {
    type Output = Self;
    fn add(self, rhs: Size) -> Self {
        Self::new(self.x + rhs.width, self.y + rhs.height)
    }
}

impl From<(f32, f32)> for Point {
    fn from((x, y): (f32, f32)) -> Self {
        Self::new(x, y)
    }
}

// ---------------------------------------------------------------------------
// Size
// ---------------------------------------------------------------------------

/// A two-dimensional size in logical pixel space.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    #[inline]
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Returns `true` if `width` or `height` is `<= 0.0`.
    ///
    /// Both zero-area and negative-dimension sizes are considered empty.
    /// Negative dimensions can arise from unclamped layout arithmetic.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

impl From<(f32, f32)> for Size {
    fn from((w, h): (f32, f32)) -> Self {
        Self::new(w, h)
    }
}

impl From<(u32, u32)> for Size {
    fn from((w, h): (u32, u32)) -> Self {
        Self::new(w as f32, h as f32)
    }
}

// ---------------------------------------------------------------------------
// Rect
// ---------------------------------------------------------------------------

/// An axis-aligned rectangle defined by an origin and size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub const ZERO: Self = Self {
        origin: Point::ZERO,
        size: Size::ZERO,
    };

    #[inline]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    #[inline]
    pub fn from_origin_size(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    #[inline]
    pub fn min_x(self) -> f32 {
        self.origin.x
    }
    #[inline]
    pub fn min_y(self) -> f32 {
        self.origin.y
    }
    #[inline]
    pub fn max_x(self) -> f32 {
        self.origin.x + self.size.width
    }
    #[inline]
    pub fn max_y(self) -> f32 {
        self.origin.y + self.size.height
    }

    /// Returns `true` if `p` lies within this rectangle.
    ///
    /// Uses half-open interval semantics: `min <= coord < max` on the trailing
    /// edges, so a point exactly on a shared boundary belongs to only one rect.
    #[inline]
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.min_x() && p.x < self.max_x() && p.y >= self.min_y() && p.y < self.max_y()
    }

    /// Returns the intersection of two rectangles, or `None` if they don't overlap.
    pub fn intersect(self, other: Self) -> Option<Self> {
        let x0 = self.min_x().max(other.min_x());
        let y0 = self.min_y().max(other.min_y());
        let x1 = self.max_x().min(other.max_x());
        let y1 = self.max_y().min(other.max_y());
        if x1 > x0 && y1 > y0 {
            Some(Self::new(x0, y0, x1 - x0, y1 - y0))
        } else {
            None
        }
    }

    /// Returns the smallest rectangle that contains both rectangles.
    pub fn union(self, other: Self) -> Self {
        let x0 = self.min_x().min(other.min_x());
        let y0 = self.min_y().min(other.min_y());
        let x1 = self.max_x().max(other.max_x());
        let y1 = self.max_y().max(other.max_y());
        Self::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// Expands the rect outward by `insets` on all sides.
    ///
    /// Negative inset components shrink the corresponding side without clamping;
    /// this can produce a rect with negative width or height. Use [`shrink`][Self::shrink]
    /// when you want clamping at zero.
    ///
    /// Note: `expand(i).shrink(i)` is an identity only when the rect stays
    /// positive after expansion; `shrink(i).expand(i)` is an identity only when
    /// the shrink does not clamp.
    pub fn expand(self, insets: Insets) -> Self {
        Self::new(
            self.origin.x - insets.left,
            self.origin.y - insets.top,
            self.size.width + insets.left + insets.right,
            self.size.height + insets.top + insets.bottom,
        )
    }

    /// Shrinks the rect inward by `insets` on all sides.
    ///
    /// Width and height are clamped to `0.0` so the result is never
    /// negative-sized. Negative inset components expand the corresponding side
    /// without clamping.
    ///
    /// Note: `shrink(i).expand(i)` round-trips only when no dimension was clamped.
    pub fn shrink(self, insets: Insets) -> Self {
        Self::new(
            self.origin.x + insets.left,
            self.origin.y + insets.top,
            (self.size.width - insets.left - insets.right).max(0.0),
            (self.size.height - insets.top - insets.bottom).max(0.0),
        )
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.size.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Insets
// ---------------------------------------------------------------------------

/// Distances from each edge of a rectangle (used for margin / padding).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Insets {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    #[inline]
    pub fn all(v: f32) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    #[inline]
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    #[inline]
    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }
    #[inline]
    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

/// An RGBA colour with components in `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    #[inline]
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub fn from_rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    #[inline]
    pub fn from_rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::from_rgba_u8(r, g, b, 255)
    }

    #[inline]
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Returns the colour with a different alpha value.
    #[inline]
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Pre-multiplied alpha version.
    #[inline]
    pub fn premultiply(self) -> Self {
        Self::new(self.r * self.a, self.g * self.a, self.b * self.a, self.a)
    }
}

// ---------------------------------------------------------------------------
// BorderRadius
// ---------------------------------------------------------------------------

/// Corner radii for a rounded rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl BorderRadius {
    pub const ZERO: Self = Self {
        top_left: 0.0,
        top_right: 0.0,
        bottom_right: 0.0,
        bottom_left: 0.0,
    };

    #[inline]
    pub fn all(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        self.top_left == 0.0
            && self.top_right == 0.0
            && self.bottom_right == 0.0
            && self.bottom_left == 0.0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains() {
        let r = Rect::new(10.0, 10.0, 100.0, 50.0);
        assert!(r.contains(Point::new(50.0, 30.0)));
        assert!(!r.contains(Point::new(5.0, 30.0)));
    }

    #[test]
    fn rect_intersect() {
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(50.0, 50.0, 100.0, 100.0);
        let i = a.intersect(b).unwrap();
        assert_eq!(i, Rect::new(50.0, 50.0, 50.0, 50.0));
    }

    #[test]
    fn color_from_u8() {
        let c = Color::from_rgba_u8(255, 128, 0, 255);
        assert!((c.r - 1.0).abs() < 1e-6);
        assert!((c.g - 128.0 / 255.0).abs() < 1e-6);
    }
}
