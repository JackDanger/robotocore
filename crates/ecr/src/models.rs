//! Ecr in-memory state models.
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct EcrState {
    pub resources: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl EcrState {
    pub fn new() -> Self {
        Self { resources: Arc::new(RwLock::new(HashMap::new())) }
}
}

impl Default for EcrState {
    fn default() -> Self { Self::new() }
}
