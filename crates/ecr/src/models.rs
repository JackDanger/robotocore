//! ECR in-memory state models.
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct EcrState {
    pub repositories: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub images: Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>,
    pub policies: Arc<RwLock<HashMap<String, String>>>,
    pub scanning_configs: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub tags: Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>,
}

impl EcrState {
    pub fn new() -> Self {
        Self {
            repositories: Arc::new(RwLock::new(HashMap::new())),
            images: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            scanning_configs: Arc::new(RwLock::new(HashMap::new())),
            tags: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for EcrState {
    fn default() -> Self { Self::new() }
}
