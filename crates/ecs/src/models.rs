//! ECS in-memory state models.
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct EcsState {
    pub clusters: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub services: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub tasks: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub container_instances: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub task_definitions: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub tags: Arc<RwLock<HashMap<String, Vec<serde_json::Value>>>>,
}

impl EcsState {
    pub fn new() -> Self {
        Self {
            clusters: Arc::new(RwLock::new(HashMap::new())),
            services: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            container_instances: Arc::new(RwLock::new(HashMap::new())),
            task_definitions: Arc::new(RwLock::new(HashMap::new())),
            tags: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for EcsState {
    fn default() -> Self { Self::new() }
}
