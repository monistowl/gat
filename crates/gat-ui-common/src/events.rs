//! Event types for reactive UI updates.

use std::path::PathBuf;

use crate::JobId;

/// Events emitted by the workspace when state changes.
#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    /// A network was loaded successfully.
    NetworkLoaded {
        path: PathBuf,
        n_bus: usize,
        n_branch: usize,
        n_gen: usize,
    },

    /// The network was unloaded.
    NetworkUnloaded,

    /// All cached analysis results were invalidated.
    CacheInvalidated,

    /// An analysis completed and results are cached.
    AnalysisComplete { kind: AnalysisKind },
}

/// Types of analysis that can be performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalysisKind {
    /// Y-bus admittance matrix computation.
    YBus,

    /// DC power flow solution.
    DcPowerFlow,

    /// AC power flow solution.
    AcPowerFlow,

    /// DC optimal power flow.
    DcOpf,

    /// AC optimal power flow.
    AcOpf,

    /// N-1 contingency screening.
    N1Screening,

    /// Power transfer distribution factors.
    Ptdf,

    /// Line outage distribution factors.
    Lodf,
}

impl std::fmt::Display for AnalysisKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisKind::YBus => write!(f, "Y-bus"),
            AnalysisKind::DcPowerFlow => write!(f, "DC Power Flow"),
            AnalysisKind::AcPowerFlow => write!(f, "AC Power Flow"),
            AnalysisKind::DcOpf => write!(f, "DC-OPF"),
            AnalysisKind::AcOpf => write!(f, "AC-OPF"),
            AnalysisKind::N1Screening => write!(f, "N-1 Screening"),
            AnalysisKind::Ptdf => write!(f, "PTDF"),
            AnalysisKind::Lodf => write!(f, "LODF"),
        }
    }
}

/// Events emitted by the job tracker.
#[derive(Debug, Clone)]
pub enum JobEvent {
    /// A new job was started.
    Started {
        id: JobId,
        kind: AnalysisKind,
    },

    /// A job made progress.
    Progress {
        id: JobId,
        fraction: f32,
        message: Option<String>,
    },

    /// A job completed successfully.
    Completed {
        id: JobId,
        kind: AnalysisKind,
    },

    /// A job failed.
    Failed {
        id: JobId,
        error: String,
    },

    /// A job was cancelled.
    Cancelled {
        id: JobId,
    },
}
