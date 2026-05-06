//! winit-based implementation of the webgpui platform abstraction.

use std::sync::Arc;

use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{
        ElementState, Event, KeyEvent, MouseButton as WinitMouseButton, MouseScrollDelta,
        WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowBuilder},
};

use webgpui_geometry::{Point, Size};
use webgpui_input::{InputEvent, KeyCode, Modifiers, MouseButton};
use webgpui_platform::{
    EventHandler, Platform, PlatformError, PlatformEvent, PlatformResult, WindowConfig,
    WindowHandle,
};

// ---------------------------------------------------------------------------
// WinitWindowHandle
// ---------------------------------------------------------------------------

pub struct WinitWindowHandle {
    window: Arc<Window>,
}

impl std::fmt::Debug for WinitWindowHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WinitWindowHandle")
    }
}

impl WindowHandle for WinitWindowHandle {
    fn physical_size(&self) -> Size {
        let ps: PhysicalSize<u32> = self.window.inner_size();
        Size::new(ps.width as f32, ps.height as f32)
    }

    fn scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn title(&self) -> &str {
        // winit does not expose a getter; return a placeholder.
        "webgpui"
    }
}

impl WinitWindowHandle {
    /// Returns a clone of the underlying `Arc<Window>` for use by wgpu.
    pub fn window(&self) -> Arc<Window> {
        Arc::clone(&self.window)
    }
}

// ---------------------------------------------------------------------------
// WinitPlatform
// ---------------------------------------------------------------------------

/// The winit-based platform backend.
///
/// Use [`WinitPlatform::run`] to start the event loop.
pub struct WinitPlatform;

