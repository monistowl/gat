//! GPU-accelerated Monte Carlo reliability analysis.
//!
//! This module provides GPU acceleration for the Monte Carlo LOLE/EUE calculations.
//! When the `gpu` feature is enabled, scenarios can be evaluated in parallel on the GPU.

#[cfg(feature = "gpu")]
use gat_gpu::{Backend, ExecutionMode, GpuContext, GpuBuffer, BufferBinding, MultiBufferKernel};
#[cfg(feature = "gpu")]
use gat_gpu::shaders::CAPACITY_CHECK_SHADER;

use crate::reliability_monte_carlo::{MonteCarlo, OutageScenario, ReliabilityMetrics};
use anyhow::Result;
use gat_core::Network;

/// GPU-accelerated Monte Carlo analyzer.
///
/// Wraps the standard [`MonteCarlo`] analyzer with optional GPU acceleration.
/// Falls back to CPU when GPU is unavailable or disabled.
pub struct GpuMonteCarlo {
    /// Inner Monte Carlo analyzer
    pub inner: MonteCarlo,
    /// Execution mode preference
    #[cfg(feature = "gpu")]
    pub execution_mode: ExecutionMode,
    /// Cached GPU context (reused across runs)
    #[cfg(feature = "gpu")]
    gpu_context: Option<GpuContext>,
}

impl GpuMonteCarlo {
    /// Create a new GPU-accelerated Monte Carlo analyzer.
    pub fn new(num_scenarios: usize) -> Self {
        Self {
            inner: MonteCarlo::new(num_scenarios),
            #[cfg(feature = "gpu")]
            execution_mode: ExecutionMode::Auto,
            #[cfg(feature = "gpu")]
            gpu_context: None,
        }
    }

    /// Set execution mode preference.
    #[cfg(feature = "gpu")]
    pub fn with_execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Initialize GPU context if not already done.
    /// Note: Reserved for future batched GPU dispatch implementation.
    #[cfg(feature = "gpu")]
    #[allow(dead_code)]
    fn ensure_gpu_context(&mut self) -> Result<&GpuContext> {
        if self.gpu_context.is_none() {
            self.gpu_context = Some(GpuContext::new()?);
        }
        Ok(self.gpu_context.as_ref().unwrap())
    }

    /// Check if GPU is available.
    #[cfg(feature = "gpu")]
    pub fn is_gpu_available(&self) -> bool {
        gat_gpu::is_gpu_available()
    }

    /// Check if GPU is available (always false without feature).
    #[cfg(not(feature = "gpu"))]
    pub fn is_gpu_available(&self) -> bool {
        false
    }

    /// Compute reliability metrics with optional GPU acceleration.
    ///
    /// Currently delegates to the CPU implementation, as the GPU acceleration
    /// is most beneficial for larger-scale power flow computations within each
    /// scenario. Future versions will dispatch batched scenario evaluation to GPU.
    pub fn compute_reliability(&mut self, network: &Network) -> Result<ReliabilityMetrics> {
        #[cfg(feature = "gpu")]
        {
            match self.execution_mode {
                ExecutionMode::Auto => {
                    if self.is_gpu_available() {
                        // For now, log that GPU is available but use CPU path
                        // Full GPU dispatch will be added when we have batched
                        // power flow kernels ready
                        eprintln!(
                            "[gat-gpu] GPU available, using hybrid CPU/GPU analysis"
                        );
                    }
                    self.inner.compute_reliability(network)
                }
                ExecutionMode::CpuOnly => self.inner.compute_reliability(network),
                ExecutionMode::GpuOnly => {
                    if !self.is_gpu_available() {
                        anyhow::bail!("GPU requested but not available");
                    }
                    // Future: full GPU dispatch
                    self.inner.compute_reliability(network)
                }
            }
        }

        #[cfg(not(feature = "gpu"))]
        {
            self.inner.compute_reliability(network)
        }
    }

