/// Async Event Dispatcher for Background Operations
///
/// This module implements an event-driven system for managing background async
/// operations like data fetching, command execution, and service calls. Events
/// are dispatched through a channel-based architecture with retry logic and
/// error recovery.
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

/// Events that can be dispatched to background handlers
#[derive(Debug, Clone)]
pub enum AsyncEvent {
    // Data loading events
    FetchDatasets,
    FetchDataset(String),
    FetchDatasetDescribe(String),
    FetchDatasetFetch(String, String), // (id, out dir)
    FetchWorkflows,
    FetchMetrics,
    FetchPipelineConfig,
    FetchCommands,

    // Analytics events
    RunAnalytics(String, Vec<(String, String)>), // (analytics_type, options)
    RunScenarioValidation(String),
    RunScenarioMaterialize(String, String), // (template, output)

    // Batch operation events
    RunBatchPowerFlow(String, usize),   // (manifest, max_jobs)
    RunBatchOPF(String, usize, String), // (manifest, max_jobs, solver)

    // Geographic operations
    RunGeoJoin(String, String, String), // (left, right, output)

    // Command execution
    ExecuteCommand(String),
    DescribeRun(String), // run.json path
    ResumeRun(String),   // run.json path

    // Lifecycle
    Shutdown,
}

impl AsyncEvent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::FetchDatasets => "FetchDatasets",
            Self::FetchDataset(_) => "FetchDataset",
            Self::FetchDatasetDescribe(_) => "FetchDatasetDescribe",
            Self::FetchDatasetFetch(_, _) => "FetchDatasetFetch",
            Self::FetchWorkflows => "FetchWorkflows",
            Self::FetchMetrics => "FetchMetrics",
            Self::FetchPipelineConfig => "FetchPipelineConfig",
            Self::FetchCommands => "FetchCommands",
            Self::RunAnalytics(_, _) => "RunAnalytics",
            Self::RunScenarioValidation(_) => "RunScenarioValidation",
            Self::RunScenarioMaterialize(_, _) => "RunScenarioMaterialize",
            Self::RunBatchPowerFlow(_, _) => "RunBatchPowerFlow",
            Self::RunBatchOPF(_, _, _) => "RunBatchOPF",
            Self::RunGeoJoin(_, _, _) => "RunGeoJoin",
            Self::ExecuteCommand(_) => "ExecuteCommand",
            Self::DescribeRun(_) => "DescribeRun",
            Self::ResumeRun(_) => "ResumeRun",
            Self::Shutdown => "Shutdown",
        }
    }

    /// Default timeout for the event. Keep conservative values to avoid
    /// hanging the TUI on long-running background tasks.
    pub fn default_timeout(&self) -> Duration {
        match self {
            // Lightweight metadata calls
            Self::FetchDatasets
            | Self::FetchDataset(_)
            | Self::FetchDatasetDescribe(_)
            | Self::FetchDatasetFetch(_, _)
            | Self::FetchWorkflows
            | Self::FetchMetrics
            | Self::FetchPipelineConfig
            | Self::FetchCommands => Duration::from_secs(30),

            // Analytics and scenarios can be heavier
            Self::RunAnalytics(_, _)
            | Self::RunScenarioValidation(_)
            | Self::RunScenarioMaterialize(_, _)
            | Self::RunGeoJoin(_, _, _) => Duration::from_secs(120),

            // Batch operations may fan out
            Self::RunBatchPowerFlow(_, _) | Self::RunBatchOPF(_, _, _) => Duration::from_secs(180),

            // Arbitrary user commands
            Self::ExecuteCommand(_) | Self::DescribeRun(_) | Self::ResumeRun(_) => {
                Duration::from_secs(180)
            }

            // Shutdown should be immediate but give a small budget
            Self::Shutdown => Duration::from_secs(5),
        }
    }
}

#[cfg(test)]
mod async_event_tests {
    use super::*;

    #[test]
    fn default_timeout_is_reasonable_per_event() {
        assert_eq!(
            AsyncEvent::FetchDatasets.default_timeout(),
            Duration::from_secs(30)
        );
        assert_eq!(
            AsyncEvent::RunAnalytics("r".into(), vec![]).default_timeout(),
            Duration::from_secs(120)
        );
        assert_eq!(
            AsyncEvent::RunBatchPowerFlow("m".into(), 10).default_timeout(),
            Duration::from_secs(180)
        );
        assert_eq!(
            AsyncEvent::Shutdown.default_timeout(),
            Duration::from_secs(5)
        );
    }
}

