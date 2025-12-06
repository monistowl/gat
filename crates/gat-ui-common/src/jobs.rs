//! Background job tracking with progress channels.
//!
//! The [`JobTracker`] manages async jobs, providing progress updates via tokio channels.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tokio::sync::{broadcast, oneshot, watch};
use uuid::Uuid;

use crate::events::{AnalysisKind, JobEvent};

/// Unique identifier for a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(Uuid);

impl JobId {
    /// Create a new random job ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Progress information for a running job.
#[derive(Debug, Clone)]
pub struct Progress {
    /// Completion fraction (0.0 to 1.0).
    pub fraction: f32,
    /// Optional status message.
    pub message: Option<String>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            fraction: 0.0,
            message: None,
        }
    }
}

impl Progress {
    /// Create progress with a message.
    pub fn with_message(fraction: f32, message: impl Into<String>) -> Self {
        Self {
            fraction,
            message: Some(message.into()),
        }
    }

    /// Create progress without a message.
    pub fn at(fraction: f32) -> Self {
        Self {
            fraction,
            message: None,
        }
    }
}

/// Handle to a running job.
///
/// Provides progress monitoring and result awaiting.
pub struct JobHandle {
    /// Unique job identifier.
    pub id: JobId,

    /// Receiver for progress updates.
    pub progress: watch::Receiver<Progress>,

    /// Receiver for the final result.
    pub result: oneshot::Receiver<JobResult>,
}

/// Result of a completed job.
#[derive(Debug, Clone)]
pub enum JobResult {
    /// Job completed successfully.
    Success {
        kind: AnalysisKind,
        message: Option<String>,
    },

    /// Job failed with an error.
    Failed { error: String },

    /// Job was cancelled.
    Cancelled,
}

impl JobResult {
    /// Check if the job succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, JobResult::Success { .. })
    }
}

/// Internal state for a tracked job.
struct JobState {
    kind: AnalysisKind,
    #[allow(dead_code)] // For future job duration reporting
    started_at: DateTime<Utc>,
    progress_tx: watch::Sender<Progress>,
    result_tx: Option<oneshot::Sender<JobResult>>,
}

/// Manages background jobs with progress tracking.
pub struct JobTracker {
    /// Active jobs by ID.
    jobs: DashMap<JobId, JobState>,

    /// Event broadcaster for job lifecycle events.
    events_tx: broadcast::Sender<JobEvent>,
}

impl JobTracker {
    /// Create a new job tracker.
    pub fn new() -> Self {
        let (events_tx, _) = broadcast::channel(256);
        Self {
            jobs: DashMap::new(),
            events_tx,
        }
    }

    /// Create a new job and return its handle.
    pub fn create(&self, kind: AnalysisKind) -> JobHandle {
        let id = JobId::new();
        let (progress_tx, progress_rx) = watch::channel(Progress::default());
        let (result_tx, result_rx) = oneshot::channel();

        let state = JobState {
            kind,
            started_at: Utc::now(),
            progress_tx,
            result_tx: Some(result_tx),
        };

        self.jobs.insert(id, state);

        // Notify subscribers
        let _ = self.events_tx.send(JobEvent::Started { id, kind });

        JobHandle {
            id,
            progress: progress_rx,
            result: result_rx,
        }
    }

    /// Update progress for a job.
    pub fn update_progress(&self, id: JobId, progress: Progress) {
        if let Some(state) = self.jobs.get(&id) {
            let _ = state.progress_tx.send(progress.clone());
            let _ = self.events_tx.send(JobEvent::Progress {
                id,
                fraction: progress.fraction,
                message: progress.message,
            });
        }
    }

    /// Complete a job successfully.
    pub fn complete(&self, id: JobId, message: Option<String>) {
        if let Some((_, mut state)) = self.jobs.remove(&id) {
            let kind = state.kind;

            // Send final progress
            let _ = state.progress_tx.send(Progress::at(1.0));

            // Send result
            if let Some(tx) = state.result_tx.take() {
                let _ = tx.send(JobResult::Success {
                    kind,
                    message: message.clone(),
                });
            }

            // Notify subscribers
            let _ = self.events_tx.send(JobEvent::Completed { id, kind });
        }
    }

    /// Mark a job as failed.
    pub fn fail(&self, id: JobId, error: impl Into<String>) {
        let error = error.into();
        if let Some((_, mut state)) = self.jobs.remove(&id) {
            if let Some(tx) = state.result_tx.take() {
                let _ = tx.send(JobResult::Failed {
                    error: error.clone(),
                });
            }

            let _ = self.events_tx.send(JobEvent::Failed { id, error });
        }
    }

    /// Cancel a job.
    pub fn cancel(&self, id: JobId) {
        if let Some((_, mut state)) = self.jobs.remove(&id) {
            if let Some(tx) = state.result_tx.take() {
                let _ = tx.send(JobResult::Cancelled);
            }

            let _ = self.events_tx.send(JobEvent::Cancelled { id });
        }
    }

    /// Get the number of active jobs.
    pub fn active_count(&self) -> usize {
        self.jobs.len()
    }

    /// List active job IDs.
    pub fn active_jobs(&self) -> Vec<JobId> {
        self.jobs.iter().map(|r| *r.key()).collect()
    }

    /// Subscribe to job events.
    pub fn subscribe(&self) -> broadcast::Receiver<JobEvent> {
        self.events_tx.subscribe()
    }
}

impl Default for JobTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension to create an Arc-wrapped tracker.
impl JobTracker {
    /// Create a new tracker wrapped in Arc for sharing.
    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_lifecycle() {
        let tracker = JobTracker::new();
        let handle = tracker.create(AnalysisKind::DcPowerFlow);

        assert_eq!(tracker.active_count(), 1);

        tracker.update_progress(handle.id, Progress::at(0.5));
        tracker.complete(handle.id, Some("Done".to_string()));

        assert_eq!(tracker.active_count(), 0);
    }

    #[test]
    fn test_job_failure() {
        let tracker = JobTracker::new();
        let handle = tracker.create(AnalysisKind::AcPowerFlow);

        tracker.fail(handle.id, "Solver diverged");

        assert_eq!(tracker.active_count(), 0);
    }

    #[test]
    fn test_job_cancel() {
        let tracker = JobTracker::new();
        let handle = tracker.create(AnalysisKind::N1Screening);

        tracker.cancel(handle.id);

        assert_eq!(tracker.active_count(), 0);
    }
}
