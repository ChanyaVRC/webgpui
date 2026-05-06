//! Compat/FastPath equivalence tests — api-mapping.md §12.
//!
//! Tests verify return values, side-effects on the core `NodeTree`, and error
//! variants for all MUST-tier functions.
//!
//! Because all functions share process-global state, each test must acquire
//! `TEST_LOCK` and call `reset_for_test()` before touching the API.

use std::sync::Mutex;

use crate::app::{app_mount, app_unmount, render_request, render_vsync, viewport_resize};
use crate::event::{event_on, event_stop_propagation, focus_set};
use crate::node::{node_append, node_create, node_remove, node_update};
use crate::state::{reset_for_test, with_state, with_tree};
use crate::style::{
    style_background, style_border, style_margin, style_opacity, style_padding, style_position,
    style_set, style_set_many, style_size,
};
use crate::types::{CompatError, EventType, NodeId, NodeKind};

// ---------------------------------------------------------------------------
// Test serialization helper
// ---------------------------------------------------------------------------

/// Global mutex serializing tests that share process-global CompatState.
static TEST_LOCK: Mutex<()> = Mutex::new(());

macro_rules! fresh {
    () => {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset_for_test();
    };
}

// ---------------------------------------------------------------------------
// node_create
// ---------------------------------------------------------------------------

#[test]
fn node_create_returns_ok() {
    fresh!();
    assert!(node_create(NodeKind::Container).is_ok());
}

#[test]
fn node_create_ids_are_unique() {
    fresh!();
    let a = node_create(NodeKind::Container).unwrap();
    let b = node_create(NodeKind::Text).unwrap();
    let c = node_create(NodeKind::Image).unwrap();
    assert_ne!(a, b);
    assert_ne!(b, c);
}

#[test]
fn node_create_leaves_node_staged() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    with_state(|s| {
        assert!(s.staged.contains_key(&id.0));
        assert!(!s.id_map.contains_key(&id.0));
    });
}

// ---------------------------------------------------------------------------
// node_append
// ---------------------------------------------------------------------------

#[test]
fn node_append_staged_to_staged_records_child() {
    fresh!();
    let parent = node_create(NodeKind::Container).unwrap();
    let child = node_create(NodeKind::Container).unwrap();
    assert!(node_append(parent, child).is_ok());
    with_state(|s| {
        assert!(s.staged[&parent.0].children.contains(&child.0));
    });
}

#[test]
fn node_append_invalid_child_errors() {
    fresh!();
    let parent = node_create(NodeKind::Container).unwrap();
    assert!(matches!(
        node_append(parent, NodeId(9999)),
        Err(CompatError::InvalidNode)
    ));
}

#[test]
fn node_append_invalid_parent_errors() {
    fresh!();
    let child = node_create(NodeKind::Container).unwrap();
    assert!(matches!(
        node_append(NodeId(9999), child),
        Err(CompatError::InvalidNode)
    ));
}

#[test]
fn node_append_after_mount_adds_to_core_tree() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    app_mount(root).unwrap();

    let extra = node_create(NodeKind::Text).unwrap();
    assert!(node_append(root, extra).is_ok());
    // core tree: implicit ROOT + root + extra = 3
    with_tree(|t| assert_eq!(t.len(), 3));
}

// ---------------------------------------------------------------------------
// node_remove
// ---------------------------------------------------------------------------

#[test]
fn node_remove_staged_node_ok() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    let child = node_create(NodeKind::Container).unwrap();
    node_append(root, child).unwrap();
    assert!(node_remove(root, child).is_ok());
    with_state(|s| assert!(!s.staged.contains_key(&child.0)));
}

#[test]
fn node_remove_mounted_node_ok() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    let child = node_create(NodeKind::Container).unwrap();
    node_append(root, child).unwrap();
    app_mount(root).unwrap();
    assert!(node_remove(root, child).is_ok());
    // core tree: implicit ROOT + root = 2
    with_tree(|t| assert_eq!(t.len(), 2));
}

