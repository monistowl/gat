//! GPU performance benchmarks for gat-gpu crate.
//!
//! This benchmark suite measures:
//! - GPU context initialization time
//! - Buffer upload/download throughput
//! - Compute shader dispatch latency
//! - Power mismatch kernel performance at various scales
//! - CPU vs GPU comparison for parallel workloads
//!
//! ## Running Benchmarks
//!
//! ```bash
//! # Run all GPU benchmarks
//! cargo bench -p gat-gpu
//!
//! # Run specific benchmark group
//! cargo bench -p gat-gpu -- buffer_transfer
//!
//! # Generate HTML reports
//! cargo bench -p gat-gpu -- --save-baseline gpu-baseline
//! ```
//!
//! ## Performance Targets
//!
//! | Operation | Size | Target | Notes |
//! |-----------|------|--------|-------|
//! | Context init | - | <100ms | One-time startup cost |
//! | Buffer upload | 1MB | <5ms | Host->Device |
//! | Buffer download | 1MB | <10ms | Device->Host |
//! | Simple kernel | 1M elements | <1ms | Element-wise operation |
//! | Power mismatch | 10K buses | <10ms | Full Y-bus sparse mult |
//!
//! Note: First GPU operation may be slower due to shader compilation.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use gat_gpu::shaders::{CAPACITY_CHECK_SHADER, LODF_SCREENING_SHADER};
use gat_gpu::{BufferBinding, GpuBuffer, GpuContext, KernelRunner, MultiBufferKernel};

/// Buffer sizes to benchmark (in number of f32 elements)
const BUFFER_SIZES: &[usize] = &[
    1_024,     // 4 KB
    16_384,    // 64 KB
    262_144,   // 1 MB
    1_048_576, // 4 MB
    4_194_304, // 16 MB
];

/// Bus counts for power flow benchmarks
const BUS_COUNTS: &[usize] = &[
    100,    // Small distribution network
    1_000,  // Medium transmission network
    10_000, // Large transmission network
];

/// Scenario counts for Monte Carlo benchmarks
const SCENARIO_COUNTS: &[usize] = &[
    1_000,   // Small test
    10_000,  // Medium scale
    100_000, // Large scale
];

/// Branch/contingency counts for LODF benchmarks
const CONTINGENCY_COUNTS: &[(usize, usize)] = &[
    (100, 50),   // 100 branches, 50 contingencies
    (500, 100),  // 500 branches, 100 contingencies
    (1000, 200), // 1000 branches, 200 contingencies
];

/// Simple WGSL shader for benchmarking element-wise operations
const SCALE_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx < arrayLength(&data)) {
        data[idx] = data[idx] * 2.0;
    }
}
"#;

/// Benchmark: GPU context initialization
fn bench_context_init(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_init");

    // Skip if no GPU available
    if !gat_gpu::is_gpu_available() {
        eprintln!("Skipping GPU benchmarks: no GPU available");
        return;
    }

    group.bench_function("gpu_context_new", |b| {
        b.iter(|| {
            let ctx = GpuContext::new().expect("Failed to create GPU context");
            black_box(ctx)
        })
    });

    group.finish();
}

