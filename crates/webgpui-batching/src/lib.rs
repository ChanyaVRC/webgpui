//! Draw-call batching and vertex packing for webgpui.
//!
//! The [`Batcher`] consumes a [`DrawList`] and produces a small number of
//! [`DrawBatch`]es.  Commands that share the same [`BatchKey`] (pipeline,
//! blend mode) are merged into one batch, minimising GPU draw calls.
//!
//! # Pipeline
//! ```text
//! DrawList  →  Batcher::process()  →  Vec<DrawBatch>  →  GPU
//! ```

use bytemuck::{Pod, Zeroable};
use webgpui_geometry::{Color, Rect};
use webgpui_render::{BlendMode, DrawCommand, DrawList};

// ---------------------------------------------------------------------------
// Vertex
// ---------------------------------------------------------------------------

/// A single GPU vertex (position in pixel space + colour).
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    /// Position in *logical* pixel space (x-right, y-down).
    pub position: [f32; 2],
    /// RGBA colour.
    pub color: [f32; 4],
}

impl Vertex {
    /// Constructs a vertex at logical-pixel position `(x, y)` with the given colour.
    pub fn new(x: f32, y: f32, color: Color) -> Self {
        Self {
            position: [x, y],
            color: color.to_array(),
        }
    }
}

/// Byte stride of a single `Vertex`.
pub const VERTEX_SIZE: u64 = std::mem::size_of::<Vertex>() as u64;

// ---------------------------------------------------------------------------
// BatchKey
// ---------------------------------------------------------------------------

/// Groups draw calls that can be merged into a single GPU draw command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BatchKey {
    pub blend_mode: BlendModeKey,
    /// Reserved for texture binding (MVP: always 0).
    pub texture_id: u32,
    /// Reserved for pipeline variant (MVP: always 0).
    pub pipeline_id: u32,
    /// Z-order bucket (lower = drawn first).
    pub z_order: u16,
}

/// Blend mode encoded as a key-safe value (implements `Ord`/`Hash` for use in [`BatchKey`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum BlendModeKey {
    /// No blending; source colour replaces destination entirely.
    #[default]
    Opaque,
    /// Standard alpha blending (`src_alpha * src + (1 - src_alpha) * dst`).
    Alpha,
    /// Additive blending (`src + dst`); brightens overlapping regions.
    Additive,
}

impl From<BlendMode> for BlendModeKey {
    fn from(m: BlendMode) -> Self {
        match m {
            BlendMode::Opaque => BlendModeKey::Opaque,
            BlendMode::Alpha => BlendModeKey::Alpha,
            BlendMode::Additive => BlendModeKey::Additive,
        }
    }
}

// ---------------------------------------------------------------------------
// DrawBatch
// ---------------------------------------------------------------------------

/// A finalised batch ready to be uploaded to the GPU.
#[derive(Debug, Default, Clone)]
pub struct DrawBatch {
    pub key: BatchKey,
    /// Packed vertices.
    pub vertices: Vec<Vertex>,
    /// Indices into `vertices` forming triangles.
    pub indices: Vec<u32>,
}

