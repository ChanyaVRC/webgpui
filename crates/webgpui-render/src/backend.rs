//! Backend selection and factory for runtime backend switching.
//!
//! This module provides a `BackendSelector` enum and factory functions
//! to create a renderer at runtime, supporting wgpu, CUDA, and CPU backends.
//!
//! ## Feature Flags
//! - `backend-wgpu`: Enable wgpu backend (can be combined with other backends)
//! - `backend-cuda`: Enable CUDA backend (can be combined with other backends)
//! - `backend-cpu`: Enable CPU backend (can be combined with other backends)
//!
//! ## Usage
//!
//! ```ignore
//! use webgpui_render::BackendSelector;
//!
//! // Create a wgpu renderer
//! let renderer = BackendSelector::Wgpu.create(&window)?;
//!
//! // Or create a CUDA renderer (if compiled with backend-cuda feature)
//! #[cfg(feature = "backend-cuda")]
//! {
//!     let renderer = BackendSelector::Cuda.create(&window)?;
//! }
//!
//! // Or create a CPU renderer (if compiled with backend-cpu feature)
//! #[cfg(feature = "backend-cpu")]
//! {
//!     let renderer = BackendSelector::Cpu.create()?;
//! }
//!
//! // Or let the app choose at runtime
//! let backend = BackendSelector::from_str("cpu")?;
//! let renderer = backend.create(&window)?;
//! ```

use crate::RenderError;
use std::str::FromStr;

/// Backend selection enum for runtime backend switching.
///
/// This allows the application to select which GPU/rendering backend to use at runtime,
/// provided backends are compiled in (via feature flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendSelector {
    /// Use the wgpu backend (cross-platform, GPU-accelerated)
    Wgpu,
    /// Use the CUDA backend (NVIDIA GPU only)
    Cuda,
    /// Use the CPU backend (headless, no GPU)
    Cpu,
}

impl BackendSelector {
    /// Returns a list of available backends based on compile-time feature flags.
    ///
    /// # Example
    /// ```ignore
    /// let available = BackendSelector::available();
    /// println!("Available backends: {:?}", available);
    /// // Output: [BackendSelector::Wgpu] (or [BackendSelector::Wgpu, BackendSelector::Cuda, BackendSelector::Cpu] if all features enabled)
    /// ```
    pub fn available() -> Vec<Self> {
        #[allow(unused_mut)]
        let mut backends = Vec::new();

        #[cfg(feature = "backend-wgpu")]
        backends.push(BackendSelector::Wgpu);

        #[cfg(feature = "backend-cuda")]
        backends.push(BackendSelector::Cuda);

        #[cfg(feature = "backend-cpu")]
        backends.push(BackendSelector::Cpu);

        backends
    }

    /// Returns true if this backend is available (compiled in).
    pub fn is_available(&self) -> bool {
        match self {
            BackendSelector::Wgpu => {
                #[cfg(feature = "backend-wgpu")]
                {
                    true
                }
                #[cfg(not(feature = "backend-wgpu"))]
                {
                    false
                }
            }
            BackendSelector::Cuda => {
                #[cfg(feature = "backend-cuda")]
                {
                    true
                }
                #[cfg(not(feature = "backend-cuda"))]
                {
                    false
                }
            }
            BackendSelector::Cpu => {
                #[cfg(feature = "backend-cpu")]
                {
                    true
                }
                #[cfg(not(feature = "backend-cpu"))]
                {
                    false
                }
            }
        }
    }

