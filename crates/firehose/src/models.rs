//! Firehose in-memory state models.
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct FirehoseState {
    pub resources: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl FirehoseState {
    pub fn new() -> Self {
        Self { resources: Arc::new(RwLock::new(HashMap::new())) }
}
}

impl Default for FirehoseState {
    fn default() -> Self { Self::new() }
}
