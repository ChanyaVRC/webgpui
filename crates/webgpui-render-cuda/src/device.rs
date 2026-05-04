//! CUDA backend device module.
//!
//! This is currently a scaffold so workspace-wide tooling (`cargo fmt --all`)
//! can resolve the module graph even when the CUDA backend is feature-enabled.

/// CUDA renderer implementation placeholder.
#[derive(Debug, Default)]
pub struct CudaRenderer;

impl CudaRenderer {
    /// Creates a CUDA renderer.
    ///
    /// The concrete backend implementation is pending.
    pub fn new() -> Result<Self, &'static str> {
        Err("CUDA backend implementation is not yet available")
    }
}
