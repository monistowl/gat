//! # gat-gpu: GPU Acceleration for Power System Analysis
//!
//! This crate provides GPU-accelerated compute kernels using [wgpu](https://wgpu.rs),
//! enabling hardware-accelerated power flow calculations, contingency analysis,
//! and Monte Carlo simulations.
//!
//! ## Features
//!
//! - **Cross-platform GPU support**: Vulkan, Metal, DX12, and WebGPU backends
//! - **Automatic fallback**: Falls back to CPU when no GPU is available
//! - **WGSL shaders**: Power system compute kernels in WebGPU Shading Language
//! - **Zero-copy buffer management**: Efficient data transfer between CPU and GPU
//! - **Precision-aware dispatch**: f32 for safe workloads, configurable for sensitive ones
//!
//! ## Available Shaders
//!
//! | Shader | Precision | Description |
//! |--------|-----------|-------------|
//! | `POWER_MISMATCH_SHADER` | f32 | AC power flow mismatch computation |
//! | `CAPACITY_CHECK_SHADER` | f32 | Monte Carlo capacity adequacy check |
//! | `LODF_SCREENING_SHADER` | f32 | N-1 contingency LODF-based screening |
//! | `PTDF_SHADER` | f32 | Power Transfer Distribution Factors |
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use gat_gpu::{GpuContext, GpuBuffer, KernelRunner, is_gpu_available};
//!
//! // Check if GPU is available
//! if !is_gpu_available() {
//!     println!("No GPU detected, falling back to CPU");
//!     return;
//! }
//!
//! // Create GPU context
//! let ctx = GpuContext::new().expect("Failed to create GPU context");
//! println!("Using GPU: {}", ctx.adapter_name());
//!
//! // Create buffer with data
//! let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
//! let buffer = GpuBuffer::new(&ctx, &data, "my_data");
//!
//! // Run shader and read results
//! let result = buffer.read(&ctx);
//! ```
//!
//! ## Execution Modes
//!
//! The [`ExecutionMode`] enum controls GPU/CPU dispatch:
//!
//! - [`ExecutionMode::Auto`]: Use GPU if available, otherwise CPU (default)
//! - [`ExecutionMode::CpuOnly`]: Always use CPU (for testing/debugging)
//! - [`ExecutionMode::GpuOnly`]: Always use GPU (error if unavailable)
//!
//! ## Precision Control
//!
//! WGSL (WebGPU Shading Language) only supports f32 natively. The [`GpuPrecision`] enum
//! controls how workloads handle this limitation:
//!
//! - [`GpuPrecision::Single`]: Use f32 (fast, sufficient for most workloads)
//! - [`GpuPrecision::SingleWithWarning`]: Use f32 but warn about precision loss
//! - [`GpuPrecision::EmulatedDouble`]: Software f64 emulation on GPU (slower)
//! - [`GpuPrecision::Hybrid`]: GPU for f32-safe parts, CPU for precision-critical parts
//!
//! All current shaders use f32, which is appropriate for:
//! - Power mismatch computation (iterative refinement handles precision)
//! - Monte Carlo capacity checks (statistical noise >> precision error)
//! - LODF/PTDF screening (conservative estimates acceptable)
//!
//! Newton-Raphson Jacobian solves require f64 and are planned for Phase 3.
//!
//! ## CLI Integration
//!
//! Enable GPU acceleration in the CLI with:
//!
//! ```bash
//! # Build with GPU support
//! cargo build -p gat-cli --features gpu
//!
//! # Run with GPU acceleration
//! gat --gpu batch pf ...
//!
//! # Control precision mode
//! gat --gpu --gpu-precision f32 batch pf ...
//! gat --gpu --gpu-precision auto batch pf ...  # default
//!
//! # Check GPU availability
//! gat doctor
//! ```
//!
//! ## Benchmarks
//!
//! Run performance benchmarks with:
//!
//! ```bash
//! cargo bench -p gat-gpu
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐    ┌──────────────┐    ┌─────────────┐
//! │  GpuContext │───▶│  GpuBuffer   │───▶│KernelRunner │
//! │  (device,   │    │  (host↔GPU   │    │ (compute    │
//! │   queue)    │    │   transfer)  │    │  dispatch)  │
//! └─────────────┘    └──────────────┘    └─────────────┘
//!        │                                      │
//!        ▼                                      ▼
//! ┌─────────────┐                        ┌─────────────┐
//! │   wgpu      │                        │  WGSL       │
//! │  Vulkan/    │                        │  Shaders    │
//! │  Metal/DX12 │                        │             │
//! └─────────────┘                        └─────────────┘
//! ```

mod buffers;
mod context;
mod dispatch;
mod kernels;
pub mod shaders;

pub use buffers::GpuBuffer;
pub use context::GpuContext;
pub use dispatch::{Backend, ComputeDispatch, DispatchResult, ExecutionMode, GpuPrecision};
pub use kernels::*;

/// Check if GPU acceleration is available
pub fn is_gpu_available() -> bool {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .is_ok()
    })
}