#[test]
fn node_remove_invalid_errors() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    assert!(matches!(
        node_remove(root, NodeId(9999)),
        Err(CompatError::InvalidNode)
    ));
}

// ---------------------------------------------------------------------------
// node_update
// ---------------------------------------------------------------------------

#[test]
fn node_update_staged_ok() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(node_update(id, "text=hello").is_ok());
}

#[test]
fn node_update_mounted_ok() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    app_mount(root).unwrap();
    assert!(node_update(root, "").is_ok());
}

#[test]
fn node_update_invalid_errors() {
    fresh!();
    assert!(matches!(
        node_update(NodeId(9999), ""),
        Err(CompatError::InvalidNode)
    ));
}

// ---------------------------------------------------------------------------
// style_background
// ---------------------------------------------------------------------------

#[test]
fn style_background_hex6_sets_color() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_background(id, "#ff8000").is_ok());
    with_state(|s| {
        let bg = s.staged[&id.0].style.background;
        assert!((bg.r - 1.0).abs() < 1e-3);
        assert!((bg.g - 0x80 as f32 / 255.0).abs() < 1e-3);
        assert!((bg.b - 0.0).abs() < 1e-3);
    });
}

#[test]
fn style_background_invalid_color_errors() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(matches!(
        style_background(id, "red"),
        Err(CompatError::StyleParseError(_))
    ));
}

#[test]
fn style_background_invalid_node_errors() {
    fresh!();
    assert!(matches!(
        style_background(NodeId(9999), "#000000"),
        Err(CompatError::InvalidNode)
    ));
}

#[test]
fn style_background_mounted_node_updates_core() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    app_mount(root).unwrap();
    assert!(style_background(root, "#ffffff").is_ok());
    with_state(|s| {
        let core = s.core_id_of(root.0).unwrap();
        let bg = s.tree.get(core).unwrap().style.background;
        assert!((bg.r - 1.0).abs() < 1e-3);
        assert!((bg.g - 1.0).abs() < 1e-3);
        assert!((bg.b - 1.0).abs() < 1e-3);
    });
}

// ---------------------------------------------------------------------------
// style_position
// ---------------------------------------------------------------------------

#[test]
fn style_position_sets_absolute_xy() {
    fresh!();
    use webgpui_layout::PositionType;
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_position(id, 10.0, 20.0).is_ok());
    with_state(|s| {
        let layout = &s.staged[&id.0].layout;
        assert_eq!(layout.position, PositionType::Absolute);
        assert!((layout.x - 10.0).abs() < 1e-6);
        assert!((layout.y - 20.0).abs() < 1e-6);
    });
}

// ---------------------------------------------------------------------------
// style_size
// ---------------------------------------------------------------------------

#[test]
fn style_size_sets_width_height() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_size(id, Some(320.0), Some(240.0)).is_ok());
    with_state(|s| {
        let layout = &s.staged[&id.0].layout;
        assert_eq!(layout.width, Some(320.0));
        assert_eq!(layout.height, Some(240.0));
    });
}

#[test]
fn style_size_none_clears_dimension() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    style_size(id, Some(100.0), Some(100.0)).unwrap();
    assert!(style_size(id, None, None).is_ok());
    with_state(|s| {
        let layout = &s.staged[&id.0].layout;
        assert_eq!(layout.width, None);
        assert_eq!(layout.height, None);
    });
}

// ---------------------------------------------------------------------------
// style_margin / style_padding
// ---------------------------------------------------------------------------

#[test]
fn style_margin_sets_all_sides() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_margin(id, 1.0, 2.0, 3.0, 4.0).is_ok());
    with_state(|s| {
        let m = s.staged[&id.0].layout.margin;
        assert!((m.left - 1.0).abs() < 1e-6);
        assert!((m.top - 2.0).abs() < 1e-6);
        assert!((m.right - 3.0).abs() < 1e-6);
        assert!((m.bottom - 4.0).abs() < 1e-6);
    });
}

