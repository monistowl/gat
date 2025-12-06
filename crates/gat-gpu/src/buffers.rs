//! GPU buffer management for data transfer.

use crate::GpuContext;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// A GPU buffer that can be read/written from CPU
pub struct GpuBuffer<T: Pod + Zeroable> {
    pub(crate) buffer: wgpu::Buffer,
    pub(crate) staging: wgpu::Buffer,
    pub(crate) size: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Pod + Zeroable> GpuBuffer<T> {
    /// Create a new GPU buffer with initial data
    pub fn new(ctx: &GpuContext, data: &[T], label: &str) -> Self {
        let size = data.len();
        let byte_size = std::mem::size_of_val(data) as u64;

        // Storage buffer for compute shader access
        let buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            });

        // Staging buffer for CPU readback
        let staging = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{}_staging", label)),
            size: byte_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            staging,
            size,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Read buffer contents back to CPU
    pub fn read(&self, ctx: &GpuContext) -> Vec<T> {
        // Copy from storage to staging
        let mut encoder = ctx.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
            &self.buffer,
            0,
            &self.staging,
            0,
            (self.size * std::mem::size_of::<T>()) as u64,
        );
        ctx.queue.submit(Some(encoder.finish()));

        // Map staging buffer and read
        let slice = self.staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = ctx.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        let data = slice.get_mapped_range();
        let result: Vec<T> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        self.staging.unmap();

        result
    }

    /// Number of elements
    pub fn len(&self) -> usize {
        self.size
    }

    /// Whether the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_roundtrip() {
        if !crate::is_gpu_available() {
            eprintln!("Skipping: no GPU");
            return;
        }

        let ctx = GpuContext::new().unwrap();
        let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
        let buffer = GpuBuffer::new(&ctx, &data, "test");
        let result = buffer.read(&ctx);
        assert_eq!(data, result);
    }
}
