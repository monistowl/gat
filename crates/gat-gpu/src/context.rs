//! GPU device and queue management.

use anyhow::{anyhow, Result};
use std::sync::Arc;

/// GPU context holding device and queue handles
pub struct GpuContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    adapter_info: wgpu::AdapterInfo,
}

impl GpuContext {
    /// Create a new GPU context, selecting the best available adapter
    pub fn new() -> Result<Self> {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| anyhow!("No GPU adapter found: {}", e))?;

        let adapter_info = adapter.get_info();

        let (device, queue) = adapter
            .request_device(&Default::default())
            .await
            .map_err(|e| anyhow!("Failed to create device: {}", e))?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
        })
    }

    /// Get adapter name for diagnostics
    pub fn adapter_name(&self) -> &str {
        &self.adapter_info.name
    }

    /// Get backend type (Vulkan, Metal, DX12, etc.)
    pub fn backend(&self) -> wgpu::Backend {
        self.adapter_info.backend
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_context_creation() {
        // This test will skip if no GPU is available
        if !crate::is_gpu_available() {
            eprintln!("Skipping GPU test: no GPU available");
            return;
        }

        let ctx = GpuContext::new().expect("Failed to create GPU context");
        println!("GPU: {} ({:?})", ctx.adapter_name(), ctx.backend());
    }
}
