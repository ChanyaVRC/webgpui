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

To enable dynamic switching, compile multiple backends into your binary:

```bash
# Enable both GPU backends for runtime selection
cargo build --features backend-wgpu,backend-cuda

# Enable GPU and CPU backends
cargo build --features backend-wgpu,backend-cpu

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
cargo build --features backend-cpu   # CPU only (headless, no GPU required)
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

// Output (with wgpu + CUDA features):
// Available: wgpu (wgpu)
// Available: CUDA (CUDA)
//
// Output (with wgpu + CPU features):
// Available: wgpu (wgpu)
// Available: CPU (cpu)
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

if BackendSelector::Cpu.is_available() {
    println!("CPU backend is available (no GPU required)");
}
```

### Detect Hardware and Select Backend

```rust
use webgpui_render::BackendSelector;

// Example: detect NVIDIA GPU and prefer CUDA, fall back to wgpu, then CPU
fn select_backend() -> BackendSelector {
    // Check for NVIDIA GPU (pseudocode; actual implementation depends on system)
    if has_nvidia_gpu() && BackendSelector::Cuda.is_available() {
        BackendSelector::Cuda
    } else if BackendSelector::Wgpu.is_available() {
        BackendSelector::Wgpu
    } else if BackendSelector::Cpu.is_available() {
        BackendSelector::Cpu
    } else {
        panic!("No backend available!");
    }
}

fn has_nvidia_gpu() -> bool {
    // Implementation: check nvidia-smi or CUDA runtime
    // For MVP, assume true if user requests CUDA
    true
}
```

## CPU Backend (`BackendSelector::Cpu`)

### What It Is

`BackendSelector::Cpu` selects the headless CPU software renderer. It requires no GPU
and produces correct output entirely on the CPU. The implementation lives in the
`webgpui-render-cpu` crate.

### When to Use It

- **CI environments**: run rendering tests on machines without a GPU (e.g., standard GitHub
  Actions runners).
- **Automated testing**: deterministic pixel output without driver variation.
- **Headless servers**: generate UI screenshots or thumbnails server-side with no display.
- **Fallback of last resort**: ensure the application can start even when all GPU backends
  fail to initialize.

The CPU backend trades throughput for portability. It is not intended for interactive,
frame-rate-sensitive use.

### How to Enable

Add the `backend-cpu` feature flag in `Cargo.toml`:

```toml
[dependencies]
webgpui = { version = "0.1", features = ["backend-cpu"] }
```

Or pass it on the command line:

```bash
cargo build --features backend-cpu
cargo test --features backend-cpu
```

Combining it with a GPU backend enables a CPU fallback at runtime:

```bash
cargo build --features backend-wgpu,backend-cpu
```

### Example: Fallback to CPU in Headless / CI Contexts

```rust
use webgpui_render::BackendSelector;

fn select_backend() -> BackendSelector {
    if BackendSelector::Wgpu.is_available() {
        BackendSelector::Wgpu
    } else if BackendSelector::Cpu.is_available() {
        // No GPU present — fall back to the headless CPU renderer.
        // Suitable for CI and server-side rendering.
        BackendSelector::Cpu
    } else {
        panic!("No backend available!");
    }
}
```

### Testing with the CPU Backend

```rust
#[cfg(test)]
mod tests {
    use webgpui_render::BackendSelector;

    #[test]
    #[cfg(feature = "backend-cpu")]
    fn cpu_available_when_feature_enabled() {
        assert!(BackendSelector::Cpu.is_available());
    }

    #[test]
    #[cfg(not(feature = "backend-cpu"))]
    fn cpu_unavailable_when_feature_disabled() {
        assert!(!BackendSelector::Cpu.is_available());
    }
}
```

```bash
# Run tests with the CPU backend (no GPU required)
cargo test --features backend-cpu
```

### CI Configuration with CPU Backend

```yaml
# .github/workflows/test.yml
strategy:
  matrix:
    backend:
      - wgpu
      - cuda
      - cpu    # headless; runs on standard runners without GPU
env:
  WEBGPUI_BACKEND: ${{ matrix.backend }}
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
                "cpu"  => Some(BackendSelector::Cpu),
                _ => None,
            }
        })
        .unwrap_or_else(|| {
            // Default: prefer CUDA if available, else wgpu, else CPU
            if BackendSelector::Cuda.is_available() {
                BackendSelector::Cuda
            } else if BackendSelector::Wgpu.is_available() {
                BackendSelector::Wgpu
            } else {
                BackendSelector::Cpu
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
                "cpu"  => Some(BackendSelector::Cpu),
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
        "cpu" => {
            if BackendSelector::Cpu.is_available() {
                Ok(BackendSelector::Cpu)
            } else {
                Err("cpu backend requested but not available".into())
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
    } else if BackendSelector::Cpu.is_available() {
        // No GPU available — fall back to headless CPU renderer.
        BackendSelector::Cpu
    } else {
        panic!("No backend available!");
    }
}

// Simpler version (CUDA > wgpu > CPU)
fn select_backend_smart() -> BackendSelector {
    [BackendSelector::Cuda, BackendSelector::Wgpu, BackendSelector::Cpu]
        .into_iter()
        .find(|b| b.is_available())
        .expect("No backend available!")
}
```

## Feature Matrix

| Scenario | Compile Flags | Backend Count | Runtime Selection |
|----------|---------------|---------------|-------------------|
| Development (wgpu only) | `--features backend-wgpu` | 1 | N/A (forced to wgpu) |
| Development (CUDA only) | `--features backend-cuda` | 1 | N/A (forced to CUDA) |
| CI / headless testing | `--features backend-cpu` | 1 | N/A (forced to CPU) |
| Server (GPU + fallback) | `--features backend-wgpu,backend-cpu` | 2 | Via CLI/env/config |
| Server (both GPU backends) | `--features backend-wgpu,backend-cuda` | 2 | Via CLI/env/config |
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

# Test CPU backend (headless; no GPU required)
cargo test --features backend-cpu

# Test wgpu + CPU (GPU with headless fallback)
cargo test --features backend-wgpu,backend-cpu

# Test all GPU backends
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
      - cpu    # headless; runs on standard runners without GPU
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
