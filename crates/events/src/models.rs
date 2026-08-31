//! EventBridge in-memory state models.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// An EventBridge rule.
#[derive(Debug)]
pub struct Rule {
    pub name: String,
    pub arn: String,
    pub description: String,
    pub event_pattern: String,
    pub state: parking_lot::RwLock<String>,
    pub role_arn: Option<String>,
    pub schedule_expression: Option<String>,
    pub created: u64,
    pub targets: RwLock<Vec<serde_json::Value>>,
    pub tags: RwLock<Vec<serde_json::Value>>,
    pub inputs: RwLock<Vec<serde_json::Value>>,
}

/// The EventBridge state store.
#[derive(Clone)]
pub struct EventsState {
    pub rules: Arc<RwLock<HashMap<String, Arc<Rule>>>>,
    pub event_buses: Arc<RwLock<HashMap<String, String>>>,
}

impl EventsState {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            event_buses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_rule(&self, name: &str) -> Option<Arc<Rule>> {
        self.rules.read().get(name).cloned()
    }
}

impl Default for EventsState {
    fn default() -> Self { Self::new() }
}
