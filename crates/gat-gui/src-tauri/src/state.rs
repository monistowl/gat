// crates/gat-gui/src-tauri/src/state.rs
use gat_batch::BatchJobRecord;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct BatchRun {
    pub run_id: String,
    pub status: String, // "running" | "completed" | "failed"
    pub completed: usize,
    pub total: usize,
    pub results: Option<Vec<BatchJobRecord>>,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct AppState {
    pub batch_runs: Arc<Mutex<HashMap<String, BatchRun>>>,
}