#[test]
fn style_padding_sets_all_sides() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_padding(id, 4.0, 8.0, 4.0, 8.0).is_ok());
    with_state(|s| {
        let p = s.staged[&id.0].layout.padding;
        assert!((p.left - 4.0).abs() < 1e-6);
        assert!((p.top - 8.0).abs() < 1e-6);
    });
}

// ---------------------------------------------------------------------------
// style_border
// ---------------------------------------------------------------------------

#[test]
fn style_border_sets_width_and_color() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_border(id, 2.0, "#0080ff").is_ok());
    with_state(|s| {
        let st = &s.staged[&id.0].style;
        assert!((st.border.left - 2.0).abs() < 1e-6);
        assert!((st.border_color.b - 1.0).abs() < 1e-3);
    });
}

// ---------------------------------------------------------------------------
// style_opacity
// ---------------------------------------------------------------------------

#[test]
fn style_opacity_sets_value() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_opacity(id, 0.5).is_ok());
    with_state(|s| {
        assert!((s.staged[&id.0].style.opacity - 0.5).abs() < 1e-6);
    });
}

#[test]
fn style_opacity_clamps_below_zero() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_opacity(id, -1.0).is_ok());
    with_state(|s| {
        assert!((s.staged[&id.0].style.opacity - 0.0).abs() < 1e-6);
    });
}

#[test]
fn style_opacity_clamps_above_one() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_opacity(id, 2.0).is_ok());
    with_state(|s| {
        assert!((s.staged[&id.0].style.opacity - 1.0).abs() < 1e-6);
    });
}

// ---------------------------------------------------------------------------
// style_set / style_set_many
// ---------------------------------------------------------------------------

#[test]
fn style_set_background_key() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_set(id, "background", "#123456").is_ok());
}

#[test]
fn style_set_opacity_key() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_set(id, "opacity", "0.75").is_ok());
    with_state(|s| {
        assert!((s.staged[&id.0].style.opacity - 0.75).abs() < 1e-4);
    });
}

#[test]
fn style_set_unknown_key_errors() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(matches!(
        style_set(id, "unknown-prop", "value"),
        Err(CompatError::StyleParseError(_))
    ));
}

#[test]
fn style_set_many_applies_all() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(style_set_many(
        id,
        &[("width", "200"), ("height", "100"), ("opacity", "0.8")]
    )
    .is_ok());
    with_state(|s| {
        assert_eq!(s.staged[&id.0].layout.width, Some(200.0));
        assert_eq!(s.staged[&id.0].layout.height, Some(100.0));
        assert!((s.staged[&id.0].style.opacity - 0.8).abs() < 1e-4);
    });
}

#[test]
fn style_set_many_stops_on_first_error() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    let result = style_set_many(id, &[("width", "100"), ("bad-prop", "x"), ("height", "50")]);
    assert!(result.is_err());
    // width was applied before the error
    with_state(|s| assert_eq!(s.staged[&id.0].layout.width, Some(100.0)));
}

// ---------------------------------------------------------------------------
// event_on
// ---------------------------------------------------------------------------

#[test]
fn event_on_staged_node_ok() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    let result = event_on(id, EventType::Click, Box::new(|| {}));
    assert!(result.is_ok());
}

#[test]
fn event_on_mounted_node_ok() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    app_mount(root).unwrap();
    let result = event_on(root, EventType::Focus, Box::new(|| {}));
    assert!(result.is_ok());
}

#[test]
fn event_on_invalid_node_errors() {
    fresh!();
    assert!(matches!(
        event_on(NodeId(9999), EventType::Click, Box::new(|| {})),
        Err(CompatError::InvalidNode)
    ));
}

#[test]
fn event_on_listener_ids_are_unique() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    let l1 = event_on(id, EventType::Click, Box::new(|| {})).unwrap();
    let l2 = event_on(id, EventType::KeyDown, Box::new(|| {})).unwrap();
    assert_ne!(l1, l2);
}

