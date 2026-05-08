//! Dev-tools overlay renderers (perf, inspector, dirty-rect).
//!
//! All functions in this module draw directly into a [`DrawList`] using the
//! existing primitive draw commands — no extra GPU resources required.
//! The entire module is compiled out when the `dev-tools` feature is disabled.

use webgpui_core::{DirtyTracker, Node, NodeKind, NodeRole};
use webgpui_geometry::{Color, Rect, Size};
use webgpui_profiler::FrameStats;
use webgpui_render::DrawList;

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Logical pixels per bitmap "pixel" cell.
const SCALE: f32 = 2.0;
/// Advance width per character (3 cols × SCALE + 1 gap column).
const CHAR_ADV: f32 = 3.0 * SCALE + SCALE;
/// Height of one character cell (5 rows × SCALE).
const CHAR_H: f32 = 5.0 * SCALE;
/// Vertical advance per text line (char height + gap row).
const ROW_ADV: f32 = CHAR_H + SCALE;
/// Padding inside overlay panel boxes.
const PAD: f32 = 4.0;

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const PANEL_BG: Color = Color {
    r: 0.05,
    g: 0.05,
    b: 0.05,
    a: 0.88,
};
const TEXT_FG: Color = Color {
    r: 0.95,
    g: 0.95,
    b: 0.95,
    a: 1.0,
};
const DIRTY_TINT: Color = Color {
    r: 1.0,
    g: 0.15,
    b: 0.0,
    a: 0.22,
};

// ---------------------------------------------------------------------------
// 3 × 5 bitmap font (uppercase + digits + symbols)
// ---------------------------------------------------------------------------

/// Returns the 5-row bitmap for a character, or `None` if unsupported.
///
/// Each `u8` in the array encodes one row: bit 2 = left column, bit 1 = middle,
/// bit 0 = right column.  Only the lower 3 bits are used.
fn char_pixels(c: char) -> Option<[u8; 5]> {
    Some(match c.to_ascii_uppercase() {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        ' ' => [0b000, 0b000, 0b000, 0b000, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        ',' => [0b000, 0b000, 0b000, 0b010, 0b100],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '_' => [0b000, 0b000, 0b000, 0b000, 0b111],
        '=' => [0b000, 0b111, 0b000, 0b111, 0b000],
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b111, 0b100, 0b100, 0b100, 0b111],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b111, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b111, 0b100, 0b100],
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b011, 0b001, 0b001, 0b101, 0b011],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b101, 0b101, 0b101],
        'N' => [0b101, 0b111, 0b101, 0b101, 0b101],
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'Q' => [0b011, 0b101, 0b101, 0b111, 0b001],
        'R' => [0b111, 0b101, 0b111, 0b110, 0b101],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b010, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        _ => return None,
    })
}

/// Draws a string at (`x`, `y`) using the 3×5 bitmap font scaled by `SCALE`.
fn draw_text(list: &mut DrawList, x: f32, y: f32, text: &str, color: Color) {
    let mut cx = x;
    for ch in text.chars() {
        if let Some(rows) = char_pixels(ch) {
            for (row, &bits) in rows.iter().enumerate() {
                for col in 0u8..3 {
                    if bits & (0b100 >> col) != 0 {
                        list.fill_rect(
                            Rect::new(
                                cx + col as f32 * SCALE,
                                y + row as f32 * SCALE,
                                SCALE,
                                SCALE,
                            ),
                            color,
                        );
                    }
                }
            }
        }
        cx += CHAR_ADV;
    }
}

/// Returns the rendered width of `text` in logical pixels.
fn text_width(text: &str) -> f32 {
    text.chars().count() as f32 * CHAR_ADV
}

/// Draws a semi-transparent panel background.
fn draw_panel(list: &mut DrawList, x: f32, y: f32, w: f32, h: f32) {
    list.fill_rect(Rect::new(x, y, w, h), PANEL_BG);
}

// ---------------------------------------------------------------------------
// Public overlay entry points
// ---------------------------------------------------------------------------

/// Draws the performance overlay in the top-left corner.
///
/// Shows FPS (derived from avg frame time), avg and p95 frame times in ms,
/// and the number of user draw commands recorded before this overlay ran.
pub(crate) fn draw_perf_overlay(
    list: &mut DrawList,
    stats: &FrameStats,
    draw_calls: usize,
    _viewport: Size,
) {
    let fps = if stats.avg_ms > 0.0 {
        (1000.0 / stats.avg_ms).round() as u32
    } else {
        0
    };
    let lines: &[String] = &[
        format!("FPS {}", fps),
        format!("AVG {:.1}MS", stats.avg_ms),
        format!("P95 {:.1}MS", stats.p95_ms),
        format!("DC  {}", draw_calls),
    ];
    render_panel_lines(list, PAD, PAD, lines);
}

/// Draws the node inspector overlay in the top-right corner.
///
/// Shows the inspected node's id, kind, role, and key computed style
/// properties.  Call [`DrawContext::dev_inspect`] each frame to set which
/// node is displayed.
pub(crate) fn draw_inspector_overlay(list: &mut DrawList, node: &Node, viewport: Size) {
    let bg = &node.style.background;
    let lines: &[String] = &[
        format!("ID {}", node.id.0),
        format!("KIND {}", kind_str(node.kind)),
        format!("ROLE {}", role_str(node.role)),
        format!("OPACITY {:.2}", node.style.opacity),
        format!("VISIBLE {}", if node.style.visible { "YES" } else { "NO" }),
        format!("TX {:.1}", node.style.translate_x),
        format!("TY {:.1}", node.style.translate_y),
        format!(
            "BG {},{},{}",
            (bg.r * 255.0).round() as u8,
            (bg.g * 255.0).round() as u8,
            (bg.b * 255.0).round() as u8,
        ),
    ];
    let max_w = lines.iter().map(|l| text_width(l)).fold(0.0_f32, f32::max);
    let panel_w = max_w + PAD * 2.0;
    let px = viewport.width - panel_w - PAD;
    render_panel_lines(list, px, PAD, lines);
}

/// Draws a translucent red tint over the current dirty region.
pub(crate) fn draw_dirty_rects_overlay(list: &mut DrawList, dirty: &DirtyTracker, viewport: Size) {
    if let Some(rect) = dirty.effective_area(viewport) {
        list.fill_rect(rect, DIRTY_TINT);
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Draws a dark panel containing `lines` of text starting at (`px`, `py`).
fn render_panel_lines(list: &mut DrawList, px: f32, py: f32, lines: &[String]) {
    let max_w = lines.iter().map(|l| text_width(l)).fold(0.0_f32, f32::max);
    let panel_w = max_w + PAD * 2.0;
    let panel_h = ROW_ADV * lines.len() as f32 + PAD * 2.0 - SCALE;
    draw_panel(list, px, py, panel_w, panel_h);
    for (i, line) in lines.iter().enumerate() {
        let tx = px + PAD;
        let ty = py + PAD + i as f32 * ROW_ADV;
        draw_text(list, tx, ty, line, TEXT_FG);
    }
}

fn kind_str(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Container => "CONTAINER",
        NodeKind::Text => "TEXT",
        NodeKind::Image => "IMAGE",
    }
}

fn role_str(r: NodeRole) -> &'static str {
    match r {
        NodeRole::None => "NONE",
        NodeRole::Button => "BUTTON",
        NodeRole::TextBox => "TEXTBOX",
        NodeRole::Tab => "TAB",
        NodeRole::Dialog => "DIALOG",
        NodeRole::Menu => "MENU",
    }
}
