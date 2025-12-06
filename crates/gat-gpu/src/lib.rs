//! GPU acceleration for GAT power system analysis.
//!
//! This crate provides GPU-accelerated compute kernels using wgpu.
//! Falls back to CPU implementation when no GPU is available.

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