impl DrawBatch {
    /// Creates an empty batch for the given [`BatchKey`].
    pub fn new(key: BatchKey) -> Self {
        Self {
            key,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Number of triangles (index count / 3).
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Returns `true` when the batch contains no vertices.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Appends a quad (two triangles, 4 vertices) to the batch.
    ///
    /// `rect` is in logical pixel space.  `color` is the fill colour.
    pub fn push_rect(&mut self, rect: Rect, color: Color) {
        let base = self.vertices.len() as u32;
        let (x0, y0, x1, y1) = (rect.min_x(), rect.min_y(), rect.max_x(), rect.max_y());
        self.vertices.extend_from_slice(&[
            Vertex::new(x0, y0, color), // top-left
            Vertex::new(x1, y0, color), // top-right
            Vertex::new(x1, y1, color), // bottom-right
            Vertex::new(x0, y1, color), // bottom-left
        ]);
        // Two CCW triangles.
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }

    /// Appends a rounded rectangle using CPU-side corner fans.
    ///
    /// Each corner arc is approximated with `SEGS` triangle segments.
    /// If all corner radii are zero the call is equivalent to [`push_rect`](Self::push_rect).
    /// Non-uniform radii (different per corner) are supported; each corner is
    /// independently clamped to fit within the rect.
    pub fn push_rounded_rect(
        &mut self,
        rect: Rect,
        radius: webgpui_geometry::BorderRadius,
        color: Color,
    ) {
        const SEGS: usize = 8; // triangles per quarter-arc

        let half_w = rect.size.width * 0.5;
        let half_h = rect.size.height * 0.5;
        let max_r = half_w.min(half_h).max(0.0);

        let tl = radius.top_left.clamp(0.0, max_r);
        let tr = radius.top_right.clamp(0.0, max_r);
        let br = radius.bottom_right.clamp(0.0, max_r);
        let bl = radius.bottom_left.clamp(0.0, max_r);

        if tl == 0.0 && tr == 0.0 && br == 0.0 && bl == 0.0 {
            return self.push_rect(rect, color);
        }

        let x0 = rect.min_x();
        let y0 = rect.min_y();
        let x1 = rect.max_x();
        let y1 = rect.max_y();

        use std::f32::consts::PI;
        // (cx, cy, r, start_angle) for each corner — angles in radians, CCW from +x axis.
        // In screen space (y-down): TL=180°→270°, TR=270°→360°, BR=0°→90°, BL=90°→180°.
        let corner_defs: [(f32, f32, f32, f32); 4] = [
            (x0 + tl, y0 + tl, tl, PI),             // top-left:     180°→270°
            (x1 - tr, y0 + tr, tr, 3.0 * PI / 2.0), // top-right:    270°→360°
            (x1 - br, y1 - br, br, 0.0),            // bottom-right: 0°→90°
            (x0 + bl, y1 - bl, bl, PI / 2.0),       // bottom-left:  90°→180°
        ];

        let mut arc_pts: [Vec<[f32; 2]>; 4] = [vec![], vec![], vec![], vec![]];

        for (i, &(cx, cy, r, start)) in corner_defs.iter().enumerate() {
            let pts = &mut arc_pts[i];
            pts.clear();
            if r == 0.0 {
                // Degenerate: single point at the sharp corner.
                let px = match i {
                    0 | 3 => x0,
                    _ => x1,
                };
                let py = match i {
                    0 | 1 => y0,
                    _ => y1,
                };
                pts.push([px, py]);
            } else {
                for s in 0..=SEGS {
                    let angle = start + (PI / 2.0) * s as f32 / SEGS as f32;
                    pts.push([cx + r * angle.cos(), cy + r * angle.sin()]);
                }
            }
        }

        // Collect all outline points in order (TL arc → TR arc → BR arc → BL arc),
        // skipping the last point of each arc (it equals the first of the next).
        let mut outline: Vec<[f32; 2]> = Vec::new();
        for pts in &arc_pts {
            outline.extend_from_slice(&pts[..pts.len().saturating_sub(1)]);
        }
        let n = outline.len();
        if n < 3 {
            return self.push_rect(rect, color);
        }

        // Emit as a triangle fan from the centroid.
        let cx = (x0 + x1) * 0.5;
        let cy = (y0 + y1) * 0.5;
        let base = self.vertices.len() as u32;

        // Center vertex.
        self.vertices.push(Vertex::new(cx, cy, color));
        // Outline vertices.
        for &[px, py] in &outline {
            self.vertices.push(Vertex::new(px, py, color));
        }
        // Triangles: center (base), outline[i] (base+1+i), outline[(i+1)%n] (base+1+(i+1)%n).
        for i in 0..n as u32 {
            let next = (i + 1) % n as u32;
            self.indices
                .extend_from_slice(&[base, base + 1 + i, base + 1 + next]);
        }
    }

    /// Appends a rectangular border as four quads (top, right, bottom, left).
    pub fn push_border(&mut self, rect: Rect, color: Color, width: f32) {
        let w = width.max(0.0);
        if w == 0.0 {
            return;
        }
        // Top
        self.push_rect(
            Rect::new(rect.min_x(), rect.min_y(), rect.size.width, w),
            color,
        );
        // Bottom
        self.push_rect(
            Rect::new(rect.min_x(), rect.max_y() - w, rect.size.width, w),
            color,
        );
        // Left (exclude corners)
        self.push_rect(
            Rect::new(
                rect.min_x(),
                rect.min_y() + w,
                w,
                rect.size.height - 2.0 * w,
            ),
            color,
        );
        // Right
        self.push_rect(
            Rect::new(
                rect.max_x() - w,
                rect.min_y() + w,
                w,
                rect.size.height - 2.0 * w,
            ),
            color,
        );
    }
}

// ---------------------------------------------------------------------------
// Batcher
// ---------------------------------------------------------------------------

/// Accumulates [`DrawCommand`]s and packs them into [`DrawBatch`]es.
pub struct Batcher {
    batches: Vec<DrawBatch>,
    current_z: u16,
    /// Maps [`BatchKey`] → index into `batches` for O(1) lookup.
    batch_index: std::collections::HashMap<BatchKey, usize>,
    /// Set when a new batch is created; cleared after sorting. Avoids re-sorting
    /// frames where the batch order cannot have changed.
    sort_dirty: bool,
}

impl Batcher {
    pub fn new() -> Self {
        Self {
            batches: Vec::new(),
            current_z: 0,
            batch_index: std::collections::HashMap::new(),
            sort_dirty: false,
        }
    }

    /// Processes a [`DrawList`] and returns an ordered `Vec<DrawBatch>`.
    ///
    /// The returned list is sorted by z-order (ascending) then by batch key
    /// to minimise pipeline / blend state switches.
    pub fn process(&mut self, draw_list: &DrawList) -> &[DrawBatch] {
        self.batches.clear();
        self.batch_index.clear();
        self.current_z = 0;
        self.sort_dirty = false;

        for cmd in draw_list.commands() {
            match cmd {
                DrawCommand::SetZOrder(z) => {
                    self.current_z = *z;
                }
                DrawCommand::FillRect { rect, color, blend } => {
                    let key = self.make_key(*blend);
                    self.get_or_create(key).push_rect(*rect, *color);
                }
                DrawCommand::FillRoundedRect {
                    rect,
                    color,
                    radius,
                    blend,
                } => {
                    let key = self.make_key(*blend);
                    self.get_or_create(key)
                        .push_rounded_rect(*rect, *radius, *color);
                }
                DrawCommand::DrawBorder {
                    rect,
                    color,
                    width,
                    blend,
                    ..
                } => {
                    let key = self.make_key(*blend);
                    self.get_or_create(key).push_border(*rect, *color, *width);
                }
                DrawCommand::PushClip { .. } | DrawCommand::PopClip => {
                    // MVP: clipping not yet implemented at the batch level.
                }
                DrawCommand::DrawImage { .. } => {
                    // Images bypass the batcher and are rendered directly by the backend.
                }
            }
        }

        // Sort batches: z-order ascending, then by blend mode.
        // Skip sort when there is at most one batch — nothing to reorder.
        if self.sort_dirty && self.batches.len() > 1 {
            self.batches.sort_by(|a, b| {
                a.key
                    .z_order
                    .cmp(&b.key.z_order)
                    .then_with(|| a.key.blend_mode.cmp(&b.key.blend_mode))
            });
            self.sort_dirty = false;
        }

        &self.batches
    }

    fn make_key(&self, blend: BlendMode) -> BatchKey {
        BatchKey {
            blend_mode: blend.into(),
            texture_id: 0,
            pipeline_id: 0,
            z_order: self.current_z,
        }
    }

    fn get_or_create(&mut self, key: BatchKey) -> &mut DrawBatch {
        if let Some(&pos) = self.batch_index.get(&key) {
            return &mut self.batches[pos];
        }
        // Mark sort needed only when the new batch would be out of ascending key order.
        if !self.sort_dirty {
            if let Some(last) = self.batches.last() {
                let out_of_order = last
                    .key
                    .z_order
                    .cmp(&key.z_order)
                    .then_with(|| last.key.blend_mode.cmp(&key.blend_mode))
                    .is_gt();
                if out_of_order {
                    self.sort_dirty = true;
                }
            }
        }
        let pos = self.batches.len();
        self.batches.push(DrawBatch::new(key));
        self.batch_index.insert(key, pos);
        &mut self.batches[pos]
    }

    /// Returns the number of GPU triangles that will be submitted.
    pub fn total_triangles(&self) -> usize {
        self.batches.iter().map(|b| b.triangle_count()).sum()
    }
}

impl Default for Batcher {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use webgpui_geometry::Color;
    use webgpui_render::DrawList;

    // ---- Before fix: Option<BatchKey> + unwrap() panics on default batch ----

    #[test]
    #[should_panic]
    fn before_fix_sort_panics_when_batch_has_no_key() {
        // Before the fix, DrawBatch.key was Option<BatchKey> and flush() called
        // unwrap(). A default-constructed DrawBatch (key = None) would panic here.
        // This replicates the exact sort closure that was in flush() before the fix.
        let mut batches: Vec<(Option<BatchKey>, u16)> = vec![(None, 0), (None, 1)]; // same shape as old DrawBatch
        batches.sort_by(|a, b| {
            let ka = a.0.unwrap(); // old code — panics on None
            let kb = b.0.unwrap();
            ka.z_order.cmp(&kb.z_order)
        });
    }

    // ---- After fix: BatchKey is non-optional, Batcher::process() sort is safe ----

    #[test]
    fn after_fix_batcher_sorts_by_z_order_via_process() {
        // After fix: DrawBatch.key is BatchKey (not Option). Batcher::process()
        // sort never needs unwrap. Drive entirely through the public API.
        let mut dl = DrawList::new();
        dl.set_z(20);
        dl.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::RED);
        dl.set_z(5);
        dl.fill_rect(Rect::new(20.0, 0.0, 10.0, 10.0), Color::BLUE);
        let mut batcher = Batcher::new();
        let batches = batcher.process(&dl);
        // z=5 batch must come first; no unwrap anywhere in this path.
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].key.z_order, 5);
        assert_eq!(batches[1].key.z_order, 20);
    }

    #[test]
    fn single_rect() {
        let mut dl = DrawList::new();
        dl.fill_rect(Rect::new(0.0, 0.0, 100.0, 50.0), Color::RED);
        let mut batcher = Batcher::new();
        let batches = batcher.process(&dl);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].triangle_count(), 2);
    }

    #[test]
    fn batcher_skips_sort_for_single_batch() {
        // All rects at z=0 → one batch → sort must not be called (behavioral test: result is correct)
        let mut dl = DrawList::new();
        dl.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::RED);
        dl.fill_rect(Rect::new(20.0, 0.0, 10.0, 10.0), Color::BLUE);
        let mut batcher = Batcher::new();
        let batches = batcher.process(&dl);
        // One merged batch, z=0
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].key.z_order, 0);
    }

    #[test]
    fn two_rects_same_key_merged() {
        let mut dl = DrawList::new();
        dl.fill_rect(Rect::new(0.0, 0.0, 50.0, 50.0), Color::RED);
        dl.fill_rect(Rect::new(60.0, 0.0, 50.0, 50.0), Color::BLUE);
        let mut batcher = Batcher::new();
        let batches = batcher.process(&dl);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].triangle_count(), 4);
    }

    #[test]
    fn batcher_ignores_draw_image_produces_no_geometry() {
        // DrawImage bypasses the batcher entirely — no color geometry should be emitted.
        let mut dl = DrawList::new();
        dl.draw_image(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            webgpui_render::ImageHandle(0),
        );
        let mut batcher = Batcher::new();
        let batches = batcher.process(&dl);
        let total: usize = batches.iter().map(|b| b.triangle_count()).sum();
        assert_eq!(total, 0);
    }

    #[test]
    fn batcher_draw_image_does_not_affect_color_geometry() {
        // DrawImage interspersed with color rects must not disturb color batching.
        let mut dl = DrawList::new();
        dl.fill_rect(Rect::new(0.0, 0.0, 50.0, 50.0), Color::RED);
        dl.draw_image(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            webgpui_render::ImageHandle(0),
        );
        dl.fill_rect(Rect::new(60.0, 0.0, 50.0, 50.0), Color::BLUE);
        let mut batcher = Batcher::new();
        let batches = batcher.process(&dl);
        // Two color rects → merged into one batch of 4 triangles.
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].triangle_count(), 4);
    }

    #[test]
    fn rounded_rect_zero_radius_equals_rect() {
        use webgpui_geometry::{BorderRadius, Point, Rect, Size};
        let mut plain = DrawBatch::new(BatchKey::default());
        let mut rounded = DrawBatch::new(BatchKey::default());
        let rect = Rect {
            origin: Point::new(10.0, 20.0),
            size: Size::new(100.0, 50.0),
        };
        plain.push_rect(rect, Color::WHITE);
        rounded.push_rounded_rect(rect, BorderRadius::ZERO, Color::WHITE);
        assert_eq!(plain.vertices.len(), rounded.vertices.len());
        assert_eq!(plain.indices.len(), rounded.indices.len());
    }

    #[test]
    fn rounded_rect_nonzero_radius_has_more_verts() {
        use webgpui_geometry::{BorderRadius, Point, Rect, Size};
        let mut b = DrawBatch::new(BatchKey::default());
        let rect = Rect {
            origin: Point::new(0.0, 0.0),
            size: Size::new(100.0, 100.0),
        };
        b.push_rounded_rect(rect, BorderRadius::all(10.0), Color::RED);
        // Fan from center + arc points; must have more vertices than a plain quad (4).
        assert!(b.vertices.len() > 4);
        assert!(b.triangle_count() > 2);
    }

    #[test]
    fn rounded_rect_radius_clamped_to_half_size() {
        use webgpui_geometry::{BorderRadius, Point, Rect, Size};
        let mut b = DrawBatch::new(BatchKey::default());
        let rect = Rect {
            origin: Point::new(0.0, 0.0),
            size: Size::new(40.0, 40.0),
        };
        // Radius larger than half the rect — must not panic, must produce valid geometry.
        b.push_rounded_rect(rect, BorderRadius::all(9999.0), Color::BLUE);
        assert!(b.triangle_count() > 0);
    }
}
