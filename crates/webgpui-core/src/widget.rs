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
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            state: WidgetState::Normal,
            label: label.into(),
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
            if matches!(self.state, WidgetState::Normal | WidgetState::Focused) {
                self.state = WidgetState::Hover;
            }
        } else if self.state == WidgetState::Hover {
            self.state = WidgetState::Normal;
        }
    }

    /// Notify focus gain (`true`) or loss (`false`).
    pub fn set_focused(&mut self, focused: bool) {
        if self.state == WidgetState::Disabled {
            return;
        }
        if focused {
            if self.state == WidgetState::Normal {
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
        self.state = WidgetState::Normal;
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
// ScrollView
// ---------------------------------------------------------------------------

/// A scrollable viewport that tracks scroll offset and detects content overflow.
///
/// The caller is responsible for measuring content and viewport sizes and for
/// routing scroll events to `scroll_by` / `scroll_to`.
pub struct ScrollView {
    scroll_offset: (f32, f32),
    content_size: (f32, f32),
    viewport_size: (f32, f32),
}

impl ScrollView {
    pub fn new(viewport_size: (f32, f32)) -> Self {
        Self {
            scroll_offset: (0.0, 0.0),
            content_size: (0.0, 0.0),
            viewport_size,
        }
    }

    pub fn set_content_size(&mut self, size: (f32, f32)) {
        self.content_size = size;
        self.clamp_offset();
    }

    pub fn set_viewport_size(&mut self, size: (f32, f32)) {
        self.viewport_size = size;
        self.clamp_offset();
    }

    pub fn scroll_offset(&self) -> (f32, f32) {
        self.scroll_offset
    }

    pub fn overflow_x(&self) -> bool {
        self.content_size.0 > self.viewport_size.0
    }

    pub fn overflow_y(&self) -> bool {
        self.content_size.1 > self.viewport_size.1
    }

    /// Scroll by a relative delta. Offset is clamped to `[0, max]`.
    pub fn scroll_by(&mut self, dx: f32, dy: f32) {
        self.scroll_to(self.scroll_offset.0 + dx, self.scroll_offset.1 + dy);
    }

    /// Scroll to an absolute position. Offset is clamped to `[0, max]`.
    pub fn scroll_to(&mut self, x: f32, y: f32) {
        self.scroll_offset = (
            x.clamp(0.0, self.max_offset_x()),
            y.clamp(0.0, self.max_offset_y()),
        );
    }

    fn max_offset_x(&self) -> f32 {
        (self.content_size.0 - self.viewport_size.0).max(0.0)
    }

    fn max_offset_y(&self) -> f32 {
        (self.content_size.1 - self.viewport_size.1).max(0.0)
    }

    fn clamp_offset(&mut self) {
        self.scroll_offset = (
            self.scroll_offset.0.clamp(0.0, self.max_offset_x()),
            self.scroll_offset.1.clamp(0.0, self.max_offset_y()),
        );
    }
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------

/// A horizontal strip of labelled items with configurable gap spacing.
///
/// Layout (Direction::Row + flex_grow) is handled by the layout engine; this
/// struct tracks the item list and gap for state-machine purposes.
pub struct Toolbar {
    items: Vec<String>,
    gap: f32,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            gap: 8.0,
        }
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn add_item(&mut self, label: impl Into<String>) {
        self.items.push(label.into());
    }

    pub fn items(&self) -> &[String] {
        &self.items
    }

    pub fn gap(&self) -> f32 {
        self.gap
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// TabBar
// ---------------------------------------------------------------------------

/// A tab bar with keyboard-navigable tab selection.
///
/// Selection wraps: `select_next` on the last tab goes to the first, and
/// `select_prev` on the first tab goes to the last.
pub struct TabBar {
    tabs: Vec<String>,
    selected: usize,
}

impl TabBar {
    pub fn new(tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            tabs: tabs.into_iter().map(|t| t.into()).collect(),
            selected: 0,
        }
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn label(&self, index: usize) -> &str {
        &self.tabs[index]
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Select by index. Clamps to `[0, len-1]`.
    pub fn select(&mut self, index: usize) {
        if !self.tabs.is_empty() {
            self.selected = index.min(self.tabs.len() - 1);
        }
    }

    /// Move to the next tab, wrapping from last to first.
    pub fn select_next(&mut self) {
        if !self.tabs.is_empty() {
            self.selected = (self.selected + 1) % self.tabs.len();
        }
    }

    /// Move to the previous tab, wrapping from first to last.
    pub fn select_prev(&mut self) {
        if !self.tabs.is_empty() {
            self.selected = if self.selected == 0 {
                self.tabs.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    /// Jump to the first tab (Home key).
    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    /// Jump to the last tab (End key).
    pub fn select_last(&mut self) {
        self.selected = self.tabs.len().saturating_sub(1);
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
        assert_eq!(b.state(), WidgetState::Normal);
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

    // ---- ScrollView ------------------------------------------------------

    #[test]
    fn scrollview_no_overflow_when_content_fits() {
        let mut sv = ScrollView::new((200.0, 100.0));
        sv.set_content_size((150.0, 80.0));
        assert!(!sv.overflow_x());
        assert!(!sv.overflow_y());
    }

    #[test]
    fn scrollview_overflow_when_content_exceeds_viewport() {
        let mut sv = ScrollView::new((200.0, 100.0));
        sv.set_content_size((400.0, 300.0));
        assert!(sv.overflow_x());
        assert!(sv.overflow_y());
    }

    #[test]
    fn scrollview_scroll_by_clamped_at_zero() {
        let mut sv = ScrollView::new((200.0, 100.0));
        sv.set_content_size((400.0, 300.0));
        sv.scroll_by(-50.0, -50.0);
        assert_eq!(sv.scroll_offset(), (0.0, 0.0));
    }

    #[test]
    fn scrollview_scroll_by_clamped_at_max() {
        let mut sv = ScrollView::new((200.0, 100.0));
        sv.set_content_size((400.0, 300.0));
        sv.scroll_by(9999.0, 9999.0);
        assert_eq!(sv.scroll_offset(), (200.0, 200.0));
    }

    #[test]
    fn scrollview_scroll_to_absolute() {
        let mut sv = ScrollView::new((200.0, 100.0));
        sv.set_content_size((400.0, 300.0));
        sv.scroll_to(50.0, 75.0);
        assert_eq!(sv.scroll_offset(), (50.0, 75.0));
    }

    #[test]
    fn scrollview_offset_clamped_on_viewport_resize() {
        let mut sv = ScrollView::new((100.0, 100.0));
        sv.set_content_size((400.0, 400.0));
        sv.scroll_to(200.0, 200.0);
        // Enlarge viewport so max offset shrinks.
        sv.set_viewport_size((350.0, 350.0));
        let (ox, oy) = sv.scroll_offset();
        assert!(ox <= 50.0);
        assert!(oy <= 50.0);
    }

    // ---- Toolbar ---------------------------------------------------------

    #[test]
    fn toolbar_add_items() {
        let mut tb = Toolbar::new();
        tb.add_item("File");
        tb.add_item("Edit");
        assert_eq!(tb.len(), 2);
        assert_eq!(tb.items(), &["File", "Edit"]);
    }

    #[test]
    fn toolbar_custom_gap() {
        let tb = Toolbar::new().with_gap(16.0);
        assert_eq!(tb.gap(), 16.0);
    }

    #[test]
    fn toolbar_default_is_empty() {
        assert!(Toolbar::new().is_empty());
    }

    // ---- TabBar ----------------------------------------------------------

    #[test]
    fn tabbar_initial_selection_is_first() {
        let tb = TabBar::new(["A", "B", "C"]);
        assert_eq!(tb.selected(), 0);
        assert_eq!(tb.label(0), "A");
    }

    #[test]
    fn tabbar_select_next_wraps() {
        let mut tb = TabBar::new(["A", "B", "C"]);
        tb.select(2);
        tb.select_next(); // wraps to 0
        assert_eq!(tb.selected(), 0);
    }

    #[test]
    fn tabbar_select_prev_wraps() {
        let mut tb = TabBar::new(["A", "B", "C"]);
        tb.select_prev(); // wraps from 0 to last
        assert_eq!(tb.selected(), 2);
    }

    #[test]
    fn tabbar_home_end() {
        let mut tb = TabBar::new(["A", "B", "C"]);
        tb.select(1);
        tb.select_last();
        assert_eq!(tb.selected(), 2);
        tb.select_first();
        assert_eq!(tb.selected(), 0);
    }

    #[test]
    fn tabbar_select_clamps_to_last() {
        let mut tb = TabBar::new(["A", "B"]);
        tb.select(99);
        assert_eq!(tb.selected(), 1);
    }
}
