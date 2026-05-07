# CUDA Backend Support

## Overview

WebGPUI supports two GPU rendering backends:

1. **wgpu** (default): Cross-platform, works on any hardware with modern GPU
2. **CUDA** (optional): NVIDIA-specific, offers fine-grained GPU control for potential performance gains

This guide covers setup, configuration, and usage of the CUDA backend.

## Why CUDA?

### Advantages
- **Fine-grained GPU control**: Access to NVIDIA CUDA APIs enables custom kernel compilation
- **Performance potential**: Can optimize for specific compute workloads beyond standard graphics pipelines
- **Hardware-specific optimization**: Tailor rendering to NVIDIA GPU architecture strengths
- **Compute capability**: CUDA enables future features like physics simulation, post-processing compute shaders

### Limitations
- **Hardware lock-in**: Requires NVIDIA GPU (Maxwell generation or newer)
- **Platform restrictions**: Supported on Linux (x86_64) and Windows (x86_64) only
- **Toolkit dependency**: CUDA Toolkit 11.8+ must be installed on build machine
- **Slower iteration**: Requires CUDA Toolkit installation; wgpu is simpler for rapid development

### When to Use CUDA
- You have NVIDIA hardware and want maximum performance on specific hardware
- You need advanced compute capabilities (physics, post-effects)
- You're targeting production deployment on servers with NVIDIA GPUs
- You want to benchmark CUDA vs wgpu on the same hardware

### When to Use wgpu (Default)
- Development on macOS (CUDA not available)
- Cross-platform deployment (AMD, Intel, Apple hardware)
- Rapid prototyping (no toolkit installation needed)
- Portability is more important than hardware-specific optimization

## Hardware Requirements

### GPU Support
- **NVIDIA GPU with CUDA Compute Capability 3.5 or higher**
  - Maxwell generation (GTX 750 Ti, GTX 960, Quadro M) or newer
  - All modern NVIDIA datacenter GPUs (V100, A100, H100, etc.)

### CUDA Toolkit Version
- **Minimum: CUDA Toolkit 11.8**
- **Recommended: CUDA Toolkit 12.0 or newer**

### Supported Platforms
- **Linux (x86_64)**: Primary supported platform
  - NVIDIA CUDA Toolkit for Linux
  - NVIDIA GPU Driver compatible with CUDA version
- **Windows (x86_64)**: Via CUDA for Windows
  - NVIDIA CUDA Toolkit for Windows
  - Visual Studio 2019/2022 with CUDA support
- **macOS**: Not supported (NVIDIA CUDA not available for Apple Silicon)

### Driver Compatibility
- GPU driver must be compatible with installed CUDA Toolkit version
- Check [NVIDIA CUDA Compatibility](https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html) for driver version mapping

## Installation & Setup

### 1. Install CUDA Toolkit

#### Linux (Ubuntu/Debian)
```bash
# Download CUDA 12.0 from https://developer.nvidia.com/cuda-downloads
# Or use apt (Ubuntu 22.04 example):
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_*.deb
sudo apt-get update
sudo apt-get install cuda-toolkit-12-0
```

