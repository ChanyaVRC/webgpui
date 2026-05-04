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
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Space,
    Enter,
    Tab,
    Backspace,
    Escape,
    Delete,
    // Navigation
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,
    // Modifiers
    Shift,
    Control,
    Alt,
    Meta,
    // Function
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
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
    MousePressed {
        button: MouseButton,
        position: Point,
        modifiers: Modifiers,
    },
    /// A mouse button was released.
    MouseReleased {
        button: MouseButton,
        position: Point,
        modifiers: Modifiers,
    },
    /// The scroll wheel was moved.  Positive `delta_y` is scroll down.
    MouseScrolled {
        position: Point,
        delta_x: f32,
        delta_y: f32,
        modifiers: Modifiers,
    },
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
            InputEvent::MousePressed {
                button,
                position,
                modifiers,
            } => {
                self.cursor_position = *position;
                self.modifiers = *modifiers;
                self.pressed_buttons.insert(*button);
            }
            InputEvent::MouseReleased {
                button,
                position,
                modifiers,
            } => {
                self.cursor_position = *position;
                self.modifiers = *modifiers;
                self.pressed_buttons.remove(button);
            }
            InputEvent::MouseScrolled {
                position,
                modifiers,
                ..
            } => {
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
// EventPhase
// ---------------------------------------------------------------------------

/// The phase of a routed event as it travels through the node tree.
///
/// Event dispatch follows the DOM-style three-phase model:
/// 1. **Capture** – travels from the root down toward the target.
/// 2. **Target** – fires at the target node itself.
/// 3. **Bubble** – travels from the target back up toward the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

// ---------------------------------------------------------------------------
// dispatch
// ---------------------------------------------------------------------------

/// Dispatches `event` along `path` (root-to-target, inclusive) using the
/// capture → target → bubble model.
///
/// `visitor` is called once per phase per node:
/// - Capture phase: each node in `path` from index 0 to `len-2` (ancestors).
/// - Target phase: the last element of `path` with [`EventPhase::Target`].
/// - Bubble phase: each node in reverse order, from `len-2` down to 0.
///
/// If `visitor` returns `true` the propagation stops immediately
/// (analogous to `stopPropagation`).
///
/// # Panics
/// Panics if `path` is empty.
pub fn dispatch<F>(path: &[u32], event: &InputEvent, mut visitor: F)
where
    F: FnMut(u32, EventPhase, &InputEvent) -> bool,
{
    assert!(!path.is_empty(), "dispatch: path must not be empty");

    let (ancestors, target_slice) = path.split_at(path.len() - 1);
    let target = target_slice[0];

    // Capture phase: root → parent of target.
    for &node in ancestors {
        if visitor(node, EventPhase::Capture, event) {
            return;
        }
    }

    // Target phase.
    if visitor(target, EventPhase::Target, event) {
        return;
    }

    // Bubble phase: parent of target → root.
    for &node in ancestors.iter().rev() {
        if visitor(node, EventPhase::Bubble, event) {
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// FocusManager
// ---------------------------------------------------------------------------

/// Tracks which node currently holds keyboard focus and manages Tab traversal.
///
/// Nodes must be registered in the desired tab order via
/// [`FocusManager::register_focusable`].  The list is ordered by registration
/// order; adjust by calling [`FocusManager::set_focusable_order`] if needed.
#[derive(Debug, Default)]
pub struct FocusManager {
    /// The node id of the currently focused widget, if any.
    focused: Option<u32>,
    /// Ordered list of focusable node ids (tab order).
    focusable: Vec<u32>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self::default()
    }

    // ------------------------------------------------------------------
    // Focus state
    // ------------------------------------------------------------------

    /// Sets focus to `node_id` unconditionally.
    pub fn set_focus(&mut self, node_id: u32) {
        self.focused = Some(node_id);
    }

    /// Clears focus so that no node is focused.
    pub fn clear_focus(&mut self) {
        self.focused = None;
    }

    /// Returns the currently focused node id, if any.
    pub fn focused(&self) -> Option<u32> {
        self.focused
    }

    /// Returns `true` if `node_id` is the currently focused node.
    pub fn is_focused(&self, node_id: u32) -> bool {
        self.focused == Some(node_id)
    }

    // ------------------------------------------------------------------
    // Focusable registry
    // ------------------------------------------------------------------

    /// Appends `node_id` to the end of the tab-order list.
    ///
    /// If `node_id` is already registered this is a no-op.
    pub fn register_focusable(&mut self, node_id: u32) {
        if !self.focusable.contains(&node_id) {
            self.focusable.push(node_id);
        }
    }

    /// Removes `node_id` from the tab-order list.
    ///
    /// If the removed node was focused, focus is cleared.
    pub fn unregister_focusable(&mut self, node_id: u32) {
        self.focusable.retain(|&id| id != node_id);
        if self.focused == Some(node_id) {
            self.focused = None;
        }
    }

    /// Replaces the entire tab-order list with `order`.
    pub fn set_focusable_order(&mut self, order: Vec<u32>) {
        self.focusable = order;
    }

    /// Returns a slice of all focusable node ids in tab order.
    pub fn focusable_order(&self) -> &[u32] {
        &self.focusable
    }

    // ------------------------------------------------------------------
    // Tab traversal
    // ------------------------------------------------------------------

    /// Moves focus to the next focusable node (Tab).
    ///
    /// Wraps around to the first node after the last.
    /// Returns the newly focused node id, or `None` if the list is empty.
    pub fn move_focus_forward(&mut self) -> Option<u32> {
        self.step_focus(1)
    }

    /// Moves focus to the previous focusable node (Shift+Tab).
    ///
    /// Wraps around to the last node before the first.
    /// Returns the newly focused node id, or `None` if the list is empty.
    pub fn move_focus_backward(&mut self) -> Option<u32> {
        self.step_focus(-1)
    }

    fn step_focus(&mut self, delta: i64) -> Option<u32> {
        if self.focusable.is_empty() {
            return None;
        }
        let len = self.focusable.len();
        let next = match self
            .focused
            .and_then(|id| self.focusable.iter().position(|&x| x == id))
        {
            Some(pos) => (pos as i64 + delta).rem_euclid(len as i64) as usize,
            None if delta > 0 => 0,
            None => len - 1,
        };
        let next_id = self.focusable[next];
        self.focused = Some(next_id);
        Some(next_id)
    }

    // ------------------------------------------------------------------
    // Keyboard integration
    // ------------------------------------------------------------------

    /// Handles a key event for focus traversal.
    ///
    /// Moves focus forward on `Tab`, backward on `Shift+Tab`.
    /// Returns `true` if the key was consumed (i.e., was a Tab key).
    pub fn handle_key(&mut self, key: KeyCode, modifiers: Modifiers) -> bool {
        match key {
            KeyCode::Tab if modifiers.shift => {
                self.move_focus_backward();
                true
            }
            KeyCode::Tab => {
                self.move_focus_forward();
                true
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- InputState -------------------------------------------------------

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

    /// M1-1: pointer position must be consistent at press/release/scroll time.
    #[test]
    fn pointer_position_consistent_across_events() {
        let mut state = InputState::new();
        let pos = Point::new(42.0, 84.0);

        state.apply(&InputEvent::MousePressed {
            button: MouseButton::Left,
            position: pos,
            modifiers: Modifiers::none(),
        });
        assert_eq!(state.cursor_position, pos, "position at press");

        let release_pos = Point::new(43.0, 85.0);
        state.apply(&InputEvent::MouseReleased {
            button: MouseButton::Left,
            position: release_pos,
            modifiers: Modifiers::none(),
        });
        assert_eq!(state.cursor_position, release_pos, "position at release");

        let scroll_pos = Point::new(50.0, 50.0);
        state.apply(&InputEvent::MouseScrolled {
            position: scroll_pos,
            delta_x: 0.0,
            delta_y: 3.0,
            modifiers: Modifiers::none(),
        });
        assert_eq!(state.cursor_position, scroll_pos, "position at scroll");
    }

    // ---- dispatch / EventPhase -------------------------------------------

    fn make_click() -> InputEvent {
        InputEvent::MousePressed {
            button: MouseButton::Left,
            position: Point::new(0.0, 0.0),
            modifiers: Modifiers::none(),
        }
    }

    /// M1-2: events must arrive in capture → target → bubble order.
    #[test]
    fn event_propagation_capture_target_bubble_order() {
        let root: u32 = 0;
        let middle: u32 = 1;
        let leaf: u32 = 2;
        let path = [root, middle, leaf];

        let mut order: Vec<(u32, EventPhase)> = Vec::new();
        dispatch(&path, &make_click(), |node, phase, _| {
            order.push((node, phase));
            false
        });

        assert_eq!(
            order,
            vec![
                (root, EventPhase::Capture),
                (middle, EventPhase::Capture),
                (leaf, EventPhase::Target),
                (middle, EventPhase::Bubble),
                (root, EventPhase::Bubble),
            ]
        );
    }

    /// Stopping in the capture phase must prevent later phases.
    #[test]
    fn event_propagation_stop_in_capture() {
        let path = [0u32, 1u32, 2u32];
        let mut visited: Vec<(u32, EventPhase)> = Vec::new();
        dispatch(&path, &make_click(), |node, phase, _| {
            visited.push((node, phase));
            // Stop after root capture.
            node == 0 && phase == EventPhase::Capture
        });
        assert_eq!(visited, vec![(0, EventPhase::Capture)]);
    }

    /// Dispatch on a single-node path fires only the Target phase.
    #[test]
    fn event_propagation_single_node_is_target_only() {
        let path = [99u32];
        let mut visited: Vec<(u32, EventPhase)> = Vec::new();
        dispatch(&path, &make_click(), |node, phase, _| {
            visited.push((node, phase));
            false
        });
        assert_eq!(visited, vec![(99, EventPhase::Target)]);
    }

    // ---- FocusManager ----------------------------------------------------

    #[test]
    fn focus_manager_basic() {
        let mut fm = FocusManager::new();
        assert!(fm.focused().is_none());
        fm.set_focus(42);
        assert!(fm.is_focused(42));
        fm.clear_focus();
        assert!(fm.focused().is_none());
    }

    /// M1-3: Tab forward traversal wraps around.
    #[test]
    fn focus_tab_forward_wraps() {
        let mut fm = FocusManager::new();
        fm.register_focusable(1);
        fm.register_focusable(2);
        fm.register_focusable(3);
        fm.set_focus(1);

        assert_eq!(fm.move_focus_forward(), Some(2));
        assert_eq!(fm.move_focus_forward(), Some(3));
        assert_eq!(fm.move_focus_forward(), Some(1), "should wrap to first");
    }

    /// M1-3: Shift+Tab backward traversal wraps around.
    #[test]
    fn focus_tab_backward_wraps() {
        let mut fm = FocusManager::new();
        fm.register_focusable(10);
        fm.register_focusable(20);
        fm.register_focusable(30);
        fm.set_focus(10);

        assert_eq!(fm.move_focus_backward(), Some(30), "should wrap to last");
        assert_eq!(fm.move_focus_backward(), Some(20));
        assert_eq!(fm.move_focus_backward(), Some(10));
    }

    /// Tab from unfocused state moves to the first focusable.
    #[test]
    fn focus_tab_from_unfocused_starts_at_first() {
        let mut fm = FocusManager::new();
        fm.register_focusable(5);
        fm.register_focusable(6);
        assert_eq!(fm.move_focus_forward(), Some(5));
    }

    /// Unregistering the focused node clears focus.
    #[test]
    fn unregister_focused_clears_focus() {
        let mut fm = FocusManager::new();
        fm.register_focusable(7);
        fm.set_focus(7);
        fm.unregister_focusable(7);
        assert!(fm.focused().is_none());
        assert!(fm.focusable_order().is_empty());
    }

    /// Duplicate registration is ignored.
    #[test]
    fn register_focusable_deduplicates() {
        let mut fm = FocusManager::new();
        fm.register_focusable(1);
        fm.register_focusable(1);
        assert_eq!(fm.focusable_order().len(), 1);
    }

    /// handle_key consumes Tab and Shift+Tab, ignores others.
    #[test]
    fn handle_key_tab_consumed() {
        let mut fm = FocusManager::new();
        fm.register_focusable(1);
        fm.register_focusable(2);
        fm.set_focus(1);

        assert!(fm.handle_key(KeyCode::Tab, Modifiers::none()));
        assert_eq!(fm.focused(), Some(2));

        assert!(fm.handle_key(
            KeyCode::Tab,
            Modifiers {
                shift: true,
                ..Default::default()
            }
        ));
        assert_eq!(fm.focused(), Some(1));

        assert!(!fm.handle_key(KeyCode::Enter, Modifiers::none()));
    }

    /// Shift+Tab from unfocused state moves to the last focusable.
    #[test]
    fn focus_tab_backward_from_unfocused_starts_at_last() {
        let mut fm = FocusManager::new();
        fm.register_focusable(5);
        fm.register_focusable(6);
        fm.register_focusable(7);
        assert_eq!(fm.move_focus_backward(), Some(7));
    }

    /// handle_key with an empty focusable list returns true but stays unfocused.
    #[test]
    fn handle_key_tab_empty_list() {
        let mut fm = FocusManager::new();
        assert!(fm.handle_key(KeyCode::Tab, Modifiers::none()));
        assert!(fm.focused().is_none());
        assert!(fm.handle_key(
            KeyCode::Tab,
            Modifiers {
                shift: true,
                ..Default::default()
            }
        ));
        assert!(fm.focused().is_none());
    }

    /// set_focusable_order replaces the list; focusable_order reflects it.
    #[test]
    fn set_focusable_order_replaces_list() {
        let mut fm = FocusManager::new();
        fm.register_focusable(1);
        fm.register_focusable(2);
        fm.set_focusable_order(vec![30, 20, 10]);
        assert_eq!(fm.focusable_order(), &[30, 20, 10]);
        fm.set_focus(30);
        assert_eq!(fm.move_focus_forward(), Some(20));
    }

    /// Dispatch on a two-node path (parent + child) fires capture then target only.
    #[test]
    fn event_propagation_two_node_path() {
        let path = [0u32, 1u32];
        let mut visited: Vec<(u32, EventPhase)> = Vec::new();
        dispatch(&path, &make_click(), |node, phase, _| {
            visited.push((node, phase));
            false
        });
        assert_eq!(
            visited,
            vec![
                (0, EventPhase::Capture),
                (1, EventPhase::Target),
                (0, EventPhase::Bubble),
            ]
        );
    }
}
