#![warn(missing_docs)]
//! Input event types and state tracking for webgpui.

use std::collections::{HashMap, HashSet};
use webgpui_core::NodeId;
use webgpui_geometry::Point;

// ---------------------------------------------------------------------------
// MouseButton
// ---------------------------------------------------------------------------

/// A mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button (scroll wheel click).
    Middle,
    /// Any other mouse button identified by its platform index.
    Other(u16),
}

// ---------------------------------------------------------------------------
// KeyCode
// ---------------------------------------------------------------------------

/// Logical key identifiers (platform-independent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Printable characters / typing
    /// The `A` key.
    A,
    /// The `B` key.
    B,
    /// The `C` key.
    C,
    /// The `D` key.
    D,
    /// The `E` key.
    E,
    /// The `F` key.
    F,
    /// The `G` key.
    G,
    /// The `H` key.
    H,
    /// The `I` key.
    I,
    /// The `J` key.
    J,
    /// The `K` key.
    K,
    /// The `L` key.
    L,
    /// The `M` key.
    M,
    /// The `N` key.
    N,
    /// The `O` key.
    O,
    /// The `P` key.
    P,
    /// The `Q` key.
    Q,
    /// The `R` key.
    R,
    /// The `S` key.
    S,
    /// The `T` key.
    T,
    /// The `U` key.
    U,
    /// The `V` key.
    V,
    /// The `W` key.
    W,
    /// The `X` key.
    X,
    /// The `Y` key.
    Y,
    /// The `Z` key.
    Z,
    /// The `0` digit key.
    Digit0,
    /// The `1` digit key.
    Digit1,
    /// The `2` digit key.
    Digit2,
    /// The `3` digit key.
    Digit3,
    /// The `4` digit key.
    Digit4,
    /// The `5` digit key.
    Digit5,
    /// The `6` digit key.
    Digit6,
    /// The `7` digit key.
    Digit7,
    /// The `8` digit key.
    Digit8,
    /// The `9` digit key.
    Digit9,
    /// The Space bar.
    Space,
    /// The Enter / Return key.
    Enter,
    /// The Tab key.
    Tab,
    /// The Backspace key.
    Backspace,
    /// The Escape key.
    Escape,
    /// The Delete (forward-delete) key.
    Delete,
    // Navigation
    /// The left arrow key.
    ArrowLeft,
    /// The right arrow key.
    ArrowRight,
    /// The up arrow key.
    ArrowUp,
    /// The down arrow key.
    ArrowDown,
    /// The Home key.
    Home,
    /// The End key.
    End,
    /// The Page Up key.
    PageUp,
    /// The Page Down key.
    PageDown,
    // Modifiers
    /// The Shift key.
    Shift,
    /// The Control key.
    Control,
    /// The Alt / Option key.
    Alt,
    /// The Meta / Command / Windows key.
    Meta,
    // Function
    /// The F1 function key.
    F1,
    /// The F2 function key.
    F2,
    /// The F3 function key.
    F3,
    /// The F4 function key.
    F4,
    /// The F5 function key.
    F5,
    /// The F6 function key.
    F6,
    /// The F7 function key.
    F7,
    /// The F8 function key.
    F8,
    /// The F9 function key.
    F9,
    /// The F10 function key.
    F10,
    /// The F11 function key.
    F11,
    /// The F12 function key.
    F12,
    // Misc
    /// A key that could not be mapped to any known variant.
    Unknown,
}

// ---------------------------------------------------------------------------
// Modifiers
// ---------------------------------------------------------------------------

/// Keyboard modifier state at the time of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Whether the Shift key is held.
    pub shift: bool,
    /// Whether the Control key is held.
    pub ctrl: bool,
    /// Whether the Alt / Option key is held.
    pub alt: bool,
    /// Whether the Meta / Command / Windows key is held.
    pub meta: bool,
}

