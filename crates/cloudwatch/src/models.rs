//! Cloudwatch in-memory state models.
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct CloudwatchState {
    pub resources: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl CloudwatchState {
    pub fn new() -> Self {
        Self { resources: Arc::new(RwLock::new(HashMap::new())) }
}
}

impl Default for CloudwatchState {
    fn default() -> Self { Self::new() }
}
