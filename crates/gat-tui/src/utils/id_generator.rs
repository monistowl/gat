/// Unique ID generation for tasks and commands
///
/// Provides a convenient wrapper around uuid for generating unique identifiers.

use uuid::Uuid;
use std::fmt;

/// A unique task identifier
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct TaskId(Uuid);

impl TaskId {
    /// Generate a new random task ID
    pub fn new() -> Self {
        TaskId(Uuid::new_v4())
    }

    /// Create a task ID from a UUID string
    pub fn from_string(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(TaskId)
    }

    /// Get the string representation
    pub fn as_str(&self) -> String {
        self.0.simple().to_string()
    }

    /// Get the underlying UUID
    pub fn uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        TaskId::new()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.simple())
    }
}

/// ID generator service for various entity types
pub struct IdGenerator;

impl IdGenerator {
    /// Generate a new task ID
    pub fn task_id() -> TaskId {
        TaskId::new()
    }

    /// Generate a new command ID
    pub fn command_id() -> String {
        format!("cmd_{}", Uuid::new_v4().simple())
    }

    /// Generate a new notification ID
    pub fn notification_id() -> String {
        format!("notif_{}", Uuid::new_v4().simple())
    }

    /// Generate a new dataset ID
    pub fn dataset_id() -> String {
        format!("data_{}", Uuid::new_v4().simple())
    }

    /// Generate a new run ID
    pub fn run_id() -> String {
        format!("run_{}", Uuid::new_v4().simple())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_generation() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_task_id_default() {
        let id = TaskId::default();
        assert!(!id.as_str().is_empty());
    }

    #[test]
    fn test_task_id_string_parsing() {
        let id1 = TaskId::new();
        let id_str = id1.as_str();
        let id2 = TaskId::from_string(&id_str).expect("should parse");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_task_id_display() {
        let id = TaskId::new();
        let s = format!("{}", id);
        assert!(!s.is_empty());
        assert_eq!(s.len(), 32); // UUID simple format is 32 hex chars
    }

    #[test]
    fn test_id_generator_command_id() {
        let cmd_id = IdGenerator::command_id();
        assert!(cmd_id.starts_with("cmd_"));
    }

    #[test]
    fn test_id_generator_notification_id() {
        let notif_id = IdGenerator::notification_id();
        assert!(notif_id.starts_with("notif_"));
    }

    #[test]
    fn test_id_generator_dataset_id() {
        let data_id = IdGenerator::dataset_id();
        assert!(data_id.starts_with("data_"));
    }

    #[test]
    fn test_id_generator_run_id() {
        let run_id = IdGenerator::run_id();
        assert!(run_id.starts_with("run_"));
    }

    #[test]
    fn test_task_id_uniqueness() {
        let ids: Vec<TaskId> = (0..100).map(|_| TaskId::new()).collect();
        let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, 100);
    }

    #[test]
    fn test_task_id_serialization() {
        use serde_json;
        let id = TaskId::new();
        let json = serde_json::to_string(&id).expect("should serialize");
        let id2: TaskId = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(id, id2);
    }
}