/// Result of async event processing
#[derive(Debug, Clone)]
pub enum EventResult {
    Success(String),
    Error(String),
    Retry,
}

/// Configuration for event handling
#[derive(Debug, Clone)]
pub struct EventDispatcherConfig {
    pub max_retries: usize,
    pub retry_delay_ms: u64,
    pub channel_capacity: usize,
}

impl Default for EventDispatcherConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 100,
            channel_capacity: 100,
        }
    }
}

/// Handler for processing events with retry logic
pub struct EventDispatcher {
    config: EventDispatcherConfig,
    sender: mpsc::Sender<(AsyncEvent, usize)>, // (event, retry_count)
    receiver: mpsc::Receiver<(AsyncEvent, usize)>,
}

impl EventDispatcher {
    /// Create new event dispatcher with default config
    pub fn new() -> Self {
        Self::with_config(EventDispatcherConfig::default())
    }

    /// Create event dispatcher with custom config
    pub fn with_config(config: EventDispatcherConfig) -> Self {
        let (sender, receiver) = mpsc::channel(config.channel_capacity);
        Self {
            config,
            sender,
            receiver,
        }
    }

    /// Get a clone of the sender for dispatching events
    pub fn sender(&self) -> mpsc::Sender<AsyncEvent> {
        let sender = self.sender.clone();
        // Return a wrapper that doesn't require retry count
        let (tx, mut rx) = mpsc::channel(1);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let _ = sender.send((event, 0)).await;
            }
        });
        tx
    }

    /// Process events with retry logic and backoff
    pub async fn process_events<F>(&mut self, handler: F)
    where
        F: Fn(&AsyncEvent) -> EventResult + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);

        while let Some((event, retry_count)) = self.receiver.recv().await {
            if matches!(event, AsyncEvent::Shutdown) {
                break;
            }

            let handler_clone = Arc::clone(&handler);
            let config = self.config.clone();
            let sender = self.sender.clone();

            tokio::spawn(async move {
                let result = (handler_clone)(&event);

                match result {
                    EventResult::Success(_) => {
                        // Event processed successfully
                    }
                    EventResult::Error(_) => {
                        // Log error but don't retry
                    }
                    EventResult::Retry if retry_count < config.max_retries => {
                        // Retry with exponential backoff
                        let delay = config.retry_delay_ms * (2_u64.pow(retry_count as u32));
                        sleep(Duration::from_millis(delay)).await;
                        let _ = sender.send((event, retry_count + 1)).await;
                    }
                    _ => {
                        // Max retries reached
                    }
                }
            });
        }
    }

    /// Send an event for processing
    pub async fn dispatch(&self, event: AsyncEvent) -> Result<(), String> {
        self.sender
            .send((event, 0))
            .await
            .map_err(|e| format!("Failed to dispatch event: {}", e))
    }

    /// Shutdown the dispatcher
    pub async fn shutdown(&self) -> Result<(), String> {
        self.sender
            .send((AsyncEvent::Shutdown, 0))
            .await
            .map_err(|e| format!("Failed to send shutdown: {}", e))
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Event handler trait for custom event processing
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &AsyncEvent) -> EventResult;
}

/// Background event processor that continuously processes events
pub struct BackgroundEventProcessor {
    dispatcher: EventDispatcher,
}

