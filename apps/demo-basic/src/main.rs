use std::collections::HashSet;
use std::str::FromStr;

use webgpui_app::{AppBuilder, BackendSwitcher, DrawContext, KeyCode, MouseButton};
use webgpui_geometry::{Color, Point, Rect, Size};
use webgpui_render::BackendSelector;

const KEY_W: f32 = 32.0;
const KEY_H: f32 = 28.0;
const KEY_GAP: f32 = 6.0;
const MAX_TEXT_LEN: usize = 24;
const FONT_W: usize = 5;
const FONT_H: usize = 7;

struct DemoUiState {
    text_value: String,
    text_focused: bool,
    submit_flash: u8,
    frame_index: u64,
    prev_mouse_down: bool,
    prev_pressed_keys: HashSet<KeyCode>,
    switcher: BackendSwitcher,
    notice_frames: u8,
    notice_text: String,
}

impl DemoUiState {
    fn new(switcher: BackendSwitcher) -> Self {
        Self {
            text_value: String::new(),
            text_focused: true,
            submit_flash: 0,
            frame_index: 0,
            prev_mouse_down: false,
            prev_pressed_keys: HashSet::new(),
            switcher,
            notice_frames: 0,
            notice_text: String::new(),
        }
    }

    fn draw_frame(&mut self, ctx: &mut DrawContext<'_>) {
        self.frame_index = self.frame_index.wrapping_add(1);
        let w = ctx.viewport.width;
        let h = ctx.viewport.height;

        ctx.fill_background(Color::new(0.11, 0.12, 0.15, 1.0));
        ctx.fill_rect(
            Rect::from_origin_size(Point::ZERO, Size::new(w, 52.0)),
            Color::new(0.16, 0.18, 0.22, 1.0),
        );
        draw_text(
            ctx,
            Point::new(18.0, 18.0),
            "WEBGPUI DEMO: KEYBOARD + TEXTBOX + BUTTON",
            2.0,
            Color::new(0.9, 0.95, 1.0, 1.0),
        );

        // Backend selector buttons in header (right side)
        let backend_buttons = backend_button_rects(w);
        self.draw_backend_buttons(ctx, &backend_buttons);

        // Notice toast
        if self.notice_frames > 0 {
            self.notice_frames -= 1;
            let notice_alpha = (self.notice_frames as f32 / 90.0).min(1.0);
            let msg = self.notice_text.clone();
            draw_notice(ctx, w, h, &msg, notice_alpha);
        }

        let panel = Rect::from_origin_size(
            Point::new(24.0, 72.0),
            Size::new((w - 48.0).max(300.0), (h - 96.0).max(420.0)),
        );
        ctx.fill_rounded_rect(panel, 14.0, Color::new(0.18, 0.2, 0.25, 1.0));
        ctx.draw_border(panel, Color::new(0.38, 0.42, 0.5, 1.0), 1.0);

        let text_rect = Rect::from_origin_size(
            Point::new(panel.origin.x + 22.0, panel.origin.y + 22.0),
            Size::new((panel.size.width - 220.0).max(220.0), 54.0),
        );
        let button_rect = Rect::from_origin_size(
            Point::new(
                text_rect.origin.x + text_rect.size.width + 14.0,
                text_rect.origin.y,
            ),
            Size::new(160.0, 54.0),
        );

        self.handle_interaction(ctx, text_rect, button_rect);

        self.draw_textbox(ctx, text_rect);
        self.draw_button(ctx, button_rect);

        let keyboard_origin = Point::new(panel.origin.x + 22.0, panel.origin.y + 102.0);
        self.draw_keyboard(ctx, keyboard_origin);

        if self.submit_flash > 0 {
            self.submit_flash -= 1;
        }
    }

