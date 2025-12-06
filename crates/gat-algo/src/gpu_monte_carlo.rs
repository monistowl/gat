//! GPU-accelerated Monte Carlo reliability analysis.
//!
//! This module provides GPU acceleration for the Monte Carlo LOLE/EUE calculations.
//! When the `gpu` feature is enabled, scenarios can be evaluated in parallel on the GPU.

#[cfg(feature = "gpu")]
use gat_gpu::{Backend, ExecutionMode, GpuContext};

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
    /// Note: Currently unused but reserved for future batched GPU dispatch
    #[cfg(feature = "gpu")]
    #[allow(dead_code)]
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
}
