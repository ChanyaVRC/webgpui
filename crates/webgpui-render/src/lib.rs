//! Renderer abstraction and draw-command types for webgpui.
//!
//! This crate is backend-independent.  It defines what *can* be drawn; the
//! actual GPU work is performed by backend implementations (`webgpui-render-wgpu`,
//! `webgpui-render-cuda`, etc.).
//!
//! This module also provides [`BackendSelector`] for runtime GPU backend switching,
//! allowing the application to choose between available backends (wgpu, CUDA, etc.)
//! at runtime.

mod backend;

pub use backend::BackendSelector;

use thiserror::Error;
use webgpui_geometry::{BorderRadius, Color, Rect};

// ---------------------------------------------------------------------------
// RenderError
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("surface lost; recreate the swap-chain")]
    SurfaceLost,
    #[error("GPU device lost: {0}")]
    DeviceLost(String),
    #[error("GPU timeout")]
    Timeout,
    #[error("backend not available; check feature flags or installed CUDA")]
    BackendUnavailable,
    #[error("GPU error: {0}")]
    Other(String),
}

pub type RenderResult<T> = Result<T, RenderError>;

// ---------------------------------------------------------------------------
// BlendMode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendMode {
    /// Standard alpha compositing (src-over).
    #[default]
    Alpha,
    /// Fully opaque; no blending.
    Opaque,
    /// Additive blending.
    Additive,
}

// ---------------------------------------------------------------------------
// DrawCommand
// ---------------------------------------------------------------------------

/// A single drawing instruction.
///
/// Commands are collected into a [`DrawList`] and submitted to the renderer
/// at the end of each frame.
#[derive(Debug, Clone)]
pub enum DrawCommand {
    /// Fill a solid-colour rectangle.
    FillRect {
        rect: Rect,
        color: Color,
        blend: BlendMode,
    },
    /// Fill a rounded rectangle.
    FillRoundedRect {
        rect: Rect,
        radius: BorderRadius,
        color: Color,
        blend: BlendMode,
    },
    /// Draw a rectangle border.
    DrawBorder {
        rect: Rect,
        color: Color,
        width: f32,
        radius: BorderRadius,
        blend: BlendMode,
    },
    /// Push a clipping rectangle.  Subsequent commands are clipped to `rect`.
    PushClip { rect: Rect },
    /// Pop the most recently pushed clipping rectangle.
    PopClip,
    /// Set the depth/z-order for subsequent commands.
    SetZOrder(u16),
}

// ---------------------------------------------------------------------------
// DrawList
// ---------------------------------------------------------------------------

/// An ordered list of [`DrawCommand`]s for one frame.
///
/// The list is cheap to clone and clear.
#[derive(Debug, Default, Clone)]
pub struct DrawList {
    commands: Vec<DrawCommand>,
    /// Current z-order, updated by [`DrawCommand::SetZOrder`].
    current_z: u16,
}

impl DrawList {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a command.
    pub fn push(&mut self, cmd: DrawCommand) {
        if let DrawCommand::SetZOrder(z) = cmd {
            self.current_z = z;
        }
        self.commands.push(cmd);
    }

    // ------------------------------------------------------------------
    // Convenience builders
    // ------------------------------------------------------------------

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.push(DrawCommand::FillRect { rect, color, blend: BlendMode::Alpha });
    }

    pub fn fill_rect_opaque(&mut self, rect: Rect, color: Color) {
        self.push(DrawCommand::FillRect { rect, color, blend: BlendMode::Opaque });
    }

    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: BorderRadius, color: Color) {
        self.push(DrawCommand::FillRoundedRect {
            rect,
            radius,
            color,
            blend: BlendMode::Alpha,
        });
    }

    pub fn draw_border(&mut self, rect: Rect, color: Color, width: f32) {
        self.push(DrawCommand::DrawBorder {
            rect,
            color,
            width,
            radius: BorderRadius::ZERO,
            blend: BlendMode::Alpha,
        });
    }

    pub fn push_clip(&mut self, rect: Rect) {
        self.push(DrawCommand::PushClip { rect });
    }

    pub fn pop_clip(&mut self) {
        self.push(DrawCommand::PopClip);
    }

    pub fn set_z(&mut self, z: u16) {
        self.push(DrawCommand::SetZOrder(z));
    }

    pub fn current_z(&self) -> u16 {
        self.current_z
    }

    // ------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------

    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Clears all commands, ready for the next frame.
    pub fn clear(&mut self) {
        self.commands.clear();
        self.current_z = 0;
    }
}

// ---------------------------------------------------------------------------
// Renderer trait
// ---------------------------------------------------------------------------

/// The interface implemented by a GPU rendering backend.
pub trait Renderer {
    /// Called once after the surface/swap-chain is created or re-created.
    fn resize(&mut self, width: u32, height: u32);

    /// Renders `draw_list` to the current swap-chain frame.
    fn render(&mut self, draw_list: &DrawList) -> RenderResult<()>;

    /// Returns the current surface size in physical pixels.
    fn surface_size(&self) -> (u32, u32);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use webgpui_geometry::Rect;

    #[test]
    fn draw_list_basics() {
        let mut dl = DrawList::new();
        assert!(dl.is_empty());
        dl.fill_rect(Rect::new(0.0, 0.0, 100.0, 50.0), Color::RED);
        assert_eq!(dl.len(), 1);
        dl.clear();
        assert!(dl.is_empty());
    }
}