    fn handle_interaction(&mut self, ctx: &DrawContext<'_>, text_rect: Rect, button_rect: Rect) {
        let mouse_pos = ctx.input.cursor_position;
        let mouse_down = ctx.input.is_button_pressed(MouseButton::Left);
        let mouse_pressed_edge = mouse_down && !self.prev_mouse_down;
        let w = ctx.viewport.width;

        if mouse_pressed_edge {
            // Check backend buttons first
            let backend_buttons = backend_button_rects(w);
            let mut handled = false;
            for (backend, rect) in &backend_buttons {
                if rect.contains(mouse_pos) {
                    handled = true;
                    if *backend == ctx.current_backend {
                        // already active — do nothing
                    } else if backend.is_available() {
                        eprintln!("[demo-basic] switching to {} backend", backend.name());
                        self.switcher.switch_to(*backend);
                    } else {
                        let msg = format!(
                            "{} not compiled in (add --features backend-{})",
                            backend.name(),
                            backend.name().to_lowercase()
                        );
                        eprintln!("[demo-basic] {}", msg);
                        self.notice_text = msg;
                        self.notice_frames = 180;
                    }
                    break;
                }
            }
            if !handled {
                if text_rect.contains(mouse_pos) {
                    self.text_focused = true;
                } else if button_rect.contains(mouse_pos) {
                    self.submit();
                } else {
                    self.text_focused = false;
                }
            }
        }

        if self.text_focused {
            self.handle_text_input(ctx);
        }

        self.log_pressed_key_edges(ctx);
        self.prev_mouse_down = mouse_down;
        self.prev_pressed_keys = tracked_pressed_keys(ctx);
    }

    fn handle_text_input(&mut self, ctx: &DrawContext<'_>) {
        for &(key, ch) in CHAR_KEYS {
            if is_new_key_press(ctx, &self.prev_pressed_keys, key)
                && self.text_value.len() < MAX_TEXT_LEN
            {
                self.text_value.push(ch);
            }
        }

        if is_new_key_press(ctx, &self.prev_pressed_keys, KeyCode::Backspace) {
            self.text_value.pop();
        }

        if is_new_key_press(ctx, &self.prev_pressed_keys, KeyCode::Enter) {
            self.submit();
        }
    }

    fn submit(&mut self) {
        self.submit_flash = 18;
        eprintln!("[demo-basic] submit text='{}'", self.text_value);
    }

    fn log_pressed_key_edges(&self, ctx: &DrawContext<'_>) {
        for &key in VISUAL_KEYS {
            if is_new_key_press(ctx, &self.prev_pressed_keys, key) {
                eprintln!("[demo-basic] key down: {}", key_name(key));
            }
        }
    }

    fn draw_textbox(&self, ctx: &mut DrawContext<'_>, text_rect: Rect) {
        let border = if self.text_focused {
            Color::new(0.35, 0.7, 1.0, 1.0)
        } else {
            Color::new(0.45, 0.49, 0.58, 1.0)
        };
        ctx.fill_rounded_rect(text_rect, 10.0, Color::new(0.09, 0.1, 0.14, 1.0));
        ctx.draw_border(text_rect, border, 2.0);

        let slot_w = ((text_rect.size.width - 26.0) / MAX_TEXT_LEN as f32).max(4.0);
        for i in 0..MAX_TEXT_LEN {
            let x = text_rect.origin.x + 13.0 + i as f32 * slot_w;
            let y = text_rect.origin.y + 14.0;
            let slot_rect = Rect::from_origin_size(Point::new(x, y), Size::new(slot_w - 2.0, 26.0));
            let filled = i < self.text_value.len();
            let color = if filled {
                Color::new(0.33, 0.84, 1.0, 0.2)
            } else {
                Color::new(0.24, 0.28, 0.35, 1.0)
            };
            ctx.fill_rect(slot_rect, color);
        }

        // Draw each character centered in its slot.
        let tscale = 2.0_f32;
        let th = text_pixel_height(tscale);
        let ty = (text_rect.origin.y + (text_rect.size.height - th) / 2.0).floor();
        for (i, ch) in self.text_value.chars().enumerate() {
            if i >= MAX_TEXT_LEN {
                break;
            }
            let slot_cx = text_rect.origin.x + 13.0 + i as f32 * slot_w + slot_w / 2.0;
            let tx = (slot_cx - FONT_W as f32 * tscale / 2.0).floor();
            draw_char_at(ctx, tx, ty, ch, tscale, Color::new(0.9, 0.96, 1.0, 1.0));
        }

        if self.text_focused && (self.frame_index / 24).is_multiple_of(2) {
            let ci = self.text_value.len().min(MAX_TEXT_LEN);
            // Place caret at the left edge of the next slot.
            let slot_cx = text_rect.origin.x + 13.0 + ci as f32 * slot_w + slot_w / 2.0;
            let cx = (slot_cx - FONT_W as f32 * tscale / 2.0 - 1.0).floor();
            let caret = Rect::from_origin_size(Point::new(cx, ty - 2.0), Size::new(2.0, th + 4.0));
            ctx.fill_rect(caret, Color::WHITE);
        }
    }

