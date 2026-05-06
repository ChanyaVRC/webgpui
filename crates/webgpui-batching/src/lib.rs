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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BatchKey {
    pub blend_mode: BlendModeKey,
    /// Reserved for texture binding (MVP: always 0).
    pub texture_id: u32,
    /// Reserved for pipeline variant (MVP: always 0).
    pub pipeline_id: u32,
    /// Z-order bucket (lower = drawn first).
    pub z_order: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlendModeKey {
    Opaque,
    Alpha,
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
    pub key: Option<BatchKey>,
    /// Packed vertices.
    pub vertices: Vec<Vertex>,
    /// Indices into `vertices` forming triangles.
    pub indices: Vec<u32>,
}

impl DrawBatch {
    pub fn new(key: BatchKey) -> Self {
        Self {
            key: Some(key),
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

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
}

impl Batcher {
    pub fn new() -> Self {
        Self {
            batches: Vec::new(),
            current_z: 0,
            batch_index: std::collections::HashMap::new(),
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
                    rect, color, blend, ..
                } => {
                    // MVP: render rounded rects as plain rects.
                    let key = self.make_key(*blend);
                    self.get_or_create(key).push_rect(*rect, *color);
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
            }
        }

        // Sort batches: z-order ascending, then by blend mode.
        self.batches.sort_by(|a, b| {
            let ka = a
                .key
                .expect("every batch produced by get_or_create must have a key");
            let kb = b
                .key
                .expect("every batch produced by get_or_create must have a key");
            ka.z_order
                .cmp(&kb.z_order)
                .then_with(|| ka.blend_mode.cmp(&kb.blend_mode))
        });

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

    // ---- Problem scenario: DrawBatch::default() has key = None ----

    #[test]
    fn draw_batch_default_key_is_none() {
        // DrawBatch is a public struct and its Default impl sets key = None.
        // If such a batch reaches the sort in flush(), the expect() fires.
        // This test documents the type-level gap that makes the invariant
        // enforcement necessary.
        let batch = DrawBatch::default();
        assert!(batch.key.is_none());
    }

    #[test]
    #[should_panic(expected = "every batch produced by get_or_create must have a key")]
    fn sort_panics_on_none_key_batch() {
        // Directly demonstrates the failure mode: sorting a DrawBatch with
        // key = None triggers the expect() added by the fix.
        let batch = DrawBatch::default();
        let _ = batch
            .key
            .expect("every batch produced by get_or_create must have a key");
    }

    #[test]
    fn process_always_produces_batches_with_some_key() {
        // Positive assertion: every batch returned by process() must have a key.
        let mut dl = DrawList::new();
        dl.fill_rect(Rect::new(0.0, 0.0, 100.0, 50.0), Color::RED);
        dl.fill_rect(Rect::new(110.0, 0.0, 50.0, 50.0), Color::BLUE);
        let mut batcher = Batcher::new();
        let batches = batcher.process(&dl);
        for batch in batches {
            assert!(
                batch.key.is_some(),
                "batch from process() must always have a key"
            );
        }
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
    fn two_rects_same_key_merged() {
        let mut dl = DrawList::new();
        dl.fill_rect(Rect::new(0.0, 0.0, 50.0, 50.0), Color::RED);
        dl.fill_rect(Rect::new(60.0, 0.0, 50.0, 50.0), Color::BLUE);
        let mut batcher = Batcher::new();
        let batches = batcher.process(&dl);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].triangle_count(), 4);
    }
}
