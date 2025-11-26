//! Data types for the Operations pane
//!
//! This module contains the core data structures used across
//! different operation states (batch, allocation, reliability).

/// Type of operation being viewed/performed
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OperationType {
    #[default]
    Batch,
    Allocation,
    Reliability,
}

impl OperationType {
    pub fn label(&self) -> &'static str {
        match self {
            OperationType::Batch => "Batch",
            OperationType::Allocation => "Allocation",
            OperationType::Reliability => "Reliability",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            OperationType::Batch => 0,
            OperationType::Allocation => 1,
            OperationType::Reliability => 2,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            OperationType::Batch => OperationType::Allocation,
            OperationType::Allocation => OperationType::Reliability,
            OperationType::Reliability => OperationType::Batch,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            OperationType::Batch => OperationType::Reliability,
            OperationType::Allocation => OperationType::Batch,
            OperationType::Reliability => OperationType::Allocation,
        }
    }
}

/// Status of a batch job
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JobStatus {
    #[default]
    Queued,
    Running,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            JobStatus::Queued => "⏳",
            JobStatus::Running => "⟳",
            JobStatus::Completed => "✓",
            JobStatus::Failed => "✗",
        }
    }
}

/// Batch job entry
#[derive(Clone, Debug)]
pub struct BatchJob {
    pub id: String,
    pub name: String,
    pub status: JobStatus,
    pub progress: u32,
    pub start_time: String,
    pub est_completion: String,
}

impl BatchJob {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            status: JobStatus::Queued,
            progress: 0,
            start_time: String::new(),
            est_completion: String::new(),
        }
    }

    pub fn with_status(mut self, status: JobStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_progress(mut self, progress: u32) -> Self {
        self.progress = progress;
        self
    }

    pub fn with_times(mut self, start: impl Into<String>, est: impl Into<String>) -> Self {
        self.start_time = start.into();
        self.est_completion = est.into();
        self
    }

    pub fn display_line(&self) -> String {
        format!(
            "{} {} ({}%)",
            self.status.symbol(),
            self.name,
            self.progress
        )
    }
}

/// Allocation result with comprehensive metrics
#[derive(Clone, Debug)]
pub struct AllocationResult {
    pub node_id: String,
    pub rents: f64,
    pub contribution: f64,
    pub allocation_factor: f64,
    pub revenue_adequacy: f64,
    pub cost_recovery: f64,
    pub surplus_deficit: f64,
}

impl AllocationResult {
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            rents: 0.0,
            contribution: 0.0,
            allocation_factor: 0.0,
            revenue_adequacy: 0.0,
            cost_recovery: 0.0,
            surplus_deficit: 0.0,
        }
    }

    pub fn display_line(&self) -> String {
        format!("{}: ${:.2}", self.node_id, self.rents)
    }

    pub fn details(&self) -> String {
        format!(
            "Node: {}\nRents: ${:.2}\nContribution: ${:.2}\nAllocation Factor: {:.2}\nRevenue Adequacy: {:.1}%\nCost Recovery: {:.1}%\nSurplus/Deficit: ${:.2}",
            self.node_id,
            self.rents,
            self.contribution,
            self.allocation_factor,
            self.revenue_adequacy,
            self.cost_recovery,
            self.surplus_deficit,
        )
    }
}

/// Status of a reliability metric
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MetricStatus {
    Excellent,
    #[default]
    Good,
    Warning,
    Critical,
}

impl MetricStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            MetricStatus::Excellent => "✓",
            MetricStatus::Good => "◐",
            MetricStatus::Warning => "⚠",
            MetricStatus::Critical => "✗",
        }
    }
}

/// Reliability metric entry
#[derive(Clone, Debug)]
pub struct ReliabilityMetric {
    pub metric_name: String,
    pub value: f64,
    pub unit: String,
    pub status: MetricStatus,
}

impl ReliabilityMetric {
    pub fn new(name: impl Into<String>, value: f64, unit: impl Into<String>) -> Self {
        Self {
            metric_name: name.into(),
            value,
            unit: unit.into(),
            status: MetricStatus::Good,
        }
    }

    pub fn with_status(mut self, status: MetricStatus) -> Self {
        self.status = status;
        self
    }

    pub fn display_line(&self) -> String {
        format!("{} {:.1}{}", self.metric_name, self.value, self.unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_type_navigation() {
        assert_eq!(OperationType::Batch.next(), OperationType::Allocation);
        assert_eq!(OperationType::Allocation.next(), OperationType::Reliability);
        assert_eq!(OperationType::Reliability.next(), OperationType::Batch);

        assert_eq!(OperationType::Batch.prev(), OperationType::Reliability);
        assert_eq!(OperationType::Allocation.prev(), OperationType::Batch);
        assert_eq!(OperationType::Reliability.prev(), OperationType::Allocation);
    }

    #[test]
    fn test_operation_type_label() {
        assert_eq!(OperationType::Batch.label(), "Batch");
        assert_eq!(OperationType::Allocation.label(), "Allocation");
        assert_eq!(OperationType::Reliability.label(), "Reliability");
    }

    #[test]
    fn test_job_status_symbol() {
        assert_eq!(JobStatus::Queued.symbol(), "⏳");
        assert_eq!(JobStatus::Running.symbol(), "⟳");
        assert_eq!(JobStatus::Completed.symbol(), "✓");
        assert_eq!(JobStatus::Failed.symbol(), "✗");
    }

    #[test]
    fn test_batch_job_builder() {
        let job = BatchJob::new("j001", "Test Job")
            .with_status(JobStatus::Running)
            .with_progress(50)
            .with_times("2024-01-01 10:00", "2024-01-01 11:00");

        assert_eq!(job.id, "j001");
        assert_eq!(job.name, "Test Job");
        assert_eq!(job.status, JobStatus::Running);
        assert_eq!(job.progress, 50);
    }

    #[test]
    fn test_batch_job_display() {
        let job = BatchJob::new("j001", "Power Flow")
            .with_status(JobStatus::Running)
            .with_progress(65);

        assert_eq!(job.display_line(), "⟳ Power Flow (65%)");
    }

    #[test]
    fn test_allocation_result_display() {
        let result = AllocationResult {
            node_id: "NODE_A".into(),
            rents: 1250.50,
            contribution: 45.2,
            allocation_factor: 0.85,
            revenue_adequacy: 95.3,
            cost_recovery: 98.2,
            surplus_deficit: 152.40,
        };

        assert_eq!(result.display_line(), "NODE_A: $1250.50");
        assert!(result.details().contains("NODE_A"));
        assert!(result.details().contains("1250.50"));
    }

    #[test]
    fn test_metric_status_symbol() {
        assert_eq!(MetricStatus::Excellent.symbol(), "✓");
        assert_eq!(MetricStatus::Good.symbol(), "◐");
        assert_eq!(MetricStatus::Warning.symbol(), "⚠");
        assert_eq!(MetricStatus::Critical.symbol(), "✗");
    }

    #[test]
    fn test_reliability_metric_builder() {
        let metric = ReliabilityMetric::new("LOLE", 9.2, "h/yr").with_status(MetricStatus::Warning);

        assert_eq!(metric.metric_name, "LOLE");
        assert_eq!(metric.value, 9.2);
        assert_eq!(metric.unit, "h/yr");
        assert_eq!(metric.status, MetricStatus::Warning);
    }

    #[test]
    fn test_reliability_metric_display() {
        let metric = ReliabilityMetric::new("Deliverability", 85.5, "%");
        assert_eq!(metric.display_line(), "Deliverability 85.5%");
    }
}