impl Modifiers {
    /// Returns a `Modifiers` value with no modifier keys held.
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
    MouseMoved {
        /// New cursor position in logical pixels.
        position: Point,
    },
    /// A mouse button was pressed.
    MousePressed {
        /// The button that was pressed.
        button: MouseButton,
        /// Cursor position at the time of the press.
        position: Point,
        /// Modifier keys held at the time of the press.
        modifiers: Modifiers,
    },
    /// A mouse button was released.
    MouseReleased {
        /// The button that was released.
        button: MouseButton,
        /// Cursor position at the time of the release.
        position: Point,
        /// Modifier keys held at the time of the release.
        modifiers: Modifiers,
    },
    /// The scroll wheel was moved.  Positive `delta_y` is scroll down.
    MouseScrolled {
        /// Cursor position at the time of the scroll.
        position: Point,
        /// Horizontal scroll delta in logical pixels.
        delta_x: f32,
        /// Vertical scroll delta in logical pixels (positive = down).
        delta_y: f32,
        /// Modifier keys held at the time of the scroll.
        modifiers: Modifiers,
    },
    /// A keyboard key was pressed.
    KeyPressed {
        /// The key that was pressed.
        key: KeyCode,
        /// Modifier keys held at the time of the press.
        modifiers: Modifiers,
    },
    /// A keyboard key was released.
    KeyReleased {
        /// The key that was released.
        key: KeyCode,
        /// Modifier keys held at the time of the release.
        modifiers: Modifiers,
    },
    /// A Unicode character was typed.
    CharInput {
        /// The character that was typed.
        ch: char,
    },
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
    /// Creates a new `InputState` with all fields at their default (empty) values.
    ///
    /// # Examples
    ///
    /// ```
    /// use webgpui_input::InputState;
    ///
    /// let state = InputState::new();
    /// assert!(state.pressed_buttons.is_empty());
    /// assert!(state.pressed_keys.is_empty());
    /// ```
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

    /// Returns `true` if the given mouse `button` is currently held down.
    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        self.pressed_buttons.contains(&button)
    }

    /// Returns `true` if the given keyboard `key` is currently held down.
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
    /// Event is traveling from the root down toward the target.
    Capture,
    /// Event has reached its target node.
    Target,
    /// Event is traveling from the target back up toward the root.
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
/// Does nothing if `path` is empty.
pub fn dispatch<F>(path: &[NodeId], event: &InputEvent, mut visitor: F)
where
    F: FnMut(NodeId, EventPhase, &InputEvent) -> bool,
{
    if path.is_empty() {
        return;
    }

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
    focused: Option<NodeId>,
    /// Ordered list of focusable node ids (tab order).
    focusable: Vec<NodeId>,
    /// Maps NodeId → index in `focusable` for O(1) membership and position lookup.
    focusable_index: HashMap<NodeId, usize>,
}

impl FocusManager {
    /// Creates a new `FocusManager` with no focused node and an empty focusable list.
    ///
    /// # Examples
    ///
    /// ```
    /// use webgpui_input::FocusManager;
    ///
    /// let fm = FocusManager::new();
    /// assert!(fm.focused().is_none());
    /// assert!(fm.focusable_order().is_empty());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    // ------------------------------------------------------------------
    // Focus state
    // ------------------------------------------------------------------

    /// Sets focus to `node_id` unconditionally.
    pub fn set_focus(&mut self, node_id: NodeId) {
        self.focused = Some(node_id);
    }

    /// Clears focus so that no node is focused.
    pub fn clear_focus(&mut self) {
        self.focused = None;
    }

    /// Returns the currently focused node id, if any.
    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    /// Returns `true` if `node_id` is the currently focused node.
    pub fn is_focused(&self, node_id: NodeId) -> bool {
        self.focused == Some(node_id)
    }

    // ------------------------------------------------------------------
    // Focusable registry
    // ------------------------------------------------------------------

    /// Appends `node_id` to the end of the tab-order list.
    ///
    /// If `node_id` is already registered this is a no-op.
    pub fn register_focusable(&mut self, node_id: NodeId) {
        if self.focusable_index.contains_key(&node_id) {
            return;
        }
        let idx = self.focusable.len();
        self.focusable.push(node_id);
        self.focusable_index.insert(node_id, idx);
    }

    /// Removes `node_id` from the tab-order list.
    ///
    /// If the removed node was focused, focus is cleared.
    pub fn unregister_focusable(&mut self, node_id: NodeId) {
        if self.focusable_index.remove(&node_id).is_none() {
            // not registered, just check focus
            if self.focused == Some(node_id) {
                self.focused = None;
            }
            return;
        }
        self.focusable.retain(|&id| id != node_id);
        // Rebuild index since positions shifted
        self.focusable_index.clear();
        for (i, &id) in self.focusable.iter().enumerate() {
            self.focusable_index.insert(id, i);
        }
        if self.focused == Some(node_id) {
            self.focused = None;
        }
    }