/// Benchmark: Buffer upload (host -> device)
fn bench_buffer_upload(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_upload");

    if !gat_gpu::is_gpu_available() {
        return;
    }

    let ctx = GpuContext::new().expect("GPU context");

    for &size in BUFFER_SIZES {
        let data: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let bytes = size * std::mem::size_of::<f32>();

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(
            BenchmarkId::new("upload", format!("{}KB", bytes / 1024)),
            &data,
            |b, data| {
                b.iter(|| {
                    let buffer = GpuBuffer::new(&ctx, data, "bench_buffer");
                    black_box(buffer)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Buffer download (device -> host)
fn bench_buffer_download(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_download");

    if !gat_gpu::is_gpu_available() {
        return;
    }

    let ctx = GpuContext::new().expect("GPU context");

    for &size in BUFFER_SIZES {
        let data: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let bytes = size * std::mem::size_of::<f32>();

        // Pre-create buffer
        let buffer = GpuBuffer::new(&ctx, &data, "bench_buffer");

        group.throughput(Throughput::Bytes(bytes as u64));
        group.bench_with_input(
            BenchmarkId::new("download", format!("{}KB", bytes / 1024)),
            &buffer,
            |b, buffer| {
                b.iter(|| {
                    let result = buffer.read(&ctx);
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Simple compute shader dispatch
fn bench_compute_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("compute_dispatch");

    if !gat_gpu::is_gpu_available() {
        return;
    }

    let ctx = GpuContext::new().expect("GPU context");
    let runner = KernelRunner::from_wgsl(&ctx, SCALE_SHADER, "main").expect("kernel");

    for &size in BUFFER_SIZES {
        let data: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let buffer = GpuBuffer::new(&ctx, &data, "bench_buffer");

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("scale_kernel", format!("{}K", size / 1024)),
            &buffer,
            |b, buffer| {
                b.iter(|| {
                    runner.dispatch(buffer, 64).expect("dispatch failed");
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Full roundtrip (upload -> compute -> download)
fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    if !gat_gpu::is_gpu_available() {
        return;
    }

    let ctx = GpuContext::new().expect("GPU context");
    let runner = KernelRunner::from_wgsl(&ctx, SCALE_SHADER, "main").expect("kernel");

    for &size in BUFFER_SIZES {
        let data: Vec<f32> = (0..size).map(|i| i as f32).collect();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("upload_compute_download", format!("{}K", size / 1024)),
            &data,
            |b, data| {
                b.iter(|| {
                    // Upload
                    let buffer = GpuBuffer::new(&ctx, data, "bench_buffer");
                    // Compute
                    runner.dispatch(&buffer, 64).expect("dispatch failed");
                    // Download
                    let result = buffer.read(&ctx);
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: CPU vs GPU comparison for element-wise operations
fn bench_cpu_vs_gpu(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_vs_gpu");

    // CPU baseline: scale all elements by 2
    fn cpu_scale(data: &mut [f32]) {
        for x in data.iter_mut() {
            *x *= 2.0;
        }
    }

    // CPU parallel baseline using rayon
    fn cpu_scale_parallel(data: &mut [f32]) {
        use std::thread;
        let threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let chunk_size = (data.len() + threads - 1) / threads;

        // Simple parallel using scoped threads
        std::thread::scope(|s| {
            for chunk in data.chunks_mut(chunk_size) {
                s.spawn(|| {
                    for x in chunk.iter_mut() {
                        *x *= 2.0;
                    }
                });
            }
        });
    }

    let gpu_available = gat_gpu::is_gpu_available();
    let ctx = if gpu_available {
        Some(GpuContext::new().expect("GPU context"))
    } else {
        None
    };
    let runner = ctx
        .as_ref()
        .map(|c| KernelRunner::from_wgsl(c, SCALE_SHADER, "main").expect("kernel"));

    // Use a representative size
    let size = 1_048_576; // 1M elements = 4MB
    let data: Vec<f32> = (0..size).map(|i| i as f32).collect();

    group.throughput(Throughput::Elements(size as u64));

    // CPU single-threaded
    group.bench_function("cpu_single", |b| {
        b.iter(|| {
            let mut data = data.clone();
            cpu_scale(&mut data);
            black_box(data)
        })
    });

    // CPU parallel
    group.bench_function("cpu_parallel", |b| {
        b.iter(|| {
            let mut data = data.clone();
            cpu_scale_parallel(&mut data);
            black_box(data)
        })
    });

    // GPU (if available)
    if let (Some(ctx), Some(runner)) = (&ctx, &runner) {
        group.bench_function("gpu", |b| {
            b.iter(|| {
                let buffer = GpuBuffer::new(ctx, &data, "bench_buffer");
                runner.dispatch(&buffer, 64).expect("dispatch failed");
                let result = buffer.read(ctx);
                black_box(result)
            })
        });
    }

    group.finish();
}

/// Benchmark: Simulated power mismatch computation at various scales
///
/// This simulates the power mismatch computation workload without
/// actual Y-bus data, measuring raw compute throughput.
fn bench_power_mismatch_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("power_mismatch_scale");

    if !gat_gpu::is_gpu_available() {
        return;
    }

    let ctx = GpuContext::new().expect("GPU context");

    // Shader that simulates power mismatch computation load
    // (sin/cos operations per element like real power flow)
    const POWER_MISMATCH_SIMULATED: &str = r#"
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx < arrayLength(&data)) {
        let v = data[idx];
        // Simulate power mismatch: trig operations like real computation
        let angle = v * 0.01;
        let cos_val = cos(angle);
        let sin_val = sin(angle);
        data[idx] = v * cos_val + sin_val;
    }
}
"#;

    let runner = KernelRunner::from_wgsl(&ctx, POWER_MISMATCH_SIMULATED, "main").expect("kernel");

    for &bus_count in BUS_COUNTS {
        // Each bus has ~4 values (V_mag, V_ang, P, Q)
        let element_count = bus_count * 4;
        let data: Vec<f32> = (0..element_count).map(|i| 1.0 + 0.001 * i as f32).collect();
        let buffer = GpuBuffer::new(&ctx, &data, "power_flow");

        group.throughput(Throughput::Elements(bus_count as u64));
        group.bench_with_input(
            BenchmarkId::new("buses", bus_count),
            &buffer,
            |b, buffer| {
                b.iter(|| {
                    runner.dispatch(buffer, 64).expect("dispatch failed");
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Memory bandwidth (large sequential transfers)
fn bench_memory_bandwidth(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_bandwidth");
    group.sample_size(20); // Fewer samples for large transfers

    if !gat_gpu::is_gpu_available() {
        return;
    }

    let ctx = GpuContext::new().expect("GPU context");

    // Large buffer for bandwidth testing: 64MB
    let size = 16_777_216; // 16M elements = 64MB
    let data: Vec<f32> = (0..size).map(|i| i as f32).collect();
    let bytes = size * std::mem::size_of::<f32>();

    group.throughput(Throughput::Bytes(bytes as u64));

    group.bench_function("upload_64MB", |b| {
        b.iter(|| {
            let buffer = GpuBuffer::new(&ctx, &data, "large_buffer");
            black_box(buffer)
        })
    });

    let buffer = GpuBuffer::new(&ctx, &data, "large_buffer");
    group.bench_function("download_64MB", |b| {
        b.iter(|| {
            let result = buffer.read(&ctx);
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark: Monte Carlo capacity check shader
///
/// Measures throughput of the capacity adequacy check kernel
/// across different scenario counts with fixed generator count.
fn bench_monte_carlo_capacity(c: &mut Criterion) {
    use bytemuck::{Pod, Zeroable};

    let mut group = c.benchmark_group("monte_carlo_capacity");

    if !gat_gpu::is_gpu_available() {
        return;
    }

    let ctx = GpuContext::new().expect("GPU context");

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct Uniforms {
        n_scenarios: u32,
        n_generators: u32,
        demand: f32,
        _padding: u32,
    }

    let n_generators = 100usize; // Fixed at 100 generators
    let gen_capacity: Vec<f32> = (0..n_generators).map(|_| 100.0).collect();

    for &n_scenarios in SCENARIO_COUNTS {
        // Generate random outage states (all online for benchmark simplicity)
        let outage_state: Vec<f32> = vec![1.0; n_scenarios * n_generators];
        let adequate: Vec<f32> = vec![0.0; n_scenarios];

        let uniforms = Uniforms {
            n_scenarios: n_scenarios as u32,
            n_generators: n_generators as u32,
            demand: 5000.0, // 50% of total capacity
            _padding: 0,
        };

        let buf_uniforms = GpuBuffer::new_uniform(&ctx, &[uniforms], "uniforms");
        let buf_capacity = GpuBuffer::new(&ctx, &gen_capacity, "gen_capacity");
        let buf_outage = GpuBuffer::new(&ctx, &outage_state, "outage_state");
        let buf_adequate = GpuBuffer::new(&ctx, &adequate, "adequate");

        let kernel = MultiBufferKernel::new(
            &ctx,
            CAPACITY_CHECK_SHADER,
            "main",
            &[
                BufferBinding::Uniform,
                BufferBinding::ReadOnly,
                BufferBinding::ReadOnly,
                BufferBinding::ReadWrite,
            ],
        )
        .expect("kernel");

        group.throughput(Throughput::Elements(n_scenarios as u64));
        group.bench_with_input(
            BenchmarkId::new("scenarios", n_scenarios),
            &n_scenarios,
            |b, &n_scenarios| {
                b.iter(|| {
                    kernel
                        .dispatch(
                            &ctx,
                            &[
                                &buf_uniforms.buffer,
                                &buf_capacity.buffer,
                                &buf_outage.buffer,
                                &buf_adequate.buffer,
                            ],
                            n_scenarios as u32,
                            64,
                        )
                        .expect("dispatch failed");
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: LODF N-1 contingency screening shader
///
/// Measures throughput of the LODF-based contingency screening kernel
/// at various branch/contingency matrix sizes.
fn bench_lodf_screening(c: &mut Criterion) {
    use bytemuck::{Pod, Zeroable};

    let mut group = c.benchmark_group("lodf_screening");

    if !gat_gpu::is_gpu_available() {
        return;
    }

    let ctx = GpuContext::new().expect("GPU context");

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable)]
    struct Uniforms {
        n_branches: u32,
        n_contingencies: u32,
        _padding1: u32,
        _padding2: u32,
    }

    for &(n_branches, n_contingencies) in CONTINGENCY_COUNTS {
        // Pre-contingency flows
        let flow_pre: Vec<f32> = (0..n_branches).map(|i| 50.0 + i as f32 * 0.5).collect();

        // LODF matrix (simplified diagonal-dominant)
        let mut lodf: Vec<f32> = vec![0.1; n_branches * n_branches];
        for i in 0..n_branches {
            lodf[i * n_branches + i] = -1.0; // Diagonal
        }

        // Contingency branch indices
        let contingency_branches: Vec<u32> = (0..n_contingencies as u32).collect();

        // Output
        let flow_post: Vec<f32> = vec![0.0; n_contingencies * n_branches];

        let uniforms = Uniforms {
            n_branches: n_branches as u32,
            n_contingencies: n_contingencies as u32,
            _padding1: 0,
            _padding2: 0,
        };

        let buf_uniforms = GpuBuffer::new_uniform(&ctx, &[uniforms], "uniforms");
        let buf_flow_pre = GpuBuffer::new(&ctx, &flow_pre, "flow_pre");
        let buf_lodf = GpuBuffer::new(&ctx, &lodf, "lodf");
        let buf_contingencies = GpuBuffer::new(&ctx, &contingency_branches, "contingencies");
        let buf_flow_post = GpuBuffer::new(&ctx, &flow_post, "flow_post");

        let kernel = MultiBufferKernel::new(
            &ctx,
            LODF_SCREENING_SHADER,
            "main",
            &[
                BufferBinding::Uniform,
                BufferBinding::ReadOnly,
                BufferBinding::ReadOnly,
                BufferBinding::ReadOnly,
                BufferBinding::ReadWrite,
            ],
        )
        .expect("kernel");

        // Total computations = contingencies × branches
        let total_elements = n_contingencies * n_branches;
        group.throughput(Throughput::Elements(total_elements as u64));

        group.bench_with_input(
            BenchmarkId::new(
                "contingencies_x_branches",
                format!("{}x{}", n_contingencies, n_branches),
            ),
            &(n_contingencies, n_branches),
            |b, _| {
                b.iter(|| {
                    // Manual dispatch for 2D workgroups
                    let mut encoder = ctx.device.create_command_encoder(&Default::default());
                    {
                        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("lodf_bench_bind_group"),
                            layout: &kernel.bind_group_layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: buf_uniforms.buffer.as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: buf_flow_pre.buffer.as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 2,
                                    resource: buf_lodf.buffer.as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 3,
                                    resource: buf_contingencies.buffer.as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 4,
                                    resource: buf_flow_post.buffer.as_entire_binding(),
                                },
                            ],
                        });

                        let mut pass = encoder.begin_compute_pass(&Default::default());
                        pass.set_pipeline(&kernel.pipeline);
                        pass.set_bind_group(0, &bind_group, &[]);
                        // Dispatch for 2D: (n_contingencies, n_branches) with workgroup_size(8,8)
                        let wg_x = (n_contingencies + 7) / 8;
                        let wg_y = (n_branches + 7) / 8;
                        pass.dispatch_workgroups(wg_x as u32, wg_y as u32, 1);
                    }
                    ctx.queue.submit(Some(encoder.finish()));
                    let _ = ctx.device.poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: None,
                    });
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_context_init,
    bench_buffer_upload,
    bench_buffer_download,
    bench_compute_dispatch,
    bench_roundtrip,
    bench_cpu_vs_gpu,
    bench_power_mismatch_scale,
    bench_memory_bandwidth,
    bench_monte_carlo_capacity,
    bench_lodf_screening,
);
criterion_main!(benches);