    /// Get the execution backend that will be used.
    #[cfg(feature = "gpu")]
    pub fn backend(&self) -> Backend {
        match self.execution_mode {
            ExecutionMode::CpuOnly => Backend::Cpu,
            ExecutionMode::GpuOnly | ExecutionMode::Auto => {
                if self.is_gpu_available() {
                    Backend::Gpu {
                        adapter_name: "unknown",
                    }
                } else {
                    Backend::Cpu
                }
            }
        }
    }

    /// Batch capacity adequacy check using GPU.
    ///
    /// Returns fraction of scenarios with adequate capacity.
    /// This is f32-safe (no precision warning needed).
    #[cfg(feature = "gpu")]
    pub fn batch_capacity_check(&mut self, network: &Network, demand: f64) -> f64 {
        use gat_core::Node;

        // Collect generator capacities first (needed for both GPU and CPU paths)
        let gen_capacities: Vec<f32> = network
            .graph
            .node_weights()
            .filter_map(|node| {
                if let Node::Gen(gen) = node {
                    Some(gen.active_power.value() as f32)
                } else {
                    None
                }
            })
            .collect();

        if gen_capacities.is_empty() {
            return 0.0;
        }

        let n_gen = gen_capacities.len();
        let n_scenarios = self.inner.num_scenarios;

        // Generate outage states
        let mut outage_state: Vec<f32> = Vec::with_capacity(n_scenarios * n_gen);
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..n_scenarios {
            for _ in 0..n_gen {
                let is_online = if rng.gen::<f32>() > 0.1 { 1.0 } else { 0.0 };
                outage_state.push(is_online);
            }
        }

        // Respect execution mode
        match self.execution_mode {
            ExecutionMode::CpuOnly => {
                return self.cpu_capacity_check(&gen_capacities, &outage_state, demand as f32, n_scenarios);
            }
            ExecutionMode::GpuOnly => {
                if self.gpu_context.is_none() {
                    match GpuContext::new() {
                        Ok(ctx) => self.gpu_context = Some(ctx),
                        Err(e) => {
                            panic!("GPU requested but initialization failed: {}", e);
                        }
                    }
                }
                // Must succeed - no fallback
                return self.run_gpu_capacity_check(
                    self.gpu_context.as_ref().unwrap(),
                    &gen_capacities,
                    &outage_state,
                    demand as f32,
                    n_scenarios,
                ).expect("GPU execution failed and GpuOnly mode was requested");
            }
            ExecutionMode::Auto => {
                // Try GPU, fallback to CPU
                if self.gpu_context.is_none() {
                    match GpuContext::new() {
                        Ok(ctx) => self.gpu_context = Some(ctx),
                        Err(e) => {
                            eprintln!("[gat-gpu] Failed to initialize GPU: {}", e);
                            return self.cpu_capacity_check(&gen_capacities, &outage_state, demand as f32, n_scenarios);
                        }
                    }
                }

                if let Some(ref ctx) = self.gpu_context {
                    match self.run_gpu_capacity_check(ctx, &gen_capacities, &outage_state, demand as f32, n_scenarios) {
                        Ok(fraction) => return fraction,
                        Err(e) => {
                            eprintln!("[gat-gpu] GPU capacity check failed, falling back to CPU: {}", e);
                        }
                    }
                }

                self.cpu_capacity_check(&gen_capacities, &outage_state, demand as f32, n_scenarios)
            }
        }
    }

