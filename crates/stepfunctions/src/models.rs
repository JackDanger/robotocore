//! Step Functions in-memory state models.
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct StepfunctionsState {
    pub state_machines: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub executions: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub tags: Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>,
    pub activities: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl StepfunctionsState {
    pub fn new() -> Self {
        Self {
            state_machines: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            tags: Arc::new(RwLock::new(HashMap::new())),
            activities: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for StepfunctionsState {
    fn default() -> Self { Self::new() }
}
