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

    /// A long string that cannot fit on a single line should produce a height
    /// greater than one line and a width that does not exceed `max_width`.
    #[test]
    fn wrap_increases_height() {
        // font_size = BASE_FONT_SIZE → scale = 1.0
        // char_advance = (FONT_W + 1.0) * scale = 6.0
        // line_h       = FONT_H * scale          = 7.0
        // Single-line width of "abcdefghij" (10 chars) = 6*10 - 1 = 59.0
        // max_width = 29.0 forces a break (only 5 chars fit per line: 6*5-1=29).
        let m = DefaultTextMeasure;
        let scale = 1.0_f32;
        let line_h = FONT_H * scale; // 7.0
        let char_advance = (FONT_W + 1.0) * scale; // 6.0
                                                   // Width that fits exactly 5 chars: 6*5 - 1 = 29.0
        let max_width = char_advance * 5.0 - scale;

        let s = m.measure("abcdefghij", BASE_FONT_SIZE, max_width);

        assert!(
            s.height > line_h,
            "expected height > {line_h} (more than one line), got {}",
            s.height
        );
        let epsilon = 1e-4;
        assert!(
            s.width <= max_width + epsilon,
            "expected width <= {max_width}, got {}",
            s.width
        );
    }

    /// "hello world" with `max_width` that fits 6 chars per line forces the
    /// string onto exactly 2 lines: "hello " then "world".
    #[test]
    fn wrap_line_count_matches_words() {
        // scale = 1.0 (font_size == BASE_FONT_SIZE)
        // char_advance = 6.0,  line_h = 7.0
        // "hello " = 6 chars → width = 6*6 - 1 = 35.0  (fits)
        // "hello  " = 7 chars → width = 6*7 - 1 = 41.0 (exceeds max_width=35.0)
        // So line 1 = "hello " (chars 0..6), line 2 = "world" (chars 6..11).
        let m = DefaultTextMeasure;
        let scale = 1.0_f32;
        let line_h = FONT_H * scale; // 7.0
        let char_advance = (FONT_W + 1.0) * scale; // 6.0
                                                   // max_width that fits exactly 6 chars: 6*6 - 1 = 35.0
        let max_width = char_advance * 6.0 - scale;

        let s = m.measure("hello world", BASE_FONT_SIZE, max_width);

        let expected_height = 2.0 * line_h; // 14.0
        assert!(
            (s.height - expected_height).abs() < 1e-4,
            "expected height {expected_height}, got {}",
            s.height
        );
    }

    /// When the text fits within `max_width`, the result must be a single line.
    #[test]
    fn no_wrap_when_fits() {
        // "hi" = 2 chars, width = 6*2 - 1 = 11.0 at scale 1.0
        // max_width = 100.0 → no wrap, height = 1 * 7.0 = 7.0
        let m = DefaultTextMeasure;
        let scale = 1.0_f32;
        let line_h = FONT_H * scale; // 7.0

        let s = m.measure("hi", BASE_FONT_SIZE, 100.0);

        let expected_height = line_h; // 7.0
        assert!(
            (s.height - expected_height).abs() < 1e-4,
            "expected height {expected_height}, got {}",
            s.height
        );
    }
}