#### Windows
1. Download CUDA Toolkit from [NVIDIA Developer Site](https://developer.nvidia.com/cuda-downloads)
2. Run installer, select custom installation
3. Ensure Visual Studio integration is selected
4. Add CUDA paths to environment variables:
   ```
   CUDA_PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.0
   PATH=%CUDA_PATH%\bin;%PATH%
   ```

### 2. Verify CUDA Installation
```bash
# Check NVIDIA GPU driver
nvidia-smi

# Check CUDA Toolkit
nvcc --version
```

Example successful output:
```
$ nvidia-smi
+-----------------------------------------------------------------------------+
| NVIDIA-SMI 545.29.06    Driver Version: 545.29.06    CUDA Version: 12.0     |
+-----------------------------------------------------------------------------+
```

### 3. Enable CUDA Feature in Your Project

Update `Cargo.toml`:
```toml
[dependencies]
webgpui = { path = ".", features = ["backend-cuda"] }
```

Or from command line:
```bash
cargo build --features backend-cuda
cargo run --features backend-cuda
```

## Configuration

### Backend Availability Detection

The `BackendSelector` enum (in `webgpui-render`) provides runtime detection of available backends:

```rust
use webgpui_render::BackendSelector;

// Query which backends are compiled in
let available_backends = BackendSelector::available();
for backend in &available_backends {
    println!("Available: {}", backend.name());
}

// Check if specific backend is available
if BackendSelector::Cuda.is_available() {
    println!("CUDA is available!");
} else {
    println!("CUDA not compiled in or unavailable");
}
```

### Feature Flags

#### Compile-Time Selection
The WebGPUI build system uses feature flags to select the GPU backend at compile time:

```toml
# Option 1: Use wgpu (default)
[dependencies]
webgpui = { version = "0.1", features = ["backend-wgpu"] }

# Option 2: Use CUDA
[dependencies]
webgpui = { version = "0.1", features = ["backend-cuda"] }

# Option 3: Include both (only one active per binary)
[dependencies]
webgpui = { version = "0.1", features = ["backend-wgpu", "backend-cuda"] }
```

#### Feature Requirements
- Exactly one backend must be enabled at a time in a binary crate
- Libraries can include both backends; the binary crate selects which is used
- No automatic fallback between backends (must be explicit in build configuration)

### Runtime Configuration

After selecting the backend at compile time via features, the app can expose runtime backend selection (if both are linked):

```rust
// In webgpui-render
use webgpui_render::BackendSelector;

// Check which backends are available
let available = BackendSelector::available();
println!("Available backends: {:?}", available);

// Create a backend selector
let backend = if has_nvidia_gpu { 
    BackendSelector::Cuda 
} else { 
    BackendSelector::Wgpu 
};

// Verify backend is available before using
if !backend.is_available() {
    eprintln!("Selected backend {} not compiled in", backend.name());
}

// In webgpui-app (when both backends are linked)
pub enum BackendMode {
    Wgpu,
    Cuda,
}

pub fn init_renderer(mode: BackendMode) -> Result<Box<dyn Renderer>> {
    match mode {
        #[cfg(feature = "backend-wgpu")]
        BackendMode::Wgpu => Ok(Box::new(WgpuRenderer::new()?)),
        
        #[cfg(feature = "backend-cuda")]
        BackendMode::Cuda => Ok(Box::new(CudaRenderer::new()?)),
        
        _ => Err("Backend not compiled in"),
    }
}
```

## Performance Expectations

### Baseline Measurements

When properly configured, CUDA and wgpu should meet the same performance targets:

- **Frame time**: Average ≤ 16.6ms (60 FPS), P95 ≤ 20ms
- **Draw calls**: ≤ 200 on representative screens (after batching P1)
- **Memory**: Minimal per-frame heap allocation

### CUDA-Specific Considerations

1. **First-frame latency**: CUDA kernel compilation on first run may add startup overhead
   - Mitigation: Use `cudarc` kernel caching or JIT compilation strategies
   - Recommend prewarm on app startup

2. **GPU synchronization**: CUDA requires explicit synchronization; ensure proper use of `cuStreamSynchronize()` to avoid blocking CPU

3. **Memory management**: CUDA memory is separate from system memory; budget GPU memory for textures and buffers

### Equivalence Testing

CUDA and wgpu output **must be pixel-identical** for the same input scene:
- Visual snapshots compared via byte-exact matching
- Event sequences must be identical
- Performance measured separately (CUDA may be faster or slower depending on workload)

See [API Swapping Quality Plan](../rendering/api-swapping-quality-plan.md) for equivalence test details.

## Troubleshooting

### Issue: "CUDA support disabled; enable 'cuda' feature"
**Solution**: Add `features = ["backend-cuda"]` to your `Cargo.toml`

### Issue: CUDA Toolkit not found during build
**Solution**:
- Linux: Ensure `nvcc` is in `PATH`:
  ```bash
  export PATH=/usr/local/cuda/bin:$PATH
  ```
- Windows: Verify `CUDA_PATH` environment variable is set and correct

### Issue: NVIDIA GPU not detected
**Solution**:
- Verify NVIDIA driver: `nvidia-smi`
- Ensure GPU meets minimum Compute Capability 3.5
- Update GPU driver: `nvidia-driver-update` (Linux) or Windows Device Manager

### Issue: "cudarc" crate build fails
**Solution**:
- Upgrade CUDA Toolkit to 11.8+
- Verify `libcuda.so` (Linux) or `nvcuda.dll` (Windows) is accessible
- Check cudarc version matches your CUDA Toolkit (cudarc 0.12 → CUDA 11.8+)

### Issue: Performance is worse on CUDA than wgpu
**Possible causes**:
- Different GPU utilization patterns for compute-light GUI workloads
- Synchronization overhead between CPU and GPU
- Suboptimal kernel compilation parameters
- **Recommendation**: Profile with `CUDA_PROFILE=1 ./app` and compare with wgpu `wgpu_core::Trace` logs

## Benchmarking CUDA vs wgpu

### Environment Setup
Prepare identical hardware and driver versions:

```bash
# Record baseline metrics
cargo run --features backend-wgpu --release -- --bench-frames 1000 > wgpu-metrics.txt
cargo run --features backend-cuda --release -- --bench-frames 1000 > cuda-metrics.txt

# Compare output
diff wgpu-metrics.txt cuda-metrics.txt
```

### Metrics Comparison
Expected output format (defined in [CI Metrics Format](../quality/metrics-format.md)):
```
p0-metrics:
  avg_ms: 14.2
  p95_ms: 18.5
  draw_calls: 145
```

### Acceptable Variance
- Frame time variance: ±20% acceptable between backends
- Draw calls must be identical (batching is backend-agnostic)
- Visual output must be pixel-identical (within rounding error for float math)

## Documentation References
- [Requirements](../architecture/requirements.md) - CUDA listed as baseline technology
- [Workspace Architecture](../architecture/workspace-architecture.md) - `webgpui-render-cuda` crate design
- [API Swapping Quality Plan](../rendering/api-swapping-quality-plan.md) - Equivalence testing strategy
- [CUDA Toolkit Docs](https://docs.nvidia.com/cuda/) - Official NVIDIA documentation
- [cudarc Crate Docs](https://docs.rs/cudarc/) - Safe CUDA Rust bindings

## FAQ

**Q: Can I use both CUDA and wgpu in the same application?**  
A: Yes! With both features compiled in (`--features backend-wgpu,backend-cuda`), the app can select which backend to use at runtime via `BackendSelector::available()` and `BackendMode` enum. This enables:
  - Fallback strategy (prefer CUDA, fall back to wgpu if unavailable)
  - User selection (CLI flag or config file to choose backend)
  - Benchmarking (run same workload on both backends)
  - Graceful degradation (detect GPU unavailability and switch automatically)

**Q: Will my code run on CUDA and wgpu without changes?**  
A: Yes, the `Renderer` trait is implemented by both backends. App code only calls the trait methods, not backend-specific APIs.

**Q: What about AMD GPUs?**  
A: AMD GPUs are supported via wgpu. For AMD-specific optimization, consider HIP (AMD's CUDA equivalent) in future backend.

**Q: Can I contribute CUDA kernel optimizations?**  
A: Yes! CUDA kernel code lives in `crates/webgpui-render-cuda/kernels/` (future). Submit PRs with performance improvements and equivalence test validation.
