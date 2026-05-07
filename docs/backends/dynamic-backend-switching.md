# Dynamic GPU Backend Switching

## Overview

WebGPUI supports **runtime backend selection**, allowing applications to choose between wgpu and CUDA backends dynamically, provided both are compiled in.

This enables:
- **Fallback strategies**: Prefer CUDA, fall back to wgpu if unavailable
- **User selection**: CLI flags, environment variables, or config files to choose backend
- **Benchmarking**: Compare performance across backends on the same hardware
- **Graceful degradation**: Detect GPU unavailability and switch automatically
- **Hardware detection**: Detect NVIDIA GPU and use CUDA; fall back to wgpu otherwise

## Compile-Time Feature Selection

To enable dynamic switching, compile both backends into your binary:

```bash
# Enable both backends for runtime selection
cargo build --features backend-wgpu,backend-cuda

# Or in Cargo.toml
[dependencies]
webgpui = { version = "0.1", features = ["backend-wgpu", "backend-cuda"] }
```

To use only one backend (smaller binary):

```bash
# Default: wgpu only
cargo build

# Or explicit single backend
cargo build --features backend-cuda  # CUDA only
cargo build --features backend-wgpu  # wgpu only
```

## Runtime Backend Detection API

### Check Available Backends

```rust
use webgpui_render::BackendSelector;

// List all compiled-in backends
let available = BackendSelector::available();
for backend in &available {
    println!("Available: {} ({})", backend.name(), backend);
}

// Output (with both features):
// Available: wgpu (wgpu)
// Available: CUDA (CUDA)
```

### Check Specific Backend Availability

```rust
use webgpui_render::BackendSelector;

if BackendSelector::Cuda.is_available() {
    println!("CUDA backend is available");
} else {
    println!("CUDA backend not compiled or unavailable");
}

if BackendSelector::Wgpu.is_available() {
    println!("wgpu backend is available");
}
```

### Detect Hardware and Select Backend

```rust
use webgpui_render::BackendSelector;

// Example: detect NVIDIA GPU and prefer CUDA
fn select_backend() -> BackendSelector {
    // Check for NVIDIA GPU (pseudocode; actual implementation depends on system)
    if has_nvidia_gpu() && BackendSelector::Cuda.is_available() {
        BackendSelector::Cuda
    } else if BackendSelector::Wgpu.is_available() {
        BackendSelector::Wgpu
    } else {
        panic!("No GPU backend available!");
    }
}

fn has_nvidia_gpu() -> bool {
    // Implementation: check nvidia-smi or CUDA runtime
    // For MVP, assume true if user requests CUDA
    true
}
```

## Application-Level Backend Selection

### Example: CLI Argument

```rust
use std::env;
use webgpui_render::BackendSelector;

fn main() {
    // Parse --backend=cuda or --backend=wgpu
    let backend = env::args()
        .find(|arg| arg.starts_with("--backend="))
        .and_then(|arg| {
            match &arg[10..] {
                "cuda" => Some(BackendSelector::Cuda),
                "wgpu" => Some(BackendSelector::Wgpu),
                _ => None,
            }
        })
        .unwrap_or_else(|| {
            // Default: prefer CUDA if available, else wgpu
            if BackendSelector::Cuda.is_available() {
                BackendSelector::Cuda
            } else {
                BackendSelector::Wgpu
            }
        });

    println!("Using {} backend", backend.name());
    
    // Verify backend is actually available
    if !backend.is_available() {
        eprintln!("Error: {} backend not compiled in", backend.name());
        std::process::exit(1);
    }

    // Create renderer with selected backend
    // (Actual implementation in webgpui-app)
    // let renderer = create_renderer_for_backend(backend)?;
}
```

### Example: Environment Variable

```rust
use std::env;
use webgpui_render::BackendSelector;

fn select_backend_from_env() -> BackendSelector {
    env::var("WEBGPUI_BACKEND")
        .ok()
        .and_then(|backend_name| {
            match backend_name.to_lowercase().as_str() {
                "cuda" => Some(BackendSelector::Cuda),
                "wgpu" => Some(BackendSelector::Wgpu),
                _ => None,
            }
        })
        .and_then(|backend| {
            if backend.is_available() {
                Some(backend)
            } else {
                eprintln!(
                    "Requested backend {} not available; using default",
                    backend.name()
                );
                None
            }
        })
        .unwrap_or_else(|| {
            // Default fallback
            BackendSelector::Wgpu
        })
}

// Usage:
// WEBGPUI_BACKEND=cuda cargo run
// WEBGPUI_BACKEND=wgpu cargo run
```

### Example: Config File

```rust
use webgpui_render::BackendSelector;

#[derive(Debug, serde::Deserialize)]
struct Config {
    backend: String,
}

fn select_backend_from_config(config_path: &str) -> Result<BackendSelector, Box<dyn std::error::Error>> {
    let config_text = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_text)?;

    match config.backend.to_lowercase().as_str() {
        "cuda" => {
            if BackendSelector::Cuda.is_available() {
                Ok(BackendSelector::Cuda)
            } else {
                Err("CUDA backend requested but not available".into())
            }
        }
        "wgpu" => {
            if BackendSelector::Wgpu.is_available() {
                Ok(BackendSelector::Wgpu)
            } else {
                Err("wgpu backend requested but not available".into())
            }
        }
        other => Err(format!("Unknown backend: {}", other).into()),
    }
}

// config.toml
// [webgpui]
// backend = "cuda"  # or "wgpu"
```