    /// Replaces the entire tab-order list with `order`.
    ///
    /// Duplicates in `order` are removed (first occurrence wins).
    /// If the currently focused node is not present in the new list, focus is cleared.
    pub fn set_focusable_order(&mut self, order: Vec<NodeId>) {
        let mut seen = HashSet::new();
        self.focusable = order.into_iter().filter(|id| seen.insert(*id)).collect();
        self.focusable_index.clear();
        for (i, &id) in self.focusable.iter().enumerate() {
            self.focusable_index.insert(id, i);
        }
        if let Some(focused) = self.focused {
            if !self.focusable_index.contains_key(&focused) {
                self.focused = None;
            }
        }
    }

    /// Returns a slice of all focusable node ids in tab order.
    pub fn focusable_order(&self) -> &[NodeId] {
        &self.focusable
    }

    // ------------------------------------------------------------------
    // Tab traversal
    // ------------------------------------------------------------------

    /// Moves focus to the next focusable node (Tab).
    ///
    /// Wraps around to the first node after the last.
    /// Returns the newly focused node id, or `None` if the list is empty.
    pub fn move_focus_forward(&mut self) -> Option<NodeId> {
        self.step_focus(1)
    }

    /// Moves focus to the previous focusable node (Shift+Tab).
    ///
    /// Wraps around to the last node before the first.
    /// Returns the newly focused node id, or `None` if the list is empty.
    pub fn move_focus_backward(&mut self) -> Option<NodeId> {
        self.step_focus(-1)
    }

    fn step_focus(&mut self, delta: i64) -> Option<NodeId> {
        if self.focusable.is_empty() {
            return None;
        }
        let len = self.focusable.len();
        let next = match self
            .focused
            .and_then(|id| self.focusable_index.get(&id).copied())
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
        let root = NodeId(0);
        let middle = NodeId(1);
        let leaf = NodeId(2);
        let path = [root, middle, leaf];

        let mut order: Vec<(NodeId, EventPhase)> = Vec::new();
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
        let path = [NodeId(0), NodeId(1), NodeId(2)];
        let mut visited: Vec<(NodeId, EventPhase)> = Vec::new();
        dispatch(&path, &make_click(), |node, phase, _| {
            visited.push((node, phase));
            // Stop after root capture.
            node == NodeId(0) && phase == EventPhase::Capture
        });
        assert_eq!(visited, vec![(NodeId(0), EventPhase::Capture)]);
    }

    /// Dispatch on a single-node path fires only the Target phase.
    #[test]
    fn event_propagation_single_node_is_target_only() {
        let path = [NodeId(99)];
        let mut visited: Vec<(NodeId, EventPhase)> = Vec::new();
        dispatch(&path, &make_click(), |node, phase, _| {
            visited.push((node, phase));
            false
        });
        assert_eq!(visited, vec![(NodeId(99), EventPhase::Target)]);
    }

    // ---- FocusManager ----------------------------------------------------

    #[test]
    fn focus_manager_basic() {
        let mut fm = FocusManager::new();
        assert!(fm.focused().is_none());
        fm.set_focus(NodeId(42));
        assert!(fm.is_focused(NodeId(42)));
        fm.clear_focus();
        assert!(fm.focused().is_none());
    }

    /// M1-3: Tab forward traversal wraps around.
    #[test]
    fn focus_tab_forward_wraps() {
        let mut fm = FocusManager::new();
        fm.register_focusable(NodeId(1));
        fm.register_focusable(NodeId(2));
        fm.register_focusable(NodeId(3));
        fm.set_focus(NodeId(1));

        assert_eq!(fm.move_focus_forward(), Some(NodeId(2)));
        assert_eq!(fm.move_focus_forward(), Some(NodeId(3)));
        assert_eq!(
            fm.move_focus_forward(),
            Some(NodeId(1)),
            "should wrap to first"
        );
    }

    /// M1-3: Shift+Tab backward traversal wraps around.
    #[test]
    fn focus_tab_backward_wraps() {
        let mut fm = FocusManager::new();
        fm.register_focusable(NodeId(10));
        fm.register_focusable(NodeId(20));
        fm.register_focusable(NodeId(30));
        fm.set_focus(NodeId(10));

        assert_eq!(
            fm.move_focus_backward(),
            Some(NodeId(30)),
            "should wrap to last"
        );
        assert_eq!(fm.move_focus_backward(), Some(NodeId(20)));
        assert_eq!(fm.move_focus_backward(), Some(NodeId(10)));
    }

    /// Tab from unfocused state moves to the first focusable.
    #[test]
    fn focus_tab_from_unfocused_starts_at_first() {
        let mut fm = FocusManager::new();
        fm.register_focusable(NodeId(5));
        fm.register_focusable(NodeId(6));
        assert_eq!(fm.move_focus_forward(), Some(NodeId(5)));
    }