impl Platform for WinitPlatform {
    fn run(config: WindowConfig, mut handler: Box<dyn EventHandler>) -> PlatformResult<()> {
        let event_loop = EventLoop::new().map_err(|e| PlatformError::EventLoop(e.to_string()))?;
        let window = WindowBuilder::new()
            .with_title(&config.title)
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .with_resizable(config.resizable)
            .build(&event_loop)
            .map_err(|e| PlatformError::WindowCreation(e.to_string()))?;

        let window = Arc::new(window);
        let handle = WinitWindowHandle {
            window: Arc::clone(&window),
        };
        let mut last_cursor_pos = Point::ZERO;

        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop
            .run(move |event, elwt| match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => match event {
                    WindowEvent::CloseRequested => {
                        handler.on_event(PlatformEvent::CloseRequested, &handle);
                        elwt.exit();
                    }
                    WindowEvent::RedrawRequested => {
                        handler.on_event(PlatformEvent::RedrawRequested, &handle);
                    }
                    WindowEvent::Resized(ps) => {
                        let size = Size::new(ps.width as f32, ps.height as f32);
                        handler.on_event(
                            PlatformEvent::Resized {
                                physical_size: size,
                            },
                            &handle,
                        );
                    }
                    WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                        handler.on_event(
                            PlatformEvent::ScaleFactorChanged {
                                scale_factor: *scale_factor,
                            },
                            &handle,
                        );
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let sf = window.scale_factor();
                        let lp = position.to_logical::<f32>(sf);
                        last_cursor_pos = Point::new(lp.x, lp.y);
                        handler.on_event(
                            PlatformEvent::Input(InputEvent::MouseMoved {
                                position: last_cursor_pos,
                            }),
                            &handle,
                        );
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        let mb = convert_mouse_button(button);
                        let ev = match state {
                            ElementState::Pressed => InputEvent::MousePressed {
                                button: mb,
                                position: last_cursor_pos,
                                modifiers: Modifiers::none(),
                            },
                            ElementState::Released => InputEvent::MouseReleased {
                                button: mb,
                                position: last_cursor_pos,
                                modifiers: Modifiers::none(),
                            },
                        };
                        handler.on_event(PlatformEvent::Input(ev), &handle);
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let (dx, dy) = match delta {
                            MouseScrollDelta::LineDelta(x, y) => (*x * 20.0, *y * 20.0),
                            MouseScrollDelta::PixelDelta(p) => {
                                let lp = p.to_logical::<f32>(window.scale_factor());
                                (lp.x, lp.y)
                            }
                        };
                        handler.on_event(
                            PlatformEvent::Input(InputEvent::MouseScrolled {
                                position: last_cursor_pos,
                                delta_x: dx,
                                delta_y: dy,
                                modifiers: Modifiers::none(),
                            }),
                            &handle,
                        );
                    }
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key,
                                state,
                                text,
                                ..
                            },
                        ..
                    } => {
                        let key = convert_key(logical_key);
                        let ev = match state {
                            ElementState::Pressed => InputEvent::KeyPressed {
                                key,
                                modifiers: Modifiers::none(),
                            },
                            ElementState::Released => InputEvent::KeyReleased {
                                key,
                                modifiers: Modifiers::none(),
                            },
                        };
                        handler.on_event(PlatformEvent::Input(ev), &handle);

                        if *state == ElementState::Pressed {
                            if let Some(s) = text {
                                for ch in s.chars() {
                                    handler.on_event(
                                        PlatformEvent::Input(InputEvent::CharInput { ch }),
                                        &handle,
                                    );
                                }
                            }
                        }
                    }
                    _ => {}
                },
                Event::AboutToWait => {
                    window.request_redraw();
                    handler.on_event(PlatformEvent::Idle, &handle);
                }
                _ => {}
            })
            .map_err(|e| PlatformError::EventLoop(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Key conversion helpers
// ---------------------------------------------------------------------------

fn convert_mouse_button(button: &WinitMouseButton) -> MouseButton {
    match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Other(n) => MouseButton::Other(*n),
        _ => MouseButton::Other(0),
    }
}

fn convert_key(key: &Key) -> KeyCode {
    match key {
        Key::Named(named) => convert_named_key(named),
        Key::Character(s) => {
            let Some(ch) = s.chars().next() else {
                return KeyCode::Unknown;
            };
            let upper = ch.to_ascii_uppercase();
            if upper.is_ascii_alphabetic() {
                const LETTERS: [KeyCode; 26] = [
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
                ];
                LETTERS[(upper as u8 - b'A') as usize]
            } else if ch.is_ascii_digit() {
                const DIGITS: [KeyCode; 10] = [
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
                ];
                DIGITS[(ch as u8 - b'0') as usize]
            } else {
                KeyCode::Unknown
            }
        }
        _ => KeyCode::Unknown,
    }
}

fn convert_named_key(key: &NamedKey) -> KeyCode {
    match key {
        NamedKey::Space => KeyCode::Space,
        NamedKey::Enter => KeyCode::Enter,
        NamedKey::Tab => KeyCode::Tab,
        NamedKey::Backspace => KeyCode::Backspace,
        NamedKey::Escape => KeyCode::Escape,
        NamedKey::Delete => KeyCode::Delete,
        NamedKey::ArrowLeft => KeyCode::ArrowLeft,
        NamedKey::ArrowRight => KeyCode::ArrowRight,
        NamedKey::ArrowUp => KeyCode::ArrowUp,
        NamedKey::ArrowDown => KeyCode::ArrowDown,
        NamedKey::Home => KeyCode::Home,
        NamedKey::End => KeyCode::End,
        NamedKey::PageUp => KeyCode::PageUp,
        NamedKey::PageDown => KeyCode::PageDown,
        NamedKey::Shift => KeyCode::Shift,
        NamedKey::Control => KeyCode::Control,
        NamedKey::Alt => KeyCode::Alt,
        NamedKey::Meta => KeyCode::Meta,
        NamedKey::F1 => KeyCode::F1,
        NamedKey::F2 => KeyCode::F2,
        NamedKey::F3 => KeyCode::F3,
        NamedKey::F4 => KeyCode::F4,
        NamedKey::F5 => KeyCode::F5,
        NamedKey::F6 => KeyCode::F6,
        NamedKey::F7 => KeyCode::F7,
        NamedKey::F8 => KeyCode::F8,
        NamedKey::F9 => KeyCode::F9,
        NamedKey::F10 => KeyCode::F10,
        NamedKey::F11 => KeyCode::F11,
        NamedKey::F12 => KeyCode::F12,
        _ => KeyCode::Unknown,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::SmolStr;

    fn char_key(s: &str) -> Key {
        Key::Character(SmolStr::new(s))
    }

    #[test]
    fn letter_keys_case_insensitive() {
        assert_eq!(convert_key(&char_key("a")), KeyCode::A);
        assert_eq!(convert_key(&char_key("A")), KeyCode::A);
        assert_eq!(convert_key(&char_key("z")), KeyCode::Z);
        assert_eq!(convert_key(&char_key("Z")), KeyCode::Z);
        assert_eq!(convert_key(&char_key("m")), KeyCode::M);
        assert_eq!(convert_key(&char_key("M")), KeyCode::M);
    }

    #[test]
    fn digit_keys() {
        assert_eq!(convert_key(&char_key("0")), KeyCode::Digit0);
        assert_eq!(convert_key(&char_key("5")), KeyCode::Digit5);
        assert_eq!(convert_key(&char_key("9")), KeyCode::Digit9);
    }

    #[test]
    fn unknown_character_returns_unknown() {
        assert_eq!(convert_key(&char_key("!")), KeyCode::Unknown);
        assert_eq!(convert_key(&char_key(" ")), KeyCode::Unknown);
        assert_eq!(convert_key(&char_key("")), KeyCode::Unknown);
    }
}