    fn draw_button(&self, ctx: &mut DrawContext<'_>, button_rect: Rect) {
        let color = if self.submit_flash > 0 {
            Color::new(0.27, 0.86, 0.53, 1.0)
        } else {
            Color::new(0.24, 0.5, 0.9, 1.0)
        };
        ctx.fill_rounded_rect(button_rect, 10.0, color);
        ctx.draw_border(button_rect, Color::new(0.8, 0.9, 1.0, 1.0), 1.0);

        let label = "SUBMIT";
        let bscale = 2.0_f32;
        let tw = text_pixel_width(label, bscale);
        let th = text_pixel_height(bscale);
        let tx = (button_rect.origin.x + (button_rect.size.width - tw) / 2.0).floor();
        let ty = (button_rect.origin.y + (button_rect.size.height - th) / 2.0).floor();
        draw_text(
            ctx,
            Point::new(tx, ty),
            label,
            bscale,
            Color::new(0.95, 0.98, 1.0, 1.0),
        );
    }

    fn draw_backend_buttons(&self, ctx: &mut DrawContext<'_>, buttons: &[(BackendSelector, Rect)]) {
        for (backend, rect) in buttons {
            let is_active = *backend == ctx.current_backend;
            let is_available = backend.is_available();

            let fill = if is_active {
                Color::new(0.25, 0.55, 0.95, 1.0)
            } else if is_available {
                Color::new(0.2, 0.23, 0.3, 1.0)
            } else {
                Color::new(0.14, 0.15, 0.18, 1.0)
            };
            let border = if is_active {
                Color::new(0.55, 0.78, 1.0, 1.0)
            } else if is_available {
                Color::new(0.38, 0.42, 0.5, 1.0)
            } else {
                Color::new(0.26, 0.28, 0.34, 1.0)
            };
            let text_color = if is_available {
                Color::new(0.9, 0.95, 1.0, 1.0)
            } else {
                Color::new(0.4, 0.43, 0.5, 1.0)
            };

            ctx.fill_rounded_rect(*rect, 7.0, fill);
            ctx.draw_border(*rect, border, 1.5);

            let label = backend.name();
            let scale = 1.5_f32;
            let tw = text_pixel_width(label, scale);
            let th = text_pixel_height(scale);
            let tx = (rect.origin.x + (rect.size.width - tw) / 2.0).floor();
            let ty = (rect.origin.y + (rect.size.height - th) / 2.0).floor();
            draw_text(ctx, Point::new(tx, ty), label, scale, text_color);
        }
    }

