use webgpui_geometry::Size;

/// Measures a text string given a font size and a maximum available width.
///
/// Implementations are object-safe so they can be passed as `&dyn TextMeasure`.
pub trait TextMeasure {
    /// Returns the `(width, height)` bounding box of `text` rendered at
    /// `font_size` and wrapped to `max_width` (pass `f32::INFINITY` to disable
    /// wrapping).
    fn measure(&self, text: &str, font_size: f32, max_width: f32) -> Size;
}

/// Pixel-font implementation (`FONT_W = 5`, `FONT_H = 7`, 1 px advance gap).
///
/// Scale is derived from `font_size / 14.0` (the default `NodeStyle::font_size`).
pub struct DefaultTextMeasure;

const FONT_W: f32 = 5.0;
const FONT_H: f32 = 7.0;
const BASE_FONT_SIZE: f32 = 14.0;

impl TextMeasure for DefaultTextMeasure {
    fn measure(&self, text: &str, font_size: f32, max_width: f32) -> Size {
        if font_size <= 0.0 {
            return Size::new(0.0, 0.0);
        }
        let total = text.chars().count();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_font_size_returns_zero() {
        let m = DefaultTextMeasure;
        let s = m.measure("hello", 0.0, f32::INFINITY);
        assert_eq!(s.width, 0.0);
        assert_eq!(s.height, 0.0);
    }

    #[test]
    fn negative_font_size_returns_zero() {
        let m = DefaultTextMeasure;
        let s = m.measure("hello", -14.0, f32::INFINITY);
        assert_eq!(s.width, 0.0);
        assert_eq!(s.height, 0.0);
    }

    #[test]
    fn measure_does_not_panic_on_multibyte() {
        let m = DefaultTextMeasure;
        // "éàü" — 3 Unicode chars, each multiple bytes — count should be 3
        let s = m.measure("éàü", 14.0, f32::INFINITY);
        assert!(s.width > 0.0);
        assert!(s.height > 0.0);
    }
}
