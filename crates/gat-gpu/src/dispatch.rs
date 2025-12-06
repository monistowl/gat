//! Dispatch abstraction for GPU/CPU execution.

use anyhow::Result;

/// GPU floating-point precision mode.
///
/// WGSL only supports f32 natively. For workloads requiring higher precision:
/// - Use `EmulatedDouble` for f64 via double-single arithmetic (2x slower)
/// - Use `SingleWithWarning` when f32 is borderline acceptable
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GpuPrecision {
    /// f32 precision - fastest, suitable for screening/Monte Carlo
    #[default]
    Single,
    /// f32 with user warning about precision limitations
    SingleWithWarning,
    /// Emulated f64 via double-single arithmetic (slower but accurate)
    EmulatedDouble,
    /// Hybrid: f32 compute on GPU, f64 accumulation on CPU
    Hybrid,
}

impl GpuPrecision {
    /// Returns true if this precision mode should warn the user
    pub fn requires_warning(&self) -> bool {
        matches!(self, GpuPrecision::SingleWithWarning)
    }

    /// Returns true if this mode uses emulated double precision
    pub fn is_double(&self) -> bool {
        matches!(self, GpuPrecision::EmulatedDouble)
    }
}

/// Execution preference for compute operations
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Use GPU if available, otherwise fall back to CPU
    #[default]
    Auto,
    /// Always use CPU (useful for testing/debugging)
    CpuOnly,
    /// Always use GPU (error if unavailable)
    GpuOnly,
}

/// Trait for operations that can run on GPU or CPU.
///
/// Implementations should provide both GPU and CPU paths,
/// allowing automatic fallback when GPU is unavailable.
pub trait ComputeDispatch {
    /// Input data type
    type Input;
    /// Output data type
    type Output;

    /// Run computation using the specified execution mode.
    fn dispatch(&self, input: Self::Input, mode: ExecutionMode) -> Result<Self::Output> {
        match mode {
            ExecutionMode::Auto => {
                if crate::is_gpu_available() {
                    self.dispatch_gpu(input)
                } else {
                    self.dispatch_cpu(input)
                }
            }
            ExecutionMode::CpuOnly => self.dispatch_cpu(input),
            ExecutionMode::GpuOnly => self.dispatch_gpu(input),
        }
    }

    /// Force CPU execution
    fn dispatch_cpu(&self, input: Self::Input) -> Result<Self::Output>;

    /// Force GPU execution (fails if no GPU available)
    fn dispatch_gpu(&self, input: Self::Input) -> Result<Self::Output>;
}

/// Result of a dispatched computation with timing info
#[derive(Debug, Clone)]
pub struct DispatchResult<T> {
    /// The computed result
    pub result: T,
    /// Which backend was used
    pub backend: Backend,
    /// Execution time in microseconds
    pub elapsed_us: u64,
}

/// Which compute backend was used
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Cpu,
    Gpu { adapter_name: &'static str },
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backend::Cpu => write!(f, "CPU"),
            Backend::Gpu { adapter_name } => write!(f, "GPU ({})", adapter_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_precision_default() {
        let precision = GpuPrecision::default();
        assert_eq!(precision, GpuPrecision::Single);
    }

    #[test]
    fn test_gpu_precision_requires_warning() {
        assert!(!GpuPrecision::Single.requires_warning());
        assert!(GpuPrecision::SingleWithWarning.requires_warning());
        assert!(!GpuPrecision::EmulatedDouble.requires_warning());
        assert!(!GpuPrecision::Hybrid.requires_warning());
    }

    #[test]
    fn test_gpu_precision_is_double() {
        assert!(!GpuPrecision::Single.is_double());
        assert!(!GpuPrecision::SingleWithWarning.is_double());
        assert!(GpuPrecision::EmulatedDouble.is_double());
        assert!(!GpuPrecision::Hybrid.is_double());
    }
}
