//! CUDA backend crate for webgpui.

// webgpui-render-cuda: CUDA GPU backend implementation
//
// This crate provides a CUDA-based GPU renderer implementation
// that implements the `webgpui_render::Renderer` trait for high-performance
// GPU rendering on NVIDIA hardware.
//
// ## Feature Flags
// - `backend-cuda`: Enable CUDA backend (required for this crate to be functional)
//
// ## CUDA Requirements
// - CUDA Toolkit 11.8 or newer (12.0+ recommended)
// - Compute Capability 3.5 or higher (Maxwell architecture or newer)
// - NVIDIA GPU driver compatible with CUDA version
// - Linux, Windows (via CUDA for Windows)
//
// ## Platform Support
// - **Linux (x86_64)**: Primary supported platform
// - **Windows (x86_64)**: Supported via CUDA for Windows
// - **macOS**: Not supported (NVIDIA CUDA unavailable)
//
// ## Architecture
// Similar to `webgpui-render-wgpu`, this crate:
// 1. Initializes CUDA device and context
// 2. Manages GPU memory (textures, buffers)
// 3. Compiles CUDA kernels (.cu files compiled to PTX)
// 4. Implements frame rendering loop with timestamp queries
// 5. Handles device synchronization and error recovery
//
// ## Design Rationale
// - CUDA offers fine-grained control over GPU compute for future optimization
// - Maintains abstraction boundary via `Renderer` trait (swap wgpu ↔ CUDA)
// - Both backends can coexist in same build (feature-gated)
// - Equivalence testing ensures output parity with wgpu backend

#![warn(missing_docs)]

#[cfg(feature = "backend-cuda")]
mod device;

#[cfg(feature = "backend-cuda")]
pub use device::CudaRenderer;

/// Placeholder type when CUDA feature is disabled
#[cfg(not(feature = "backend-cuda"))]
pub struct CudaRenderer;

#[cfg(not(feature = "backend-cuda"))]
impl CudaRenderer {
    /// CUDA support requires the `backend-cuda` feature flag
    pub fn new() -> Result<Self, &'static str> {
        Err("CUDA support disabled; enable 'backend-cuda' feature in Cargo.toml")
    }
}