    fn draw_keyboard(&self, ctx: &mut DrawContext<'_>, origin: Point) {
        let keyboard_bg = Rect::from_origin_size(
            Point::new(origin.x - 12.0, origin.y - 12.0),
            Size::new(
                15.0 * (KEY_W + KEY_GAP) + 24.0,
                5.0 * (KEY_H + KEY_GAP) + 24.0,
            ),
        );
        ctx.fill_rounded_rect(keyboard_bg, 12.0, Color::new(0.13, 0.14, 0.18, 1.0));
        ctx.draw_border(keyboard_bg, Color::new(0.32, 0.35, 0.42, 1.0), 1.0);

        draw_key_row(ctx, origin, ROW1, &self.prev_pressed_keys);
        draw_key_row(
            ctx,
            Point::new(
                origin.x + 0.5 * (KEY_W + KEY_GAP),
                origin.y + KEY_H + KEY_GAP,
            ),
            ROW2,
            &self.prev_pressed_keys,
        );
        draw_key_row(
            ctx,
            Point::new(
                origin.x + 0.9 * (KEY_W + KEY_GAP),
                origin.y + 2.0 * (KEY_H + KEY_GAP),
            ),
            ROW3,
            &self.prev_pressed_keys,
        );
        draw_key_row(
            ctx,
            Point::new(
                origin.x + 1.2 * (KEY_W + KEY_GAP),
                origin.y + 3.0 * (KEY_H + KEY_GAP),
            ),
            ROW4,
            &self.prev_pressed_keys,
        );
        draw_key_row(
            ctx,
            Point::new(
                origin.x + 3.0 * (KEY_W + KEY_GAP),
                origin.y + 4.0 * (KEY_H + KEY_GAP),
            ),
            ROW5,
            &self.prev_pressed_keys,
        );
    }
}

#[derive(Clone, Copy)]
struct KeyVisual {
    key: KeyCode,
    width_units: f32,
}

const ROW1: &[KeyVisual] = &[
    KeyVisual {
        key: KeyCode::Digit1,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Digit2,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Digit3,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Digit4,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Digit5,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Digit6,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Digit7,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Digit8,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Digit9,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Digit0,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Backspace,
        width_units: 2.0,
    },
];

const ROW2: &[KeyVisual] = &[
    KeyVisual {
        key: KeyCode::Q,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::W,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::E,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::R,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::T,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Y,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::U,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::I,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::O,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::P,
        width_units: 1.0,
    },
];

const ROW3: &[KeyVisual] = &[
    KeyVisual {
        key: KeyCode::A,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::S,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::D,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::F,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::G,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::H,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::J,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::K,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::L,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Enter,
        width_units: 2.2,
    },
];

const ROW4: &[KeyVisual] = &[
    KeyVisual {
        key: KeyCode::Z,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::X,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::C,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::V,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::B,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::N,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::M,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::Space,
        width_units: 4.6,
    },
];

const ROW5: &[KeyVisual] = &[
    KeyVisual {
        key: KeyCode::ArrowLeft,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::ArrowDown,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::ArrowUp,
        width_units: 1.0,
    },
    KeyVisual {
        key: KeyCode::ArrowRight,
        width_units: 1.0,
    },
];

const CHAR_KEYS: &[(KeyCode, char)] = &[
    (KeyCode::A, 'a'),
    (KeyCode::B, 'b'),
    (KeyCode::C, 'c'),
    (KeyCode::D, 'd'),
    (KeyCode::E, 'e'),
    (KeyCode::F, 'f'),
    (KeyCode::G, 'g'),
    (KeyCode::H, 'h'),
    (KeyCode::I, 'i'),
    (KeyCode::J, 'j'),
    (KeyCode::K, 'k'),
    (KeyCode::L, 'l'),
    (KeyCode::M, 'm'),
    (KeyCode::N, 'n'),
    (KeyCode::O, 'o'),
    (KeyCode::P, 'p'),
    (KeyCode::Q, 'q'),
    (KeyCode::R, 'r'),
    (KeyCode::S, 's'),
    (KeyCode::T, 't'),
    (KeyCode::U, 'u'),
    (KeyCode::V, 'v'),
    (KeyCode::W, 'w'),
    (KeyCode::X, 'x'),
    (KeyCode::Y, 'y'),
    (KeyCode::Z, 'z'),
    (KeyCode::Digit0, '0'),
    (KeyCode::Digit1, '1'),
    (KeyCode::Digit2, '2'),
    (KeyCode::Digit3, '3'),
    (KeyCode::Digit4, '4'),
    (KeyCode::Digit5, '5'),
    (KeyCode::Digit6, '6'),
    (KeyCode::Digit7, '7'),
    (KeyCode::Digit8, '8'),
    (KeyCode::Digit9, '9'),
    (KeyCode::Space, ' '),
];