    /// Returns the display name of this backend.
    pub fn name(&self) -> &'static str {
        match self {
            BackendSelector::Wgpu => "wgpu",
            BackendSelector::Cuda => "CUDA",
            BackendSelector::Cpu => "CPU",
        }
    }

    /// Creates a renderer instance for this backend.
    ///
    /// # Errors
    /// * [`RenderError::BackendUnavailable`] — the backend is not compiled in
    ///   (check [`is_available`][Self::is_available] first).
    /// * [`RenderError::Other`] — the backend requires a window handle or
    ///   additional integration; use `webgpui-app` for full setup.
    ///
    /// # Note
    /// This is a low-level factory stub.  wgpu and CUDA backends require a
    /// window handle that is not representable without a concrete type here.
    /// Prefer `webgpui_app::AppBuilder` for typical application use.
    pub fn create(&self) -> Result<Box<dyn crate::Renderer>, RenderError> {
        match self {
            #[cfg(feature = "backend-wgpu")]
            BackendSelector::Wgpu => {
                // This would call webgpui-render-wgpu's factory
                Err(RenderError::Other(
                    "wgpu renderer creation requires window context; use webgpui-app for full integration"
                        .to_string(),
                ))
            }

            #[cfg(not(feature = "backend-wgpu"))]
            BackendSelector::Wgpu => Err(RenderError::BackendUnavailable),

            #[cfg(feature = "backend-cuda")]
            BackendSelector::Cuda => {
                // This would call webgpui-render-cuda's factory
                Err(RenderError::Other(
                    "CUDA renderer creation requires window context; use webgpui-app for full integration"
                        .to_string(),
                ))
            }

            #[cfg(not(feature = "backend-cuda"))]
            BackendSelector::Cuda => Err(RenderError::BackendUnavailable),

            #[cfg(feature = "backend-cpu")]
            BackendSelector::Cpu => {
                // CPU renderer doesn't need window context
                // Note: This requires webgpui-render-cpu crate with a factory function
                Err(RenderError::Other(
                    "CPU renderer creation requires webgpui-render-cpu crate integration"
                        .to_string(),
                ))
            }

            #[cfg(not(feature = "backend-cpu"))]
            BackendSelector::Cpu => Err(RenderError::BackendUnavailable),
        }
    }
}

impl std::fmt::Display for BackendSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for BackendSelector {
    type Err = RenderError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("wgpu") {
            Ok(BackendSelector::Wgpu)
        } else if s.eq_ignore_ascii_case("cuda") {
            Ok(BackendSelector::Cuda)
        } else if s.eq_ignore_ascii_case("cpu") {
            Ok(BackendSelector::Cpu)
        } else {
            Err(RenderError::Other(format!(
                "unknown backend '{}'; available: {}",
                s,
                BackendSelector::available()
                    .iter()
                    .map(|b| b.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_selector_name() {
        assert_eq!(BackendSelector::Wgpu.name(), "wgpu");
        assert_eq!(BackendSelector::Cuda.name(), "CUDA");
        assert_eq!(BackendSelector::Cpu.name(), "CPU");
    }

    #[test]
    fn backend_selector_display() {
        assert_eq!(format!("{}", BackendSelector::Wgpu), "wgpu");
        assert_eq!(format!("{}", BackendSelector::Cuda), "CUDA");
        assert_eq!(format!("{}", BackendSelector::Cpu), "CPU");
    }

    #[test]
    #[cfg(any(
        feature = "backend-wgpu",
        feature = "backend-cuda",
        feature = "backend-cpu"
    ))]
    fn backend_selector_available() {
        let available = BackendSelector::available();
        assert!(!available.is_empty());
    }

    #[test]
    #[cfg(not(any(
        feature = "backend-wgpu",
        feature = "backend-cuda",
        feature = "backend-cpu"
    )))]
    fn backend_selector_available_empty_when_no_features() {
        assert!(BackendSelector::available().is_empty());
    }

    #[test]
    fn backend_selector_from_str() {
        assert_eq!(
            "wgpu".parse::<BackendSelector>().unwrap(),
            BackendSelector::Wgpu
        );
        assert_eq!(
            "cuda".parse::<BackendSelector>().unwrap(),
            BackendSelector::Cuda
        );
        assert_eq!(
            "cpu".parse::<BackendSelector>().unwrap(),
            BackendSelector::Cpu
        );
        assert_eq!(
            "WGPU".parse::<BackendSelector>().unwrap(),
            BackendSelector::Wgpu
        );
        assert!("invalid".parse::<BackendSelector>().is_err());
    }
}