// ---------------------------------------------------------------------------
// event_stop_propagation
// ---------------------------------------------------------------------------

#[test]
fn event_stop_propagation_always_ok() {
    fresh!();
    assert!(event_stop_propagation().is_ok());
}

// ---------------------------------------------------------------------------
// focus_set
// ---------------------------------------------------------------------------

#[test]
fn focus_set_staged_node_ok() {
    fresh!();
    let id = node_create(NodeKind::Container).unwrap();
    assert!(focus_set(id).is_ok());
}

#[test]
fn focus_set_mounted_node_ok() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    app_mount(root).unwrap();
    assert!(focus_set(root).is_ok());
}

#[test]
fn focus_set_invalid_errors() {
    fresh!();
    assert!(matches!(
        focus_set(NodeId(9999)),
        Err(CompatError::InvalidNode)
    ));
}

// ---------------------------------------------------------------------------
// app_mount
// ---------------------------------------------------------------------------

#[test]
fn app_mount_flushes_tree() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    let child = node_create(NodeKind::Text).unwrap();
    node_append(root, child).unwrap();
    assert!(app_mount(root).is_ok());
    // implicit ROOT + root + child = 3
    with_tree(|t| assert_eq!(t.len(), 3));
}

#[test]
fn app_mount_nested_tree() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    let a = node_create(NodeKind::Container).unwrap();
    let b = node_create(NodeKind::Text).unwrap();
    let c = node_create(NodeKind::Image).unwrap();
    node_append(root, a).unwrap();
    node_append(a, b).unwrap();
    node_append(a, c).unwrap();
    app_mount(root).unwrap();
    // implicit ROOT + root + a + b + c = 5
    with_tree(|t| assert_eq!(t.len(), 5));
}

#[test]
fn app_mount_invalid_node_errors() {
    fresh!();
    assert!(matches!(
        app_mount(NodeId(9999)),
        Err(CompatError::InvalidNode)
    ));
}

#[test]
fn app_mount_twice_errors() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    app_mount(root).unwrap();
    let root2 = node_create(NodeKind::Container).unwrap();
    assert!(matches!(
        app_mount(root2),
        Err(CompatError::InternalError(_))
    ));
}

#[test]
fn app_mount_preserves_styles() {
    fresh!();
    let root = node_create(NodeKind::Container).unwrap();
    style_background(root, "#102030").unwrap();
    style_size(root, Some(800.0), Some(600.0)).unwrap();
    app_mount(root).unwrap();
    with_state(|s| {
        let core = s.core_id_of(root.0).unwrap();
        let node = s.tree.get(core).unwrap();
        assert!((node.style.background.r - 0x10 as f32 / 255.0).abs() < 1e-3);
        assert_eq!(node.layout.width, Some(800.0));
        assert_eq!(node.layout.height, Some(600.0));
    });
}

// ---------------------------------------------------------------------------
// app_unmount / render_request / render_vsync / viewport_resize
// ---------------------------------------------------------------------------

#[test]
fn app_unmount_ok() {
    fresh!();
    assert!(app_unmount().is_ok());
}

#[test]
fn render_request_sets_flag() {
    fresh!();
    with_state(|s| s.render_requested = false);
    render_request().unwrap();
    with_state(|s| assert!(s.render_requested));
}

#[test]
fn render_vsync_toggle() {
    fresh!();
    render_vsync(false).unwrap();
    with_state(|s| assert!(!s.vsync));
    render_vsync(true).unwrap();
    with_state(|s| assert!(s.vsync));
}

#[test]
fn viewport_resize_updates_dimensions() {
    fresh!();
    viewport_resize(1920, 1080).unwrap();
    with_state(|s| assert_eq!(s.viewport, (1920, 1080)));
}

#[test]
fn viewport_resize_requests_render() {
    fresh!();
    with_state(|s| s.render_requested = false);
    viewport_resize(800, 600).unwrap();
    with_state(|s| assert!(s.render_requested));
}