    /// Unregistering the focused node clears focus.
    #[test]
    fn unregister_focused_clears_focus() {
        let mut fm = FocusManager::new();
        fm.register_focusable(NodeId(7));
        fm.set_focus(NodeId(7));
        fm.unregister_focusable(NodeId(7));
        assert!(fm.focused().is_none());
        assert!(fm.focusable_order().is_empty());
    }

    /// Duplicate registration is ignored.
    #[test]
    fn register_focusable_deduplicates() {
        let mut fm = FocusManager::new();
        fm.register_focusable(NodeId(1));
        fm.register_focusable(NodeId(1));
        assert_eq!(fm.focusable_order().len(), 1);
    }

    /// handle_key consumes Tab and Shift+Tab, ignores others.
    #[test]
    fn handle_key_tab_consumed() {
        let mut fm = FocusManager::new();
        fm.register_focusable(NodeId(1));
        fm.register_focusable(NodeId(2));
        fm.set_focus(NodeId(1));

        assert!(fm.handle_key(KeyCode::Tab, Modifiers::none()));
        assert_eq!(fm.focused(), Some(NodeId(2)));

        assert!(fm.handle_key(
            KeyCode::Tab,
            Modifiers {
                shift: true,
                ..Default::default()
            }
        ));
        assert_eq!(fm.focused(), Some(NodeId(1)));

        assert!(!fm.handle_key(KeyCode::Enter, Modifiers::none()));
    }

    /// Shift+Tab from unfocused state moves to the last focusable.
    #[test]
    fn focus_tab_backward_from_unfocused_starts_at_last() {
        let mut fm = FocusManager::new();
        fm.register_focusable(NodeId(5));
        fm.register_focusable(NodeId(6));
        fm.register_focusable(NodeId(7));
        assert_eq!(fm.move_focus_backward(), Some(NodeId(7)));
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
        fm.register_focusable(NodeId(1));
        fm.register_focusable(NodeId(2));
        fm.set_focusable_order(vec![NodeId(30), NodeId(20), NodeId(10)]);
        assert_eq!(fm.focusable_order(), &[NodeId(30), NodeId(20), NodeId(10)]);
        fm.set_focus(NodeId(30));
        assert_eq!(fm.move_focus_forward(), Some(NodeId(20)));
    }

    /// set_focusable_order deduplicates the supplied list.
    #[test]
    fn set_focusable_order_deduplicates() {
        let mut fm = FocusManager::new();
        fm.set_focusable_order(vec![NodeId(1), NodeId(2), NodeId(1), NodeId(3), NodeId(2)]);
        assert_eq!(fm.focusable_order(), &[NodeId(1), NodeId(2), NodeId(3)]);
    }

    /// set_focusable_order clears focus when the focused node is absent from the new list.
    #[test]
    fn set_focusable_order_clears_focus_if_absent() {
        let mut fm = FocusManager::new();
        fm.register_focusable(NodeId(1));
        fm.set_focus(NodeId(1));
        fm.set_focusable_order(vec![NodeId(2), NodeId(3)]);
        assert!(fm.focused().is_none());
    }

    /// O(1) index: position lookup via HashMap drives correct step_focus results.
    #[test]
    fn focus_manager_o1_lookup_on_step() {
        let mut fm = FocusManager::new();
        for i in 0..100u64 {
            fm.register_focusable(NodeId(i));
        }
        fm.set_focus(NodeId(50));
        assert_eq!(fm.move_focus_forward(), Some(NodeId(51)));
        assert_eq!(fm.move_focus_backward(), Some(NodeId(50)));
    }

    /// dispatch on an empty path is a no-op.
    #[test]
    fn dispatch_empty_path_is_noop() {
        let mut called = false;
        dispatch(&[], &make_click(), |_, _, _| {
            called = true;
            false
        });
        assert!(!called);
    }

    /// Dispatch on a two-node path fires capture on parent, target on child, then bubble on parent.
    #[test]
    fn event_propagation_two_node_path() {
        let path = [NodeId(0), NodeId(1)];
        let mut visited: Vec<(NodeId, EventPhase)> = Vec::new();
        dispatch(&path, &make_click(), |node, phase, _| {
            visited.push((node, phase));
            false
        });
        assert_eq!(
            visited,
            vec![
                (NodeId(0), EventPhase::Capture),
                (NodeId(1), EventPhase::Target),
                (NodeId(0), EventPhase::Bubble),
            ]
        );
    }
}
