//! Dispatch abstraction for GPU/CPU execution.

use anyhow::Result;

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
