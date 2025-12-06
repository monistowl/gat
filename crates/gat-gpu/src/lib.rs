//! # gat-gpu: GPU Acceleration for Power System Analysis
//!
//! This crate provides GPU-accelerated compute kernels using [wgpu](https://wgpu.rs),
//! enabling hardware-accelerated power flow calculations and Monte Carlo simulations.
//!
//! ## Features
//!
//! - **Cross-platform GPU support**: Vulkan, Metal, DX12, and WebGPU backends
//! - **Automatic fallback**: Falls back to CPU when no GPU is available
//! - **WGSL shaders**: Power mismatch and utility kernels in WebGPU Shading Language
//! - **Zero-copy buffer management**: Efficient data transfer between CPU and GPU
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
pub use dispatch::{Backend, ComputeDispatch, DispatchResult, ExecutionMode};
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