const VISUAL_KEYS: &[KeyCode] = &[
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::A,
    KeyCode::B,
    KeyCode::C,
    KeyCode::D,
    KeyCode::E,
    KeyCode::F,
    KeyCode::G,
    KeyCode::H,
    KeyCode::I,
    KeyCode::J,
    KeyCode::K,
    KeyCode::L,
    KeyCode::M,
    KeyCode::N,
    KeyCode::O,
    KeyCode::P,
    KeyCode::Q,
    KeyCode::R,
    KeyCode::S,
    KeyCode::T,
    KeyCode::U,
    KeyCode::V,
    KeyCode::W,
    KeyCode::X,
    KeyCode::Y,
    KeyCode::Z,
    KeyCode::Backspace,
    KeyCode::Enter,
    KeyCode::Space,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
];

fn main() {
    // Parse command-line arguments for backend selection
    let backend_arg = std::env::args()
        .find(|arg| arg.starts_with("--backend="))
        .map(|arg| arg[10..].to_string());

    let selected_backend = match backend_arg {
        Some(backend_name) => match BackendSelector::from_str(&backend_name) {
            Ok(backend) => {
                if backend.is_available() {
                    backend
                } else {
                    eprintln!(
                        "Error: backend '{}' requested but not compiled in",
                        backend.name()
                    );
                    print_backend_usage();
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("Error parsing backend: {}", e);
                print_backend_usage();
                std::process::exit(1);
            }
        },
        None => {
            // Default: try to find first available backend (prefer wgpu)
            BackendSelector::available()
                .first()
                .copied()
                .unwrap_or(BackendSelector::Wgpu)
        }
    };

    eprintln!(
        "[demo-basic] Using backend: {} (available: {})",
        selected_backend.name(),
        BackendSelector::available()
            .iter()
            .map(|b| b.name())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let switcher = BackendSwitcher::new(selected_backend);
    let mut state = DemoUiState::new(switcher.clone());

    let result = AppBuilder::new()
        .title(format!(
            "webgpui – demo-basic [{}]",
            selected_backend.name()
        ))
        .size(800, 600)
        .target_fps(Some(60))
        .background(Color::new(0.12, 0.12, 0.14, 1.0))
        .backend_switcher(switcher)
        .build()
        .run(move |ctx| state.draw_frame(ctx));

    if let Err(err) = result {
        eprintln!("demo-basic failed: {err}");
        std::process::exit(1);
    }
}

/// Print usage information for backend selection
fn print_backend_usage() {
    eprintln!();
    eprintln!("Backend Usage:");
    eprintln!("  cargo run --bin demo-basic -- --backend=<backend>");
    eprintln!();
    eprintln!("Available backends:");
    for backend in BackendSelector::available() {
        eprintln!("  - {} ({})", backend.name().to_lowercase(), backend.name());
    }
    eprintln!();
    eprintln!("Example:");
    eprintln!("  cargo run --bin demo-basic -- --backend=wgpu");
    eprintln!("  cargo run --features backend-cuda --bin demo-basic -- --backend=cuda");
    eprintln!("  cargo run --features backend-cpu --bin demo-basic -- --backend=cpu");
    eprintln!();
}

/// Returns `(BackendSelector, Rect)` pairs for the backend buttons in the header.
/// Laid out right-aligned, vertically centred in the 52 px header bar.
fn backend_button_rects(viewport_w: f32) -> Vec<(BackendSelector, Rect)> {
    let all = [
        BackendSelector::Wgpu,
        BackendSelector::Cuda,
        BackendSelector::Cpu,
    ];
    let btn_w = 64.0_f32;
    let btn_h = 32.0_f32;
    let gap = 8.0_f32;
    let right_margin = 14.0_f32;
    let header_h = 52.0_f32;
    let top = ((header_h - btn_h) / 2.0).floor();

    let total_w = all.len() as f32 * btn_w + (all.len() as f32 - 1.0) * gap;
    let start_x = (viewport_w - right_margin - total_w).max(0.0);

    all.iter()
        .enumerate()
        .map(|(i, &b)| {
            let x = start_x + i as f32 * (btn_w + gap);
            let rect = Rect::from_origin_size(Point::new(x, top), Size::new(btn_w, btn_h));
            (b, rect)
        })
        .collect()
}

/// Draw a semi-transparent toast notification near the bottom of the screen.
fn draw_notice(ctx: &mut DrawContext<'_>, w: f32, h: f32, text: &str, alpha: f32) {
    let scale = 1.5_f32;
    let tw = text_pixel_width(text, scale);
    let th = text_pixel_height(scale);
    let pad_x = 18.0_f32;
    let pad_y = 10.0_f32;
    let box_w = tw + pad_x * 2.0;
    let box_h = th + pad_y * 2.0;
    let bx = ((w - box_w) / 2.0).max(8.0);
    let by = h - box_h - 20.0;
    let bg = Color::new(0.1, 0.12, 0.18, 0.88 * alpha);
    let border = Color::new(0.38, 0.65, 1.0, alpha);
    let tc = Color::new(0.85, 0.92, 1.0, alpha);
    let notice_rect = Rect::from_origin_size(Point::new(bx, by), Size::new(box_w, box_h));
    ctx.fill_rounded_rect(notice_rect, 8.0, bg);
    ctx.draw_border(notice_rect, border, 1.5);
    draw_text(ctx, Point::new(bx + pad_x, by + pad_y), text, scale, tc);
}

fn draw_key_row(
    ctx: &mut DrawContext<'_>,
    origin: Point,
    row: &[KeyVisual],
    pressed_keys: &HashSet<KeyCode>,
) {
    let mut x = origin.x;
    for key in row {
        let width = KEY_W * key.width_units + KEY_GAP * (key.width_units - 1.0).max(0.0);
        let rect = Rect::from_origin_size(Point::new(x, origin.y), Size::new(width, KEY_H));
        let pressed = pressed_keys.contains(&key.key);

        let fill = if pressed {
            Color::new(0.3, 0.78, 1.0, 1.0)
        } else {
            Color::new(0.24, 0.27, 0.34, 1.0)
        };
        let border = if pressed {
            Color::new(0.85, 0.96, 1.0, 1.0)
        } else {
            Color::new(0.44, 0.48, 0.58, 1.0)
        };

        ctx.fill_rounded_rect(rect, 6.0, fill);
        ctx.draw_border(rect, border, 1.0);

        let label = key_label(key.key);
        let tw = text_pixel_width(label, 1.0);
        let th = text_pixel_height(1.0);
        let tx = (rect.origin.x + (rect.size.width - tw) / 2.0).max(rect.origin.x + 2.0);
        let ty = (rect.origin.y + (rect.size.height - th) / 2.0).floor();
        draw_text(
            ctx,
            Point::new(tx, ty),
            label,
            1.0,
            Color::new(0.96, 0.98, 1.0, 1.0),
        );

        x += width + KEY_GAP;
    }
}

fn is_new_key_press(ctx: &DrawContext<'_>, previous: &HashSet<KeyCode>, key: KeyCode) -> bool {
    ctx.input.is_key_pressed(key) && !previous.contains(&key)
}

fn tracked_pressed_keys(ctx: &DrawContext<'_>) -> HashSet<KeyCode> {
    let mut set = HashSet::new();
    for &key in VISUAL_KEYS {
        if ctx.input.is_key_pressed(key) {
            set.insert(key);
        }
    }
    set
}

fn key_name(key: KeyCode) -> &'static str {
    match key {
        KeyCode::A => "A",
        KeyCode::B => "B",
        KeyCode::C => "C",
        KeyCode::D => "D",
        KeyCode::E => "E",
        KeyCode::F => "F",
        KeyCode::G => "G",
        KeyCode::H => "H",
        KeyCode::I => "I",
        KeyCode::J => "J",
        KeyCode::K => "K",
        KeyCode::L => "L",
        KeyCode::M => "M",
        KeyCode::N => "N",
        KeyCode::O => "O",
        KeyCode::P => "P",
        KeyCode::Q => "Q",
        KeyCode::R => "R",
        KeyCode::S => "S",
        KeyCode::T => "T",
        KeyCode::U => "U",
        KeyCode::V => "V",
        KeyCode::W => "W",
        KeyCode::X => "X",
        KeyCode::Y => "Y",
        KeyCode::Z => "Z",
        KeyCode::Digit0 => "0",
        KeyCode::Digit1 => "1",
        KeyCode::Digit2 => "2",
        KeyCode::Digit3 => "3",
        KeyCode::Digit4 => "4",
        KeyCode::Digit5 => "5",
        KeyCode::Digit6 => "6",
        KeyCode::Digit7 => "7",
        KeyCode::Digit8 => "8",
        KeyCode::Digit9 => "9",
        KeyCode::Backspace => "Backspace",
        KeyCode::Enter => "Enter",
        KeyCode::Space => "Space",
        KeyCode::ArrowLeft => "ArrowLeft",
        KeyCode::ArrowRight => "ArrowRight",
        KeyCode::ArrowUp => "ArrowUp",
        KeyCode::ArrowDown => "ArrowDown",
        _ => "Other",
    }
}

fn key_label(key: KeyCode) -> &'static str {
    match key {
        KeyCode::Backspace => "BSP",
        KeyCode::Enter => "ENT",
        KeyCode::Space => "SPACE",
        KeyCode::ArrowLeft => "<",
        KeyCode::ArrowRight => ">",
        KeyCode::ArrowUp => "^",
        KeyCode::ArrowDown => "V",
        _ => key_name(key),
    }
}

