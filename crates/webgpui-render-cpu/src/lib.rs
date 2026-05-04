//! CPU-based headless renderer for webgpui
//!
//! This crate provides a CPU software renderer that implements the `Renderer` trait
//! without GPU acceleration. Primarily used for:
//! - Testing and validation
//! - Headless rendering (no window display)
//! - Performance profiling (CPU path only, no GPU overhead)
//! - Debugging and diagnostics
//!
//! ## Feature Flags
//! - `backend-cpu`: Enable CPU backend
//!
//! ## Implementation Notes
//! - No actual GPU usage; all draw commands are counted but not rendered
//! - Can be used for benchmarking CPU-side rendering pipeline
//! - Surface is virtual; no window or framebuffer
//! - Useful for CI/testing environments without GPU

#![warn(missing_docs)]

use webgpui_render::{DrawList, RenderResult, Renderer};

/// CPU-based headless renderer
///
/// This renderer accepts draw commands via the `Renderer` trait but does not
/// actually render to GPU or display. It's useful for:
/// - Testing rendering pipeline without GPU
/// - Measuring CPU-side performance
/// - Headless batch processing
/// - CI environments without GPU
///
/// All draw operations complete successfully but produce no visual output.
#[derive(Debug)]
pub struct CpuRenderer {
    surface_width: u32,
    surface_height: u32,
    frame_count: u64,
    total_commands: u64,
}

impl CpuRenderer {
    /// Creates a new CPU renderer with the specified surface size.
    ///
    /// # Arguments
    /// * `width` - Surface width in pixels
    /// * `height` - Surface height in pixels
    ///
    /// # Example
    /// ```ignore
    /// let renderer = CpuRenderer::new(800, 600)?;
    /// ```
    pub fn new(width: u32, height: u32) -> RenderResult<Self> {
        log::info!("CPU renderer initialized: {}x{} (headless)", width, height);
        Ok(Self {
            surface_width: width,
            surface_height: height,
            frame_count: 0,
            total_commands: 0,
        })
    }

    /// Returns the number of frames rendered so far.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Returns the total number of draw commands processed.
    pub fn total_commands(&self) -> u64 {
        self.total_commands
    }
}

impl Renderer for CpuRenderer {
    fn resize(&mut self, width: u32, height: u32) {
        self.surface_width = width;
        self.surface_height = height;
        log::debug!("CPU renderer resized to {}x{}", width, height);
    }

    fn render(&mut self, draw_list: &DrawList) -> RenderResult<()> {
        self.frame_count += 1;
        self.total_commands += draw_list.len() as u64;

        // Headless rendering: count commands but don't actually render
        if log::log_enabled!(log::Level::Trace) {
            log::trace!(
                "CPU render frame {}: {} commands",
                self.frame_count,
                draw_list.len()
            );
        }

        Ok(())
    }

    fn surface_size(&self) -> (u32, u32) {
        (self.surface_width, self.surface_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_renderer_creation() {
        let renderer = CpuRenderer::new(800, 600).unwrap();
        assert_eq!(renderer.surface_size(), (800, 600));
        assert_eq!(renderer.frame_count(), 0);
    }

    #[test]
    fn cpu_renderer_resize() {
        let mut renderer = CpuRenderer::new(800, 600).unwrap();
        renderer.resize(1024, 768);
        assert_eq!(renderer.surface_size(), (1024, 768));
    }

    #[test]
    fn cpu_renderer_frame_counting() {
        let mut renderer = CpuRenderer::new(800, 600).unwrap();
        let draw_list = DrawList::new();
        renderer.render(&draw_list).unwrap();
        assert_eq!(renderer.frame_count(), 1);
    }
}
