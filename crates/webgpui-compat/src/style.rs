//! Style mutation — MUST-tier (api-mapping.md §4, §13.4).

use webgpui_core::NodeStyle;
use webgpui_geometry::{Color, Insets};
use webgpui_layout::{LayoutStyle, PositionType};

use crate::state::with_state;
use crate::types::{CompatError, CompatResult, NodeId, StyleProp};

// ---------------------------------------------------------------------------
// Color parsing
// ---------------------------------------------------------------------------

fn hex1(c: char) -> u8 {
    match c {
        '0'..='9' => c as u8 - b'0',
        'a'..='f' => c as u8 - b'a' + 10,
        'A'..='F' => c as u8 - b'A' + 10,
        _ => 0,
    }
}

fn hex2(s: &str) -> Option<u8> {
    u8::from_str_radix(s, 16).ok()
}

fn parse_color(s: &str) -> CompatResult<Color> {
    let s = s.trim();
    let hex = s
        .strip_prefix('#')
        .ok_or_else(|| CompatError::StyleParseError(format!("unsupported color: {s}")))?;
    let (r, g, b, a) = match hex.len() {
        3 => {
            let r = hex1(hex.as_bytes()[0] as char);
            let g = hex1(hex.as_bytes()[1] as char);
            let b = hex1(hex.as_bytes()[2] as char);
            (r << 4 | r, g << 4 | g, b << 4 | b, 255u8)
        }
        6 => {
            let r = hex2(&hex[0..2]).ok_or_else(|| bad_color(s))?;
            let g = hex2(&hex[2..4]).ok_or_else(|| bad_color(s))?;
            let b = hex2(&hex[4..6]).ok_or_else(|| bad_color(s))?;
            (r, g, b, 255u8)
        }
        8 => {
            let r = hex2(&hex[0..2]).ok_or_else(|| bad_color(s))?;
            let g = hex2(&hex[2..4]).ok_or_else(|| bad_color(s))?;
            let b = hex2(&hex[4..6]).ok_or_else(|| bad_color(s))?;
            let a = hex2(&hex[6..8]).ok_or_else(|| bad_color(s))?;
            (r, g, b, a)
        }
        _ => return Err(bad_color(s)),
    };
    Ok(Color::from_rgba_u8(r, g, b, a))
}

fn bad_color(s: &str) -> CompatError {
    CompatError::StyleParseError(format!("invalid color: {s}"))
}

// ---------------------------------------------------------------------------
// Read-modify-write helpers
// ---------------------------------------------------------------------------

fn modify_visual<F>(node: NodeId, f: F) -> CompatResult<()>
where
    F: FnOnce(&mut NodeStyle),
{
    with_state(|s| {
        if let Some(staged) = s.staged.get_mut(&node.0) {
            f(&mut staged.style);
            return Ok(());
        }
        if let Some(core_id) = s.core_id_of(node.0) {
            let node_ref = s.tree.get_mut(core_id).ok_or(CompatError::InvalidNode)?;
            f(&mut node_ref.style);
            return Ok(());
        }
        Err(CompatError::InvalidNode)
    })
}