/// Pixel width of `text` rendered at `scale` (no trailing advance after last glyph).
fn text_pixel_width(text: &str, scale: f32) -> f32 {
    let n = text.chars().count();
    if n == 0 {
        return 0.0;
    }
    (FONT_W as f32 * n as f32 + (n as f32 - 1.0)) * scale
}

/// Pixel height of a single line of text at `scale`.
fn text_pixel_height(scale: f32) -> f32 {
    FONT_H as f32 * scale
}

/// Draw a single character at pixel position `(x, y)`.
fn draw_char_at(ctx: &mut DrawContext<'_>, x: f32, y: f32, ch: char, scale: f32, color: Color) {
    let rows = glyph_rows(ch.to_ascii_uppercase());
    for (ry, bits) in rows.iter().enumerate() {
        for rx in 0..FONT_W {
            if (bits >> (FONT_W - 1 - rx)) & 1 == 1 {
                let px = x + rx as f32 * scale;
                let py = y + ry as f32 * scale;
                ctx.fill_rect(
                    Rect::from_origin_size(Point::new(px, py), Size::new(scale, scale)),
                    color,
                );
            }
        }
    }
}

fn draw_text(ctx: &mut DrawContext<'_>, origin: Point, text: &str, scale: f32, color: Color) {
    let mut x = origin.x;
    let upper = text.to_ascii_uppercase();
    for ch in upper.chars() {
        let rows = glyph_rows(ch);
        for (ry, bits) in rows.iter().enumerate() {
            for rx in 0..FONT_W {
                if (bits >> (FONT_W - 1 - rx)) & 1 == 1 {
                    let px = x + rx as f32 * scale;
                    let py = origin.y + ry as f32 * scale;
                    ctx.fill_rect(
                        Rect::from_origin_size(Point::new(px, py), Size::new(scale, scale)),
                        color,
                    );
                }
            }
        }
        x += (FONT_W as f32 + 1.0) * scale;
    }
}

fn glyph_rows(ch: char) -> [u8; FONT_H] {
    match ch {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10011, 0b10101, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        ':' => [
            0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '<' => [
            0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010,
        ],
        '>' => [
            0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000,
        ],
        '^' => [
            0b00100, 0b01010, 0b10001, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        ' ' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        _ => [
            0b11111, 0b10001, 0b00100, 0b00100, 0b00100, 0b10001, 0b11111,
        ],
    }
}