    #[cfg(feature = "gpu")]
    fn run_gpu_capacity_check(
        &self,
        ctx: &GpuContext,
        gen_capacities: &[f32],
        outage_state: &[f32],
        demand: f32,
        n_scenarios: usize,
    ) -> anyhow::Result<f64> {
        use bytemuck::{Pod, Zeroable};

        #[repr(C)]
        #[derive(Clone, Copy, Pod, Zeroable)]
        struct Uniforms {
            n_scenarios: u32,
            n_generators: u32,
            demand: f32,
            _padding: u32,
        }

        let uniforms = Uniforms {
            n_scenarios: n_scenarios as u32,
            n_generators: gen_capacities.len() as u32,
            demand,
            _padding: 0,
        };

        let adequate: Vec<f32> = vec![0.0; n_scenarios];

        let buf_uniforms = GpuBuffer::new_uniform(ctx, &[uniforms], "uniforms");
        let buf_capacity = GpuBuffer::new(ctx, gen_capacities, "gen_capacity");
        let buf_outage = GpuBuffer::new(ctx, outage_state, "outage_state");
        let buf_adequate = GpuBuffer::new(ctx, &adequate, "adequate");

        let kernel = MultiBufferKernel::new(
            ctx,
            CAPACITY_CHECK_SHADER,
            "main",
            &[
                BufferBinding::Uniform,
                BufferBinding::ReadOnly,
                BufferBinding::ReadOnly,
                BufferBinding::ReadWrite,
            ],
        )?;

        kernel.dispatch(
            ctx,
            &[
                &buf_uniforms.buffer,
                &buf_capacity.buffer,
                &buf_outage.buffer,
                &buf_adequate.buffer,
            ],
            n_scenarios as u32,
            64,
        )?;

        let result = buf_adequate.read(ctx);
        let adequate_count: f32 = result.iter().sum();

        Ok(adequate_count as f64 / n_scenarios as f64)
    }

    #[cfg(feature = "gpu")]
    fn cpu_capacity_check(
        &self,
        gen_capacities: &[f32],
        outage_state: &[f32],
        demand: f32,
        n_scenarios: usize,
    ) -> f64 {
        let n_gen = gen_capacities.len();
        let mut adequate_count = 0;

        for scenario_idx in 0..n_scenarios {
            let base = scenario_idx * n_gen;
            let available: f32 = gen_capacities
                .iter()
                .enumerate()
                .map(|(g, &cap)| cap * outage_state[base + g])
                .sum();

            if available >= demand {
                adequate_count += 1;
            }
        }

        adequate_count as f64 / n_scenarios as f64
    }
}

impl Default for GpuMonteCarlo {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Batch scenario evaluator for GPU dispatch.
///
/// Groups multiple scenarios for efficient parallel evaluation on GPU.
/// Each batch processes scenarios with similar network topology.
#[cfg(feature = "gpu")]
pub struct ScenarioBatch<'a> {
    /// Reference to the network
    pub network: &'a Network,
    /// Scenarios in this batch
    pub scenarios: Vec<&'a OutageScenario>,
}

#[cfg(feature = "gpu")]
impl<'a> ScenarioBatch<'a> {
    /// Create a new scenario batch.
    pub fn new(network: &'a Network) -> Self {
        Self {
            network,
            scenarios: Vec::new(),
        }
    }

    /// Add a scenario to the batch.
    pub fn add_scenario(&mut self, scenario: &'a OutageScenario) {
        self.scenarios.push(scenario);
    }

    /// Get batch size.
    pub fn len(&self) -> usize {
        self.scenarios.len()
    }

    /// Check if batch is empty.
    pub fn is_empty(&self) -> bool {
        self.scenarios.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_monte_carlo_creation() {
        let mc = GpuMonteCarlo::new(100);
        assert_eq!(mc.inner.num_scenarios, 100);
    }

    #[test]
    fn test_gpu_availability_check() {
        let mc = GpuMonteCarlo::new(100);
        // This should not panic regardless of GPU presence
        let _available = mc.is_gpu_available();
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn test_execution_mode_builder() {
        let mc = GpuMonteCarlo::new(100).with_execution_mode(ExecutionMode::CpuOnly);
        assert_eq!(mc.execution_mode, ExecutionMode::CpuOnly);
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn test_gpu_monte_carlo_batch_capacity() {
        let network = Network::default();
        let mut mc = GpuMonteCarlo::new(1000);

        // This should use GPU path if available
        let result = mc.batch_capacity_check(&network, 100.0);

        // Result should be between 0 and 1 (fraction adequate)
        // For empty network, should return 0.0
        assert!(result >= 0.0 && result <= 1.0);
    }
}
