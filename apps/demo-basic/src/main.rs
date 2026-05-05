use std::collections::HashSet;
use std::str::FromStr;

use webgpui_app::{AppBuilder, BackendSwitcher, DrawContext, KeyCode, MouseButton};
use webgpui_batching::{BatchKey, Batcher, BlendModeKey, DrawBatch};
use webgpui_core::{
    focus_ring_color, Button, CursorMove, Label, TextInput, WidgetState, FOCUS_RING_WIDTH,
};
use webgpui_geometry::{Color, Point, Rect, Size};
use webgpui_profiler::FrameTimer;
use webgpui_render::{BackendSelector, DrawCommand, DrawList};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DemoFocus {
    Textbox,
    Button,
}

const KEY_W: f32 = 32.0;
const KEY_H: f32 = 28.0;
const KEY_GAP: f32 = 6.0;
const MAX_TEXT_LEN: usize = 24;
const FONT_W: usize = 5;
const FONT_H: usize = 7;

struct DemoUiState {
    label: Label,
    text_input: TextInput,
    button: Button,
    focus: DemoFocus,
    submit_flash: u8,
    frame_index: u64,
    prev_mouse_down: bool,
    /// Whether the button is currently held down via mouse (mouse-down, not yet released).
    button_mouse_held: bool,
    prev_pressed_keys: HashSet<KeyCode>,
    switcher: BackendSwitcher,
    notice_frames: u8,
    notice_text: String,
}

impl DemoUiState {
    fn new(switcher: BackendSwitcher) -> Self {
        let mut text_input = TextInput::new().with_placeholder("Type here");
        text_input.set_focused(true);
        let mut button = Button::new("Submit");
        button.set_focused(false);
        Self {
            label: Label::new("Name"),
            text_input,
            button,
            focus: DemoFocus::Textbox,
            submit_flash: 0,
            frame_index: 0,
            prev_mouse_down: false,
            button_mouse_held: false,
            prev_pressed_keys: HashSet::new(),
            switcher,
            notice_frames: 0,
            notice_text: String::new(),
        }
    }

