//! Input event types and state tracking for webgpui.

use std::collections::HashSet;
use webgpui_geometry::Point;

// ---------------------------------------------------------------------------
// MouseButton
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

// ---------------------------------------------------------------------------
// KeyCode
// ---------------------------------------------------------------------------

/// Logical key identifiers (platform-independent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Printable characters / typing
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Digit0, Digit1, Digit2, Digit3, Digit4,
    Digit5, Digit6, Digit7, Digit8, Digit9,
    Space, Enter, Tab, Backspace, Escape, Delete,
    // Navigation
    ArrowLeft, ArrowRight, ArrowUp, ArrowDown,
    Home, End, PageUp, PageDown,
    // Modifiers
    Shift, Control, Alt, Meta,
    // Function
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Misc
    Unknown,
}

// ---------------------------------------------------------------------------
// Modifiers
// ---------------------------------------------------------------------------

/// Keyboard modifier state at the time of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }
}

// ---------------------------------------------------------------------------
// InputEvent
// ---------------------------------------------------------------------------

/// A single platform-independent input event.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// The cursor moved to `position` (in logical pixels).
    MouseMoved { position: Point },
    /// A mouse button was pressed.
    MousePressed { button: MouseButton, position: Point, modifiers: Modifiers },
    /// A mouse button was released.
    MouseReleased { button: MouseButton, position: Point, modifiers: Modifiers },
    /// The scroll wheel was moved.  Positive `delta_y` is scroll down.
    MouseScrolled { position: Point, delta_x: f32, delta_y: f32, modifiers: Modifiers },
    /// A keyboard key was pressed.
    KeyPressed { key: KeyCode, modifiers: Modifiers },
    /// A keyboard key was released.
    KeyReleased { key: KeyCode, modifiers: Modifiers },
    /// A Unicode character was typed.
    CharInput { ch: char },
}

// ---------------------------------------------------------------------------
// InputState
// ---------------------------------------------------------------------------

/// Snapshot of the current input device state.
///
/// Updated by feeding `InputEvent`s via [`InputState::apply`].
#[derive(Debug, Default)]
pub struct InputState {
    /// Current cursor position in logical pixels.
    pub cursor_position: Point,
    /// Currently held mouse buttons.
    pub pressed_buttons: HashSet<MouseButton>,
    /// Currently held keyboard keys.
    pub pressed_keys: HashSet<KeyCode>,
    /// Current modifier state.
    pub modifiers: Modifiers,
}

impl InputState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies an event to update the tracked state.
    pub fn apply(&mut self, event: &InputEvent) {
        match event {
            InputEvent::MouseMoved { position } => {
                self.cursor_position = *position;
            }
            InputEvent::MousePressed { button, position, modifiers } => {
                self.cursor_position = *position;
                self.modifiers = *modifiers;
                self.pressed_buttons.insert(*button);
            }
            InputEvent::MouseReleased { button, position, modifiers } => {
                self.cursor_position = *position;
                self.modifiers = *modifiers;
                self.pressed_buttons.remove(button);
            }
            InputEvent::MouseScrolled { position, modifiers, .. } => {
                self.cursor_position = *position;
                self.modifiers = *modifiers;
            }
            InputEvent::KeyPressed { key, modifiers } => {
                self.modifiers = *modifiers;
                self.pressed_keys.insert(*key);
            }
            InputEvent::KeyReleased { key, modifiers } => {
                self.modifiers = *modifiers;
                self.pressed_keys.remove(key);
            }
            InputEvent::CharInput { .. } => {}
        }
    }

    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        self.pressed_buttons.contains(&button)
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }
}

// ---------------------------------------------------------------------------
// FocusManager
// ---------------------------------------------------------------------------

/// Tracks which node currently holds keyboard focus.
#[derive(Debug, Default)]
pub struct FocusManager {
    /// The `NodeId` (stored as `u32`) of the focused widget, if any.
    focused: Option<u32>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_focus(&mut self, node_id: u32) {
        self.focused = Some(node_id);
    }

    pub fn clear_focus(&mut self) {
        self.focused = None;
    }

    pub fn focused(&self) -> Option<u32> {
        self.focused
    }

    pub fn is_focused(&self, node_id: u32) -> bool {
        self.focused == Some(node_id)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_tracks_button() {
        let mut state = InputState::new();
        state.apply(&InputEvent::MousePressed {
            button: MouseButton::Left,
            position: Point::new(10.0, 20.0),
            modifiers: Modifiers::none(),
        });
        assert!(state.is_button_pressed(MouseButton::Left));
        state.apply(&InputEvent::MouseReleased {
            button: MouseButton::Left,
            position: Point::new(10.0, 20.0),
            modifiers: Modifiers::none(),
        });
        assert!(!state.is_button_pressed(MouseButton::Left));
    }

    #[test]
    fn state_tracks_key() {
        let mut state = InputState::new();
        state.apply(&InputEvent::KeyPressed {
            key: KeyCode::A,
            modifiers: Modifiers::none(),
        });
        assert!(state.is_key_pressed(KeyCode::A));
    }

    #[test]
    fn focus_manager() {
        let mut fm = FocusManager::new();
        assert!(fm.focused().is_none());
        fm.set_focus(42);
        assert!(fm.is_focused(42));
        fm.clear_focus();
        assert!(fm.focused().is_none());
    }
}
