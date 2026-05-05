// ---------------------------------------------------------------------------
// WidgetState
// ---------------------------------------------------------------------------

/// Interaction state shared by all interactive widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WidgetState {
    #[default]
    Normal,
    Hover,
    Pressed,
    Focused,
    Disabled,
}

// ---------------------------------------------------------------------------
// TextAlign
// ---------------------------------------------------------------------------

/// Horizontal alignment for text within a [`Label`] or [`TextInput`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Start,
    Center,
    End,
}

// ---------------------------------------------------------------------------
// CursorMove
// ---------------------------------------------------------------------------

/// Logical cursor movement within a [`TextInput`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorMove {
    Left,
    Right,
    Home,
    End,
}

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

/// A stateful button widget.
///
/// The caller is responsible for routing input events to the appropriate
/// methods (`set_hovered`, `press`, `release`, etc.) based on hit-testing
/// and focus management.
pub struct Button {
    state: WidgetState,
    label: String,
    /// Whether the button currently holds keyboard focus.
    /// Tracked separately from `state` so that `release()` can restore it.
    focused: bool,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            state: WidgetState::Normal,
            label: label.into(),
            focused: false,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn state(&self) -> WidgetState {
        self.state
    }

    /// Enable or disable the button. Disabled buttons ignore all other events.
    pub fn set_disabled(&mut self, disabled: bool) {
        if disabled {
            self.state = WidgetState::Disabled;
        } else if self.state == WidgetState::Disabled {
            self.state = WidgetState::Normal;
        }
    }

    /// Notify hover enter (`true`) or leave (`false`).
    pub fn set_hovered(&mut self, hovered: bool) {
        if self.state == WidgetState::Disabled {
            return;
        }
        if hovered {
            if self.state == WidgetState::Normal {
                self.state = WidgetState::Hover;
            }
            // Focused and Pressed states take priority over Hover.
        } else if self.state == WidgetState::Hover {
            self.state = if self.focused {
                WidgetState::Focused
            } else {
                WidgetState::Normal
            };
        }
    }

    /// Notify focus gain (`true`) or loss (`false`).
    pub fn set_focused(&mut self, focused: bool) {
        if self.state == WidgetState::Disabled {
            return;
        }
        self.focused = focused;
        if focused {
            if matches!(self.state, WidgetState::Normal | WidgetState::Hover) {
                self.state = WidgetState::Focused;
            }
        } else if matches!(self.state, WidgetState::Focused | WidgetState::Pressed) {
            self.state = WidgetState::Normal;
        }
    }

    /// Begin a press (mouse-down or Enter / Space key-down when focused).
    pub fn press(&mut self) {
        if self.state == WidgetState::Disabled {
            return;
        }
        self.state = WidgetState::Pressed;
    }

    /// End a press. Returns `true` if the button was activated (pressed and
    /// released while not disabled).
    pub fn release(&mut self) -> bool {
        if self.state == WidgetState::Disabled {
            return false;
        }
        let activated = self.state == WidgetState::Pressed;
        self.state = if self.focused {
            WidgetState::Focused
        } else {
            WidgetState::Normal
        };
        activated
    }
}

// ---------------------------------------------------------------------------
// TextInput
// ---------------------------------------------------------------------------

