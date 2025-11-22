use crate::DatasetEntry;
use async_trait::async_trait;

/// Error type for query operations
#[derive(Debug, Clone)]
pub enum QueryError {
    NotFound(String),
    ConnectionFailed(String),
    Timeout,
    ParseError(String),
    Unknown(String),
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QueryError::NotFound(msg) => write!(f, "Not found: {}", msg),
            QueryError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            QueryError::Timeout => write!(f, "Query timed out"),
            QueryError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            QueryError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for QueryError {}

/// Trait for querying application data
#[async_trait]
pub trait QueryBuilder: Send + Sync {
    /// Fetch all available datasets
    async fn get_datasets(&self) -> Result<Vec<DatasetEntry>, QueryError>;

    /// Fetch a specific dataset by ID
    async fn get_dataset(&self, id: &str) -> Result<DatasetEntry, QueryError>;

    /// Fetch all workflows
    async fn get_workflows(&self) -> Result<Vec<crate::data::Workflow>, QueryError>;

    /// Fetch system metrics
    async fn get_metrics(&self) -> Result<crate::data::SystemMetrics, QueryError>;

    /// Fetch pipeline configuration
    async fn get_pipeline_config(&self) -> Result<String, QueryError>;

    /// Fetch list of available commands
    async fn get_commands(&self) -> Result<Vec<String>, QueryError>;
}

/// Mock implementation using fixture data
pub struct MockQueryBuilder;

#[async_trait]
impl QueryBuilder for MockQueryBuilder {
    async fn get_datasets(&self) -> Result<Vec<DatasetEntry>, QueryError> {
        Ok(crate::create_fixture_datasets())
    }

    async fn get_dataset(&self, id: &str) -> Result<DatasetEntry, QueryError> {
        crate::create_fixture_datasets()
            .into_iter()
            .find(|d| d.id == id)
            .ok_or_else(|| QueryError::NotFound(format!("Dataset {} not found", id)))
    }

    async fn get_workflows(&self) -> Result<Vec<crate::data::Workflow>, QueryError> {
        Ok(vec![])  // Empty for now, will populate with fixtures later
    }

    async fn get_metrics(&self) -> Result<crate::data::SystemMetrics, QueryError> {
        Ok(crate::data::SystemMetrics {
            deliverability_score: 85.5,
            lole_hours_per_year: 9.2,
            eue_mwh_per_year: 15.3,
        })
    }

    async fn get_pipeline_config(&self) -> Result<String, QueryError> {
        Ok(serde_json::json!({
            "name": "Default Pipeline",
            "stages": [
                { "name": "Data Load", "type": "source" },
                { "name": "Data Validation", "type": "transform" },
                { "name": "Power Flow", "type": "analysis" },
                { "name": "Results Export", "type": "sink" }
            ]
        }).to_string())
    }

    async fn get_commands(&self) -> Result<Vec<String>, QueryError> {
        Ok(vec![
            "datasets list".to_string(),
            "datasets upload".to_string(),
            "pipeline validate".to_string(),
            "pipeline run".to_string(),
            "batch list".to_string(),
            "batch submit".to_string(),
            "analytics reliability".to_string(),
            "analytics powerflow".to_string(),
            "scenario generate".to_string(),
            "scenario analyze".to_string(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_get_datasets() {
        let qb = MockQueryBuilder;
        let result = qb.get_datasets().await;
        assert!(result.is_ok());
        let datasets = result.unwrap();
        assert_eq!(datasets.len(), 3);  // Three fixture datasets
    }

    #[tokio::test]
    async fn test_mock_get_dataset_found() {
        let qb = MockQueryBuilder;
        let result = qb.get_dataset("opsd-2024").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "OPSD Snapshot");
    }

    #[tokio::test]
    async fn test_mock_get_dataset_not_found() {
        let qb = MockQueryBuilder;
        let result = qb.get_dataset("nonexistent").await;
        assert!(result.is_err());
        match result {
            Err(QueryError::NotFound(_)) => (),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_mock_get_metrics() {
        let qb = MockQueryBuilder;
        let result = qb.get_metrics().await;
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert!(metrics.deliverability_score > 0.0);
        assert!(metrics.deliverability_score <= 100.0);
    }

    #[tokio::test]
    async fn test_mock_get_workflows() {
        let qb = MockQueryBuilder;
        let result = qb.get_workflows().await;
        assert!(result.is_ok());
        let _workflows = result.unwrap();
        // Empty for now, but method is available
    }
}