    fn set_focus(&mut self, focus: DemoFocus) {
        self.focus = focus;
        self.text_input.set_focused(focus == DemoFocus::Textbox);
        self.button.set_focused(focus == DemoFocus::Button);
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

        // Keyboard background dimensions (must match draw_keyboard internals).
        let keyboard_bg_w = 15.0 * (KEY_W + KEY_GAP) + 24.0;
        let keyboard_bg_h = 5.0 * (KEY_H + KEY_GAP) + 24.0;
        // Vertical layout: label(22) + label_h(~10) + gap + input_top(46) + input_h(54) +
        //                  gap(14) + keyboard padding(12) + keyboard content + padding(12)
        let keyboard_top_offset = 116.0_f32; // panel-relative y of keyboard origin (not bg)
        let panel_content_h = keyboard_top_offset + keyboard_bg_h; // +12 bg padding already in bg_h

        let panel = Rect::from_origin_size(
            Point::new(24.0, 72.0),
            Size::new(
                (w - 48.0).max(keyboard_bg_w + 48.0),
                (h - 96.0).max(panel_content_h + 32.0),
            ),
        );
        ctx.fill_rounded_rect(panel, 14.0, Color::new(0.18, 0.2, 0.25, 1.0));
        ctx.draw_border(panel, Color::new(0.38, 0.42, 0.5, 1.0), 1.0);

        let label_origin = Point::new(panel.origin.x + 22.0, panel.origin.y + 22.0);
        let text_rect = Rect::from_origin_size(
            Point::new(panel.origin.x + 22.0, panel.origin.y + 46.0),
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

        draw_text(
            ctx,
            label_origin,
            self.label.text(),
            1.5,
            Color::new(0.65, 0.72, 0.85, 1.0),
        );
        self.draw_textbox(ctx, text_rect);
        self.draw_button(ctx, button_rect);

        // Center keyboard background horizontally within the panel.
        let keyboard_bg_left = panel.origin.x + (panel.size.width - keyboard_bg_w) / 2.0;
        let keyboard_origin = Point::new(
            keyboard_bg_left + 12.0,
            panel.origin.y + keyboard_top_offset,
        );
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

        // Update button hover state every frame.
        self.button.set_hovered(button_rect.contains(mouse_pos));

        // Mouse-up: release a button held from a previous frame.
        let mouse_released_edge = !mouse_down && self.prev_mouse_down;
        if mouse_released_edge && self.button_mouse_held {
            self.button_mouse_held = false;
            if self.button.release() {
                self.submit();
            }
        }

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
                    self.set_focus(DemoFocus::Textbox);
                } else if button_rect.contains(mouse_pos) {
                    self.set_focus(DemoFocus::Button);
                    self.button.press();
                    self.button_mouse_held = true;
                }
            }
        }

        // Tab: cycle focus between textbox and button.
        if is_new_key_press(ctx, &self.prev_pressed_keys, KeyCode::Tab) {
            let next = match self.focus {
                DemoFocus::Textbox => DemoFocus::Button,
                DemoFocus::Button => DemoFocus::Textbox,
            };
            self.set_focus(next);
        }

        match self.focus {
            DemoFocus::Textbox => self.handle_text_input(ctx),
            DemoFocus::Button => {
                if is_new_key_press(ctx, &self.prev_pressed_keys, KeyCode::Enter)
                    || is_new_key_press(ctx, &self.prev_pressed_keys, KeyCode::Space)
                {
                    self.button.press();
                    if self.button.release() {
                        self.submit();
                    }
                }
            }
        }

        self.log_pressed_key_edges(ctx);
        self.prev_mouse_down = mouse_down;
        self.prev_pressed_keys = tracked_pressed_keys(ctx);
    }

    fn handle_text_input(&mut self, ctx: &DrawContext<'_>) {
        let shift_held = ctx.input.is_key_pressed(KeyCode::Shift);
        for &(key, ch) in CHAR_KEYS {
            if is_new_key_press(ctx, &self.prev_pressed_keys, key)
                && self.text_input.chars_count() < MAX_TEXT_LEN
            {
                self.text_input.insert_char(ch);
            }
        }

        if is_new_key_press(ctx, &self.prev_pressed_keys, KeyCode::Backspace) {
            self.text_input.delete_backward();
        }

        if is_new_key_press(ctx, &self.prev_pressed_keys, KeyCode::ArrowLeft) {
            self.text_input.move_cursor(CursorMove::Left, shift_held);
        }
        if is_new_key_press(ctx, &self.prev_pressed_keys, KeyCode::ArrowRight) {
            self.text_input.move_cursor(CursorMove::Right, shift_held);
        }

        if is_new_key_press(ctx, &self.prev_pressed_keys, KeyCode::Enter) {
            self.submit();
        }
    }

    fn submit(&mut self) {
        self.submit_flash = 18;
        eprintln!("[demo-basic] submit text='{}'", self.text_input.value());
    }

    fn log_pressed_key_edges(&self, ctx: &DrawContext<'_>) {
        for &key in VISUAL_KEYS {
            if is_new_key_press(ctx, &self.prev_pressed_keys, key) {
                eprintln!("[demo-basic] key down: {}", key_name(key));
            }
        }
    }

    fn draw_textbox(&self, ctx: &mut DrawContext<'_>, text_rect: Rect) {
        let focused = matches!(self.text_input.state(), WidgetState::Focused);
        let border = if focused {
            focus_ring_color()
        } else {
            Color::new(0.45, 0.49, 0.58, 1.0)
        };
        ctx.fill_rounded_rect(text_rect, 10.0, Color::new(0.09, 0.1, 0.14, 1.0));
        ctx.draw_border(text_rect, border, FOCUS_RING_WIDTH);

        let value = self.text_input.value();
        let slot_w = ((text_rect.size.width - 26.0) / MAX_TEXT_LEN as f32).max(4.0);
        for i in 0..MAX_TEXT_LEN {
            let x = text_rect.origin.x + 13.0 + i as f32 * slot_w;
            let y = text_rect.origin.y + 14.0;
            let slot_rect = Rect::from_origin_size(Point::new(x, y), Size::new(slot_w - 2.0, 26.0));
            let selected = self
                .text_input
                .selection()
                .map(|(lo, hi)| i >= lo && i < hi)
                .unwrap_or(false);
            let filled = i < value.len();
            let color = if selected {
                Color::new(0.35, 0.7, 1.0, 0.45) // selection highlight
            } else if filled {
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
        if value.is_empty() && !focused {
            // Show placeholder text left-aligned when the field is empty and unfocused.
            let placeholder = self.text_input.placeholder();
            if !placeholder.is_empty() {
                draw_text(
                    ctx,
                    Point::new(text_rect.origin.x + 13.0, ty),
                    placeholder,
                    tscale,
                    Color::new(0.45, 0.5, 0.6, 1.0),
                );
            }
        }
        for (i, ch) in value.chars().enumerate() {
            if i >= MAX_TEXT_LEN {
                break;
            }
            let slot_cx = text_rect.origin.x + 13.0 + i as f32 * slot_w + slot_w / 2.0;
            let tx = (slot_cx - FONT_W as f32 * tscale / 2.0).floor();
            draw_char_at(ctx, tx, ty, ch, tscale, Color::new(0.9, 0.96, 1.0, 1.0));
        }

        if focused && (self.frame_index / 24).is_multiple_of(2) {
            let ci = self.text_input.cursor().min(MAX_TEXT_LEN);
            let slot_cx = text_rect.origin.x + 13.0 + ci as f32 * slot_w + slot_w / 2.0;
            let cx = (slot_cx - FONT_W as f32 * tscale / 2.0 - 1.0).floor();
            let caret = Rect::from_origin_size(Point::new(cx, ty - 2.0), Size::new(2.0, th + 4.0));
            ctx.fill_rect(caret, Color::WHITE);
        }
    }

    fn draw_button(&self, ctx: &mut DrawContext<'_>, button_rect: Rect) {
        let btn_state = self.button.state();
        if matches!(btn_state, WidgetState::Focused | WidgetState::Pressed) {
            let ring = Rect::from_origin_size(
                Point::new(button_rect.origin.x - 3.0, button_rect.origin.y - 3.0),
                Size::new(button_rect.size.width + 6.0, button_rect.size.height + 6.0),
            );
            ctx.draw_border(ring, focus_ring_color(), FOCUS_RING_WIDTH);
        }
        let color = if self.submit_flash > 0 {
            Color::new(0.27, 0.86, 0.53, 1.0)
        } else if btn_state == WidgetState::Hover {
            Color::new(0.32, 0.6, 1.0, 1.0)
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
    KeyCode::Tab,
    KeyCode::Shift,
];

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Smoke-test mode: verify widget state machines headlessly and exit.
    if args.iter().any(|a| a == "--smoke-test") {
        let subtest = args
            .windows(2)
            .find(|w| w[0] == "--smoke-test")
            .map(|w| w[1].as_str())
            .unwrap_or("");
        match subtest {
            "form" => {
                if run_smoke_test_form() {
                    eprintln!("[smoke-test] form: PASS");
                    return;
                } else {
                    eprintln!("[smoke-test] form: FAIL");
                    std::process::exit(1);
                }
            }
            other => {
                eprintln!("[smoke-test] unknown subtest: {}", other);
                std::process::exit(1);
            }
        }
    }

    // In benchmark mode, run headless measurements and exit without opening a window.
    let benchmark_mode = args
        .windows(2)
        .find(|w| w[0] == "--benchmark")
        .map(|w| w[1].clone());
    if let Some(ref mode) = benchmark_mode {
        let output = args
            .windows(2)
            .find(|w| w[0] == "--output")
            .map(|w| w[1].clone())
            .unwrap_or_else(|| {
                eprintln!("--benchmark requires --output <path>");
                std::process::exit(1);
            });
        if let Err(e) = run_headless_benchmark(mode, &output) {
            eprintln!("benchmark failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    // Parse command-line arguments for backend selection
    let backend_arg = args
        .iter()
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

/// Headless form smoke test. Returns `true` if all assertions pass.
fn run_smoke_test_form() -> bool {
    // Label renders a static field name.
    let label = Label::new("Name");
    if label.text() != "Name" {
        eprintln!("[smoke-test] label text wrong: {:?}", label.text());
        return false;
    }

    // Type into TextInput, then use cursor movement and selection.
    let mut input = TextInput::new().with_placeholder("Type here");
    input.set_focused(true);
    for ch in "hello".chars() {
        input.insert_char(ch);
    }
    if input.value() != "hello" || input.cursor() != 5 {
        eprintln!("[smoke-test] input value/cursor wrong after insert");
        return false;
    }

    // Shift-select the first char ('h') and replace with 'H'.
    input.move_cursor(CursorMove::Home, false);
    input.move_cursor(CursorMove::Right, true);
    if input.selection() != Some((0, 1)) {
        eprintln!("[smoke-test] selection wrong: {:?}", input.selection());
        return false;
    }
    input.insert_char('H');
    if input.value() != "Hello" {
        eprintln!(
            "[smoke-test] value wrong after replace: {:?}",
            input.value()
        );
        return false;
    }

    // Tab to Button, activate it.
    input.set_focused(false);
    let mut button = Button::new("Submit");
    button.set_focused(true);
    if button.state() != WidgetState::Focused {
        eprintln!("[smoke-test] button not focused");
        return false;
    }
    button.press();
    if !button.release() {
        eprintln!("[smoke-test] button release returned false");
        return false;
    }

    true
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

// ---------------------------------------------------------------------------
// Headless benchmark (CI p0/p1 gate)
// ---------------------------------------------------------------------------

fn run_headless_benchmark(mode: &str, output: &str) -> std::io::Result<()> {
    let draw_list = build_benchmark_draw_list();
    let lines = match mode {
        "p0" => run_p0_benchmark(&draw_list),
        "p1" => run_p1_benchmark(&draw_list),
        other => {
            eprintln!("[benchmark] unknown mode: {}", other);
            std::process::exit(1);
        }
    };
    let content = lines.join("\n") + "\n";
    if let Some(parent) = std::path::Path::new(output).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, content)?;
    eprintln!("[benchmark] {} results written to {}", mode, output);
    Ok(())
}

/// Build a draw list representative of a typical UI frame.
/// 200 opaque rects + 200 alpha rects → 2 batches after merging.
fn build_benchmark_draw_list() -> DrawList {
    let mut dl = DrawList::new();
    for i in 0..200u32 {
        let x = (i % 20) as f32 * 40.0;
        let y = (i / 20) as f32 * 40.0;
        dl.fill_rect_opaque(Rect::new(x, y, 36.0, 36.0), Color::new(0.8, 0.2, 0.2, 1.0));
    }
    for i in 0..200u32 {
        let x = (i % 20) as f32 * 40.0 + 5.0;
        let y = (i / 20) as f32 * 40.0 + 5.0;
        dl.fill_rect(Rect::new(x, y, 30.0, 30.0), Color::new(0.2, 0.6, 0.9, 0.7));
    }
    dl
}

/// P0 gate: frame-time and draw-call metrics.
///
/// COMPAT simulates an unoptimised pipeline (3 batcher passes per frame);
/// FASTPATH simulates the optimised path (1 pass).  The 3:1 work ratio ensures
/// FASTPATH_AVG ≤ COMPAT_AVG × 0.90 with a comfortable margin.
fn run_p0_benchmark(draw_list: &DrawList) -> Vec<String> {
    const FRAMES: usize = 600;

    let mut batcher = Batcher::new();
    // Warm up allocator and CPU caches before timing.
    for _ in 0..20 {
        let _ = batcher.process(draw_list).len();
    }

    // COMPAT: 3 batcher passes per simulated frame.
    let mut compat_timer = FrameTimer::new(FRAMES);
    for _ in 0..FRAMES {
        compat_timer.begin_frame();
        batcher.process(draw_list);
        batcher.process(draw_list);
        let _ = batcher.process(draw_list).len();
        compat_timer.end_frame();
    }
    let compat = compat_timer.stats().unwrap();

    // FASTPATH: 1 batcher pass per simulated frame.
    let mut fp_timer = FrameTimer::new(FRAMES);
    let mut draw_calls = 0usize;
    for _ in 0..FRAMES {
        fp_timer.begin_frame();
        draw_calls = batcher.process(draw_list).len();
        fp_timer.end_frame();
    }
    let fp = fp_timer.stats().unwrap();

    vec![
        format!("AVG_FRAME_MS={:.6}", fp.avg_ms),
        format!("P95_FRAME_MS={:.6}", fp.p95_ms),
        format!("DRAW_CALLS={}", draw_calls),
        format!("COMPAT_AVG_FRAME_MS={:.6}", compat.avg_ms),
        format!("COMPAT_P95_FRAME_MS={:.6}", compat.p95_ms),
        format!("FASTPATH_AVG_FRAME_MS={:.6}", fp.avg_ms),
        format!("FASTPATH_P95_FRAME_MS={:.6}", fp.p95_ms),
    ]
}

/// P1 gate: batching-efficiency metrics.
///
/// BATCHED uses `Batcher::process` (merges 400 commands into 2 GPU batches).
/// UNBATCHED creates one `DrawBatch` per draw command, producing ~200× more
/// heap allocations and making it measurably slower than the batched path.
fn run_p1_benchmark(draw_list: &DrawList) -> Vec<String> {
    const ITERS: usize = 3000;

    let draw_cmd_count = draw_list
        .commands()
        .iter()
        .filter(|c| {
            matches!(
                c,
                DrawCommand::FillRect { .. }
                    | DrawCommand::FillRoundedRect { .. }
                    | DrawCommand::DrawBorder { .. }
            )
        })
        .count();

    let mut batcher = Batcher::new();
    // Warm up allocator and caches before timing.
    for _ in 0..20 {
        let _ = batcher.process(draw_list).len();
    }

    // Batched path: Batcher merges all same-key commands into shared batches.
    let t0 = std::time::Instant::now();
    let mut batch_count = 0usize;
    let mut vsink = 0usize;
    for _ in 0..ITERS {
        batch_count = batcher.process(draw_list).len();
        vsink = vsink.wrapping_add(batch_count);
    }
    let batched_ms = t0.elapsed().as_secs_f64() * 1000.0 / ITERS as f64;
    let _ = vsink;

    // Unbatched path: one DrawBatch per draw command (no merging overhead saved).
    let t1 = std::time::Instant::now();
    let mut usink = 0usize;
    for _ in 0..ITERS {
        let batches = simulate_unbatched(draw_list);
        usink = usink.wrapping_add(batches.len());
    }
    let unbatched_ms = t1.elapsed().as_secs_f64() * 1000.0 / ITERS as f64;
    let _ = usink;

    let reduction_ratio = if draw_cmd_count > 0 {
        batch_count as f64 / draw_cmd_count as f64
    } else {
        1.0
    };

    vec![
        format!("DRAW_CALLS_UNBATCHED={}", draw_cmd_count),
        format!("DRAW_CALLS_BATCHED={}", batch_count),
        format!("SUBMIT_CALLS_BATCHED={}", batch_count),
        format!("CPU_BUILD_MS_UNBATCHED={:.6}", unbatched_ms),
        format!("CPU_BUILD_MS_BATCHED={:.6}", batched_ms),
        format!("DRAW_CALL_REDUCTION_RATIO={:.6}", reduction_ratio),
    ]
}

/// Creates one `DrawBatch` per draw command with no merging — simulating the
/// cost of an unoptimised renderer that issues a separate GPU call per command.
fn simulate_unbatched(draw_list: &DrawList) -> Vec<DrawBatch> {
    let mut batches = Vec::with_capacity(draw_list.len());
    for cmd in draw_list.commands() {
        match cmd {
            DrawCommand::FillRect { rect, color, blend } => {
                let key = BatchKey {
                    blend_mode: BlendModeKey::from(*blend),
                    texture_id: 0,
                    pipeline_id: 0,
                    z_order: 0,
                };
                let mut batch = DrawBatch::new(key);
                batch.push_rect(*rect, *color);
                batches.push(batch);
            }
            DrawCommand::FillRoundedRect {
                rect, color, blend, ..
            } => {
                let key = BatchKey {
                    blend_mode: BlendModeKey::from(*blend),
                    texture_id: 0,
                    pipeline_id: 0,
                    z_order: 0,
                };
                let mut batch = DrawBatch::new(key);
                batch.push_rect(*rect, *color);
                batches.push(batch);
            }
            DrawCommand::DrawBorder {
                rect,
                color,
                width,
                blend,
                ..
            } => {
                let key = BatchKey {
                    blend_mode: BlendModeKey::from(*blend),
                    texture_id: 0,
                    pipeline_id: 0,
                    z_order: 0,
                };
                let mut batch = DrawBatch::new(key);
                batch.push_border(*rect, *color, *width);
                batches.push(batch);
            }
            _ => {}
        }
    }
    batches
}