fn modify_layout<F>(node: NodeId, f: F) -> CompatResult<()>
where
    F: FnOnce(&mut LayoutStyle),
{
    with_state(|s| {
        if let Some(staged) = s.staged.get_mut(&node.0) {
            f(&mut staged.layout);
            return Ok(());
        }
        if let Some(core_id) = s.core_id_of(node.0) {
            let node_ref = s.tree.get_mut(core_id).ok_or(CompatError::InvalidNode)?;
            f(&mut node_ref.layout);
            return Ok(());
        }
        Err(CompatError::InvalidNode)
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Sets a single style property by CSS-like key string.
pub fn style_set(node: NodeId, key: &str, value: &str) -> CompatResult<()> {
    let prop = StyleProp::from_key(key)
        .ok_or_else(|| CompatError::StyleParseError(format!("unknown style key: {key}")))?;
    match prop {
        StyleProp::Background => style_background(node, value),
        StyleProp::BorderColor => {
            let c = parse_color(value)?;
            modify_visual(node, |s| s.border_color = c)
        }
        StyleProp::BorderWidth => {
            let w: f32 = value.parse().map_err(|_| {
                CompatError::StyleParseError(format!("invalid border-width: {value}"))
            })?;
            modify_visual(node, |s| s.border = Insets::all(w))
        }
        StyleProp::Opacity => {
            let a: f32 = value
                .parse()
                .map_err(|_| CompatError::StyleParseError(format!("invalid opacity: {value}")))?;
            style_opacity(node, a)
        }
        StyleProp::X => {
            let x: f32 = value
                .parse()
                .map_err(|_| CompatError::StyleParseError(format!("invalid x: {value}")))?;
            modify_layout(node, |l| {
                l.position = PositionType::Absolute;
                l.x = x;
            })
        }
        StyleProp::Y => {
            let y: f32 = value
                .parse()
                .map_err(|_| CompatError::StyleParseError(format!("invalid y: {value}")))?;
            modify_layout(node, |l| {
                l.position = PositionType::Absolute;
                l.y = y;
            })
        }
        StyleProp::Width => {
            let w: f32 = value
                .parse()
                .map_err(|_| CompatError::StyleParseError(format!("invalid width: {value}")))?;
            modify_layout(node, |l| l.width = Some(w))
        }
        StyleProp::Height => {
            let h: f32 = value
                .parse()
                .map_err(|_| CompatError::StyleParseError(format!("invalid height: {value}")))?;
            modify_layout(node, |l| l.height = Some(h))
        }
        StyleProp::MarginLeft => {
            let v: f32 = value.parse().map_err(|_| bad_f32(value))?;
            modify_layout(node, |l| l.margin.left = v)
        }
        StyleProp::MarginTop => {
            let v: f32 = value.parse().map_err(|_| bad_f32(value))?;
            modify_layout(node, |l| l.margin.top = v)
        }
        StyleProp::MarginRight => {
            let v: f32 = value.parse().map_err(|_| bad_f32(value))?;
            modify_layout(node, |l| l.margin.right = v)
        }
        StyleProp::MarginBottom => {
            let v: f32 = value.parse().map_err(|_| bad_f32(value))?;
            modify_layout(node, |l| l.margin.bottom = v)
        }
        StyleProp::PaddingLeft => {
            let v: f32 = value.parse().map_err(|_| bad_f32(value))?;
            modify_layout(node, |l| l.padding.left = v)
        }
        StyleProp::PaddingTop => {
            let v: f32 = value.parse().map_err(|_| bad_f32(value))?;
            modify_layout(node, |l| l.padding.top = v)
        }
        StyleProp::PaddingRight => {
            let v: f32 = value.parse().map_err(|_| bad_f32(value))?;
            modify_layout(node, |l| l.padding.right = v)
        }
        StyleProp::PaddingBottom => {
            let v: f32 = value.parse().map_err(|_| bad_f32(value))?;
            modify_layout(node, |l| l.padding.bottom = v)
        }
    }
}

fn bad_f32(v: &str) -> CompatError {
    CompatError::StyleParseError(format!("expected f32, got: {v}"))
}

/// Sets multiple style properties in one call.
pub fn style_set_many(node: NodeId, styles: &[(&str, &str)]) -> CompatResult<()> {
    for &(key, value) in styles {
        style_set(node, key, value)?;
    }
    Ok(())
}

/// Sets `(x, y)` position; switches the node to `Absolute` positioning.
pub fn style_position(node: NodeId, x: f32, y: f32) -> CompatResult<()> {
    modify_layout(node, |l| {
        l.position = PositionType::Absolute;
        l.x = x;
        l.y = y;
    })
}

/// Sets explicit width and/or height.  `None` means "auto".
pub fn style_size(node: NodeId, w: Option<f32>, h: Option<f32>) -> CompatResult<()> {
    modify_layout(node, |l| {
        l.width = w;
        l.height = h;
    })
}

/// Sets all four margin sides.
pub fn style_margin(node: NodeId, l: f32, t: f32, r: f32, b: f32) -> CompatResult<()> {
    modify_layout(node, |layout| {
        layout.margin = Insets::new(t, r, b, l);
    })
}

/// Sets all four padding sides.
pub fn style_padding(node: NodeId, l: f32, t: f32, r: f32, b: f32) -> CompatResult<()> {
    modify_layout(node, |layout| {
        layout.padding = Insets::new(t, r, b, l);
    })
}

/// Sets the background fill from a hex color string (`#rgb`, `#rrggbb`, or
/// `#rrggbbaa`).
pub fn style_background(node: NodeId, color: &str) -> CompatResult<()> {
    let c = parse_color(color)?;
    modify_visual(node, |s| s.background = c)
}

/// Sets the border width and color.  Color is a hex string.
pub fn style_border(node: NodeId, width: f32, color: &str) -> CompatResult<()> {
    let c = parse_color(color)?;
    modify_visual(node, |s| {
        s.border = Insets::all(width);
        s.border_color = c;
    })
}

/// Sets the node opacity, clamped to `[0.0, 1.0]`.
pub fn style_opacity(node: NodeId, alpha: f32) -> CompatResult<()> {
    let alpha = alpha.clamp(0.0, 1.0);
    modify_visual(node, |s| s.opacity = alpha)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_color_hex6() {
        let c = parse_color("#1e1e2e").unwrap();
        assert!((c.r - 0x1e as f32 / 255.0).abs() < 1e-3);
        assert!((c.g - 0x1e as f32 / 255.0).abs() < 1e-3);
        assert!((c.b - 0x2e as f32 / 255.0).abs() < 1e-3);
        assert!((c.a - 1.0).abs() < 1e-3);
    }

    #[test]
    fn parse_color_hex3() {
        let c = parse_color("#fff").unwrap();
        assert!((c.r - 1.0).abs() < 1e-3);
        assert!((c.g - 1.0).abs() < 1e-3);
        assert!((c.b - 1.0).abs() < 1e-3);
    }

    #[test]
    fn parse_color_hex8() {
        let c = parse_color("#20242a80").unwrap();
        assert!((c.a - 0x80 as f32 / 255.0).abs() < 1e-3);
    }

    #[test]
    fn parse_color_invalid() {
        assert!(parse_color("red").is_err());
        assert!(parse_color("#gg0000").is_err());
    }

    #[test]
    fn hex1_ascii_correctness() {
        // digits 0–9
        for (ch, expected) in [
            ('0', 0u8),
            ('1', 1),
            ('2', 2),
            ('3', 3),
            ('4', 4),
            ('5', 5),
            ('6', 6),
            ('7', 7),
            ('8', 8),
            ('9', 9),
        ] {
            assert_eq!(hex1(ch), expected, "char '{ch}'");
        }
        // lowercase a–f
        for (ch, expected) in [
            ('a', 10u8),
            ('b', 11),
            ('c', 12),
            ('d', 13),
            ('e', 14),
            ('f', 15),
        ] {
            assert_eq!(hex1(ch), expected, "char '{ch}'");
        }
        // uppercase A–F
        for (ch, expected) in [
            ('A', 10u8),
            ('B', 11),
            ('C', 12),
            ('D', 13),
            ('E', 14),
            ('F', 15),
        ] {
            assert_eq!(hex1(ch), expected, "char '{ch}'");
        }
        // invalid char
        assert_eq!(hex1('g'), 0, "invalid char 'g'");
        assert_eq!(hex1('Z'), 0, "invalid char 'Z'");
        assert_eq!(hex1('!'), 0, "invalid char '!'");
    }
}