## Fallback Strategy

Implement automatic fallback when preferred backend is unavailable:

```rust
use webgpui_render::BackendSelector;

fn select_backend_with_fallback(prefer_cuda: bool) -> BackendSelector {
    if prefer_cuda && BackendSelector::Cuda.is_available() {
        BackendSelector::Cuda
    } else if BackendSelector::Wgpu.is_available() {
        BackendSelector::Wgpu
    } else if prefer_cuda && BackendSelector::Cuda.is_available() {
        // Already tried CUDA above
        panic!("No GPU backend available!");
    } else {
        panic!("No GPU backend available!");
    }
}

// Simpler version
fn select_backend_smart() -> BackendSelector {
    [BackendSelector::Cuda, BackendSelector::Wgpu]
        .into_iter()
        .find(|b| b.is_available())
        .expect("No GPU backend available!")
}
```

## Feature Matrix

| Scenario | Compile Flags | Backend Count | Runtime Selection |
|----------|---------------|---------------|-------------------|
| Development (wgpu only) | `--features backend-wgpu` | 1 | N/A (forced to wgpu) |
| Development (CUDA only) | `--features backend-cuda` | 1 | N/A (forced to CUDA) |
| Server (both available) | `--features backend-wgpu,backend-cuda` | 2 | Via CLI/env/config |
| Release (optimized) | `--features backend-wgpu` (default) | 1 | N/A (smallest binary) |

## Performance Considerations

### Binary Size
- Single backend: ~X MB
- Both backends: ~X MB (code duplication minimal; both backends use same `Renderer` trait)
- Backend switching has zero runtime overhead (branching is at compile time via feature gates)

### Initialization Time
- wgpu: typically 100-500ms (depends on GPU driver and system)
- CUDA: typically 50-200ms (JIT kernel compilation may add 100-500ms on first run)
- Combined code: negligible overhead from both being linked

### Switching Overhead
- Switching backends requires app restart (not hot-swappable within a running instance)
- If needed for runtime switching, consider using a wrapper process or separate binaries

## Testing Backend Selection

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use webgpui_render::BackendSelector;

    #[test]
    fn backends_are_available() {
        let available = BackendSelector::available();
        assert!(!available.is_empty());
    }

    #[test]
    fn wgpu_available() {
        assert!(BackendSelector::Wgpu.is_available());
    }

    #[test]
    #[cfg(feature = "backend-cuda")]
    fn cuda_available_when_feature_enabled() {
        assert!(BackendSelector::Cuda.is_available());
    }

    #[test]
    #[cfg(not(feature = "backend-cuda"))]
    fn cuda_unavailable_when_feature_disabled() {
        assert!(!BackendSelector::Cuda.is_available());
    }
}
```

### Integration Tests

```bash
# Test wgpu backend
cargo test --features backend-wgpu

# Test CUDA backend
cargo test --features backend-cuda

# Test both backends
cargo test --features backend-wgpu,backend-cuda

# Test default (wgpu)
cargo test
```

### CI Configuration

```yaml
# .github/workflows/test.yml
strategy:
  matrix:
    backend:
      - wgpu
      - cuda
      - both
env:
  WEBGPUI_BACKEND: ${{ matrix.backend }}
```

## Comparison with Other Solutions

### Option A: Compile-Time Only (Original Design)
- ✅ Smallest binary
- ✅ Clearest code paths
- ❌ No runtime flexibility
- ❌ Can't detect unavailable backends until runtime failure

### Option B: Runtime Selection (Current Design)
- ✅ Flexible runtime selection
- ✅ Graceful fallback support
- ✅ Benchmarking capabilities
- ✅ Smart hardware detection
- ✅ Feature-gated code eliminates unused backend
- ❌ Slightly larger binary when both backends enabled
- ❌ Small code overhead for detection API

### Option C: Plugin System
- ✅ Most flexible
- ✅ Can load backends from external libraries
- ❌ Much more complex
- ❌ Larger runtime overhead
- ❌ Dependency management complexity

**Recommendation**: Option B (current design) balances flexibility, code simplicity, and runtime performance.

## Future Enhancements

1. **Hot switching**: Support switching backends without app restart (requires state serialization)
2. **Per-window backends**: Different windows using different backends
3. **Performance telemetry**: Automatic profiling and backend selection recommendation
4. **Cloud deployment**: Select backend based on cloud provider GPU availability
5. **Mobile support**: Add mobile-specific backends (Metal, Vulkan) alongside current backends

## References
- [CUDA Backend Support](cuda-backend.md) - Full CUDA setup and configuration guide
- [Workspace Architecture](../architecture/workspace-architecture.md) - Feature flag policy details
- [Requirements](../architecture/requirements.md) - Architecture decision rationale
