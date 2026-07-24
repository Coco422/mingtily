# Native acceleration build guide

Mingtily exposes Whisper acceleration through Cargo features. Build flags affect compilation only; actual runtime performance depends on the user's hardware, drivers, model, and audio workload. Do not publish unmeasured speed multipliers from CI runners.

## Supported feature flags

| Feature | Intended platform | Requirement |
|---|---|---|
| `metal` | macOS | Apple Metal; enabled by the macOS `whisper-rs` dependency. |
| `coreml` | macOS | Apple CoreML; enabled by the macOS dependency and requires compatible model/runtime support. |
| `openblas` | Linux/Windows | OpenBLAS development libraries available at build time. |
| `vulkan` | Linux/Windows | Vulkan SDK and compatible runtime/driver. |
| `cuda` | Linux/Windows | NVIDIA CUDA toolkit and supported GPU. |
| `hipblas` | Linux | ROCm/HIP toolchain and supported AMD GPU. |

## CI choices

- macOS workflows use the platform dependency configuration with Metal/CoreML support.
- Linux bundle workflows use `--features openblas` because GitHub-hosted runners do not provide a useful GPU target.
- GPU-specific Windows/Linux builds should be treated as development experiments until they are tested on matching physical hardware.
- The automatic Rust validation job uses default features so it checks portable compilation rather than assuming an accelerator.

## Local commands

Run from `frontend/`:

```bash
pnpm tauri:build:openblas
pnpm tauri:build:vulkan
pnpm tauri:build:cuda
pnpm tauri:build:hipblas
pnpm tauri:build:metal
pnpm tauri:build:coreml
```

Install the native SDK or library before enabling a feature. A feature compiling successfully does not prove that inference uses the accelerator; verify runtime logs and measure a representative audio file on the target machine.
