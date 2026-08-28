//! CloudWatch in-memory state models.
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct CloudwatchState {
    pub metrics: Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>,
    pub alarms: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub dashboards: Arc<RwLock<HashMap<String, String>>>,
    pub tags: Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>,
}

impl CloudwatchState {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            alarms: Arc::new(RwLock::new(HashMap::new())),
            dashboards: Arc::new(RwLock::new(HashMap::new())),
            tags: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for CloudwatchState {
    fn default() -> Self { Self::new() }
}