/// A stateful single-line text-input widget.
///
/// Internally stores characters as a `Vec<char>` so that all cursor indices
/// are char-boundary-safe.
pub struct TextInput {
    state: WidgetState,
    chars: Vec<char>,
    /// Cursor position as a char index in `[0, chars.len()]`.
    cursor: usize,
    /// When `Some`, a selection is active from `selection_anchor` to `cursor`.
    selection_anchor: Option<usize>,
    placeholder: String,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            state: WidgetState::Normal,
            chars: Vec::new(),
            cursor: 0,
            selection_anchor: None,
            placeholder: String::new(),
        }
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn state(&self) -> WidgetState {
        self.state
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Returns the current text value.
    pub fn value(&self) -> String {
        self.chars.iter().collect()
    }

    /// Returns the ordered selection range `(lo, hi)` if any chars are selected.
    /// Returns `None` when `anchor == cursor` (collapsed caret).
    pub fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        if anchor == self.cursor {
            return None;
        }
        let lo = anchor.min(self.cursor);
        let hi = anchor.max(self.cursor);
        Some((lo, hi))
    }

    /// Notify focus gain (`true`) or loss (`false`).
    pub fn set_focused(&mut self, focused: bool) {
        if self.state == WidgetState::Disabled {
            return;
        }
        self.state = if focused {
            WidgetState::Focused
        } else {
            WidgetState::Normal
        };
        if !focused {
            self.selection_anchor = None;
        }
    }

    /// Insert a character at the cursor (replacing the current selection if any).
    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        self.chars.insert(self.cursor, ch);
        self.cursor += 1;
    }

    /// Delete the character before the cursor (Backspace), or the current
    /// selection. Returns `true` if any content was removed.
    pub fn delete_backward(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.chars.remove(self.cursor);
        true
    }

    /// Delete the character after the cursor (Delete key), or the current
    /// selection. Returns `true` if any content was removed.
    pub fn delete_forward(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor >= self.chars.len() {
            return false;
        }
        self.chars.remove(self.cursor);
        true
    }

    /// Move the cursor. When `select` is `true` the selection anchor is pinned
    /// at the current position (extending or creating a selection). When
    /// `select` is `false` any existing selection is collapsed.
    pub fn move_cursor(&mut self, mv: CursorMove, select: bool) {
        if select && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        } else if !select {
            self.selection_anchor = None;
        }
        self.cursor = match mv {
            CursorMove::Left => self.cursor.saturating_sub(1),
            CursorMove::Right => (self.cursor + 1).min(self.chars.len()),
            CursorMove::Home => 0,
            CursorMove::End => self.chars.len(),
        };
    }

    /// Deletes the selected range. Returns `true` if anything was deleted.
    fn delete_selection(&mut self) -> bool {
        let Some((lo, hi)) = self.selection() else {
            return false;
        };
        self.chars.drain(lo..hi);
        self.cursor = lo;
        self.selection_anchor = None;
        true
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Label
// ---------------------------------------------------------------------------

/// A read-only text label widget.
pub struct Label {
    text: String,
    align: TextAlign,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            align: TextAlign::Start,
        }
    }

    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
    pub fn align(&self) -> TextAlign {
        self.align
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Button state transitions -----------------------------------------

    #[test]
    fn button_default_is_normal() {
        assert_eq!(Button::new("OK").state(), WidgetState::Normal);
    }

    #[test]
    fn button_hover_enter_leave() {
        let mut b = Button::new("OK");
        b.set_hovered(true);
        assert_eq!(b.state(), WidgetState::Hover);
        b.set_hovered(false);
        assert_eq!(b.state(), WidgetState::Normal);
    }

    #[test]
    fn button_press_release_activates() {
        let mut b = Button::new("OK");
        b.press();
        assert_eq!(b.state(), WidgetState::Pressed);
        assert!(b.release());
        // Not focused, so returns to Normal.
        assert_eq!(b.state(), WidgetState::Normal);
    }

    #[test]
    fn button_focused_press_release_stays_focused() {
        let mut b = Button::new("OK");
        b.set_focused(true);
        assert_eq!(b.state(), WidgetState::Focused);
        b.press();
        assert_eq!(b.state(), WidgetState::Pressed);
        assert!(b.release());
        // Keyboard focus is preserved after release.
        assert_eq!(b.state(), WidgetState::Focused);
    }

    #[test]
    fn button_hover_does_not_clobber_focused() {
        let mut b = Button::new("OK");
        b.set_focused(true);
        assert_eq!(b.state(), WidgetState::Focused);
        // Hovering a focused button must not drop the focus ring.
        b.set_hovered(true);
        assert_eq!(b.state(), WidgetState::Focused);
        // Un-hovering must restore Focused, not Normal.
        b.set_hovered(false);
        assert_eq!(b.state(), WidgetState::Focused);
    }

    #[test]
    fn button_release_without_press_returns_false() {
        let mut b = Button::new("OK");
        assert!(!b.release());
    }

    #[test]
    fn button_focused_then_unfocused() {
        let mut b = Button::new("OK");
        b.set_focused(true);
        assert_eq!(b.state(), WidgetState::Focused);
        b.set_focused(false);
        assert_eq!(b.state(), WidgetState::Normal);
    }

    #[test]
    fn button_disabled_ignores_all_events() {
        let mut b = Button::new("OK");
        b.set_disabled(true);
        b.set_hovered(true);
        b.set_focused(true);
        b.press();
        assert_eq!(b.state(), WidgetState::Disabled);
        assert!(!b.release());
        assert_eq!(b.state(), WidgetState::Disabled);
    }

    #[test]
    fn button_reenable_returns_to_normal() {
        let mut b = Button::new("OK");
        b.set_disabled(true);
        b.set_disabled(false);
        assert_eq!(b.state(), WidgetState::Normal);
    }

    // ---- TextInput -------------------------------------------------------

    #[test]
    fn textinput_insert_chars() {
        let mut t = TextInput::new();
        t.insert_char('h');
        t.insert_char('i');
        assert_eq!(t.value(), "hi");
        assert_eq!(t.cursor(), 2);
    }

    #[test]
    fn textinput_backspace_removes_previous() {
        let mut t = TextInput::new();
        t.insert_char('h');
        t.insert_char('i');
        assert!(t.delete_backward());
        assert_eq!(t.value(), "h");
        assert_eq!(t.cursor(), 1);
    }

    #[test]
    fn textinput_backspace_at_start_is_noop() {
        let mut t = TextInput::new();
        assert!(!t.delete_backward());
        assert_eq!(t.value(), "");
    }

    #[test]
    fn textinput_delete_forward_removes_next() {
        let mut t = TextInput::new();
        for ch in "hi".chars() {
            t.insert_char(ch);
        }
        t.move_cursor(CursorMove::Home, false);
        assert!(t.delete_forward());
        assert_eq!(t.value(), "i");
        assert_eq!(t.cursor(), 0);
    }

    #[test]
    fn textinput_delete_forward_at_end_is_noop() {
        let mut t = TextInput::new();
        assert!(!t.delete_forward());
    }

    #[test]
    fn textinput_cursor_left_right_home_end() {
        let mut t = TextInput::new();
        for ch in "hello".chars() {
            t.insert_char(ch);
        }
        assert_eq!(t.cursor(), 5);
        t.move_cursor(CursorMove::Left, false);
        assert_eq!(t.cursor(), 4);
        t.move_cursor(CursorMove::Home, false);
        assert_eq!(t.cursor(), 0);
        t.move_cursor(CursorMove::Right, false);
        assert_eq!(t.cursor(), 1);
        t.move_cursor(CursorMove::End, false);
        assert_eq!(t.cursor(), 5);
    }

    #[test]
    fn textinput_selection_via_shift_right() {
        let mut t = TextInput::new();
        for ch in "hello".chars() {
            t.insert_char(ch);
        }
        t.move_cursor(CursorMove::Home, false);
        t.move_cursor(CursorMove::Right, true);
        t.move_cursor(CursorMove::Right, true);
        assert_eq!(t.selection(), Some((0, 2)));
    }

    #[test]
    fn textinput_selection_collapsed_is_none() {
        let mut t = TextInput::new();
        t.insert_char('a');
        t.move_cursor(CursorMove::Left, true); // anchor=1, cursor=0 → Some
        t.move_cursor(CursorMove::Right, true); // cursor back to 1 == anchor → None
        assert_eq!(t.selection(), None);
    }

    #[test]
    fn textinput_delete_backward_removes_selection() {
        let mut t = TextInput::new();
        for ch in "hello".chars() {
            t.insert_char(ch);
        }
        t.move_cursor(CursorMove::Home, false);
        t.move_cursor(CursorMove::Right, true);
        t.move_cursor(CursorMove::Right, true); // select "he"
        assert!(t.delete_backward());
        assert_eq!(t.value(), "llo");
        assert_eq!(t.cursor(), 0);
        assert_eq!(t.selection(), None);
    }

    #[test]
    fn textinput_insert_replaces_selection() {
        let mut t = TextInput::new();
        for ch in "hello".chars() {
            t.insert_char(ch);
        }
        t.move_cursor(CursorMove::Home, false);
        t.move_cursor(CursorMove::Right, true);
        t.move_cursor(CursorMove::Right, true); // select "he"
        t.insert_char('X');
        assert_eq!(t.value(), "Xllo");
        assert_eq!(t.cursor(), 1);
    }

    #[test]
    fn textinput_placeholder() {
        let t = TextInput::new().with_placeholder("Type here");
        assert_eq!(t.placeholder(), "Type here");
        assert_eq!(t.value(), "");
    }

    #[test]
    fn textinput_focus_clears_selection() {
        let mut t = TextInput::new();
        t.insert_char('a');
        t.move_cursor(CursorMove::Left, true);
        assert!(t.selection().is_some());
        t.set_focused(false);
        assert!(t.selection().is_none());
    }

    // ---- Label -----------------------------------------------------------

    #[test]
    fn label_default_align_start() {
        let l = Label::new("Hello");
        assert_eq!(l.text(), "Hello");
        assert_eq!(l.align(), TextAlign::Start);
    }

    #[test]
    fn label_with_center_align() {
        let l = Label::new("Hi").with_align(TextAlign::Center);
        assert_eq!(l.align(), TextAlign::Center);
    }

    #[test]
    fn label_with_end_align() {
        let l = Label::new("Hi").with_align(TextAlign::End);
        assert_eq!(l.align(), TextAlign::End);
    }

    #[test]
    fn label_set_text() {
        let mut l = Label::new("Hello");
        l.set_text("World");
        assert_eq!(l.text(), "World");
    }
}
