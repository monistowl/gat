//! GPU-accelerated branch power flow calculation.
//!
//! Provides GPU acceleration for computing branch flows in ADMM and other OPF solvers.
//! Falls back to CPU when GPU is unavailable.

#[cfg(feature = "gpu")]
use gat_gpu::GpuContext;

/// GPU-accelerated branch flow calculator.
///
/// Wraps the WGSL compute shader for parallel branch flow computation.
/// Falls back to CPU when GPU is unavailable or disabled.
pub struct GpuBranchFlowCalculator {
    /// Cached GPU context (reused across runs)
    #[cfg(feature = "gpu")]
    gpu_context: Option<GpuContext>,
}

impl GpuBranchFlowCalculator {
    /// Create a new GPU branch flow calculator.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "gpu")]
            gpu_context: None,
        }
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
}

impl Default for GpuBranchFlowCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_branch_flow_calculator_creation() {
        let calc = GpuBranchFlowCalculator::new();
        // Should not panic regardless of GPU presence
        let _available = calc.is_gpu_available();
    }
}