impl BackgroundEventProcessor {
    /// Create new background processor
    pub fn new() -> Self {
        Self {
            dispatcher: EventDispatcher::new(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: EventDispatcherConfig) -> Self {
        Self {
            dispatcher: EventDispatcher::with_config(config),
        }
    }

    /// Get sender for dispatching events
    pub fn sender(&self) -> mpsc::Sender<AsyncEvent> {
        self.dispatcher.sender()
    }

    /// Start processing events with the given handler
    pub async fn start<F>(&mut self, handler: F)
    where
        F: Fn(&AsyncEvent) -> EventResult + Send + Sync + 'static,
    {
        self.dispatcher.process_events(handler).await
    }

    /// Dispatch an event
    pub async fn dispatch(&self, event: AsyncEvent) -> Result<(), String> {
        self.dispatcher.dispatch(event).await
    }

    /// Request shutdown
    pub async fn shutdown(&self) -> Result<(), String> {
        self.dispatcher.shutdown().await
    }
}

impl Default for BackgroundEventProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod dispatcher_tests {
    use super::*;

    #[test]
    fn test_event_names() {
        assert_eq!(AsyncEvent::FetchDatasets.name(), "FetchDatasets");
        assert_eq!(AsyncEvent::FetchWorkflows.name(), "FetchWorkflows");
        assert_eq!(
            AsyncEvent::RunAnalytics("test".to_string(), vec![]).name(),
            "RunAnalytics"
        );
        assert_eq!(
            AsyncEvent::ExecuteCommand("test".to_string()).name(),
            "ExecuteCommand"
        );
    }

    #[test]
    fn test_dispatcher_creation() {
        let _dispatcher = EventDispatcher::new();
        // Just verify it creates without panicking
    }

    #[test]
    fn test_dispatcher_with_config() {
        let config = EventDispatcherConfig {
            max_retries: 5,
            retry_delay_ms: 50,
            channel_capacity: 200,
        };
        let dispatcher = EventDispatcher::with_config(config.clone());
        assert_eq!(dispatcher.config.max_retries, 5);
        assert_eq!(dispatcher.config.retry_delay_ms, 50);
        assert_eq!(dispatcher.config.channel_capacity, 200);
    }

    #[tokio::test]
    async fn test_event_dispatch() {
        let dispatcher = EventDispatcher::new();
        let result = dispatcher.dispatch(AsyncEvent::FetchDatasets).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_background_processor_creation() {
        let _processor = BackgroundEventProcessor::new();
        // Just verify it creates without panicking
    }

    #[tokio::test]
    async fn test_background_processor_dispatch() {
        let processor = BackgroundEventProcessor::new();
        let result = processor.dispatch(AsyncEvent::FetchMetrics).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_event_processing_success() {
        let mut dispatcher = EventDispatcher::new();
        let processed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let processed_clone = processed.clone();

        let handler = move |_event: &AsyncEvent| {
            processed_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            EventResult::Success("Processed".to_string())
        };

        // Dispatch event
        let sender = dispatcher.sender();
        tokio::spawn(async move {
            let _ = sender.send(AsyncEvent::FetchDatasets).await;
            let _ = sender.send(AsyncEvent::Shutdown).await;
        });

        // Process events
        dispatcher.process_events(handler).await;

        // Give a moment for the background task to complete
        sleep(Duration::from_millis(10)).await;
        assert!(processed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_event_processing_retry() {
        let mut dispatcher = EventDispatcher::new();
        let attempt_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let attempt_count_clone = attempt_count.clone();

        let handler = move |event: &AsyncEvent| {
            if matches!(event, AsyncEvent::FetchDatasets) {
                let count = attempt_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if count < 2 {
                    return EventResult::Retry;
                }
            }
            EventResult::Success("Processed".to_string())
        };

        // Dispatch event
        let sender = dispatcher.sender();
        tokio::spawn(async move {
            let _ = sender.send(AsyncEvent::FetchDatasets).await;
            sleep(Duration::from_millis(500)).await; // Wait for retries
            let _ = sender.send(AsyncEvent::Shutdown).await;
        });

        // Process events
        dispatcher.process_events(handler).await;

        // Verify retries happened (should have attempted at least twice)
        assert!(attempt_count.load(std::sync::atomic::Ordering::SeqCst) >= 2);
    }

    #[tokio::test]
    async fn test_event_result_variants() {
        let success = EventResult::Success("done".to_string());
        let error = EventResult::Error("failed".to_string());
        let retry = EventResult::Retry;

        assert!(matches!(success, EventResult::Success(_)));
        assert!(matches!(error, EventResult::Error(_)));
        assert!(matches!(retry, EventResult::Retry));
    }

    #[tokio::test]
    async fn test_multiple_events() {
        let mut dispatcher = EventDispatcher::new();
        let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = count.clone();

        let handler = move |_event: &AsyncEvent| {
            count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            EventResult::Success("Processed".to_string())
        };

        // Dispatch multiple events
        let sender = dispatcher.sender();
        tokio::spawn(async move {
            let _ = sender.send(AsyncEvent::FetchDatasets).await;
            let _ = sender.send(AsyncEvent::FetchWorkflows).await;
            let _ = sender.send(AsyncEvent::FetchMetrics).await;
            sleep(Duration::from_millis(100)).await;
            let _ = sender.send(AsyncEvent::Shutdown).await;
        });

        // Process events
        dispatcher.process_events(handler).await;

        sleep(Duration::from_millis(50)).await;
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_event_dispatcher_config_default() {
        let config = EventDispatcherConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 100);
        assert_eq!(config.channel_capacity, 100);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let dispatcher = EventDispatcher::new();
        let result = dispatcher.shutdown().await;
        assert!(result.is_ok());
    }
}
