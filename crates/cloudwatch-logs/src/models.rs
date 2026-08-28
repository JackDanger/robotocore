//! CloudWatch Logs in-memory state models.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// A log group.
#[derive(Debug)]
pub struct LogGroup {
    pub name: String,
    pub arn: String,
    pub created: u64,
    pub log_streams: RwLock<Vec<LogStream>>,
    pub retention_in_days: RwLock<Option<i64>>,
    pub tags: RwLock<HashMap<String, String>>,
}

/// A log stream.
#[derive(Debug)]
pub struct LogStream {
    pub name: String,
    pub arn: String,
    pub created: u64,
    pub first_event_time: i64,
    pub last_event_time: i64,
    pub last_ingested_time: i64,
    pub store_name: Option<String>,
    pub events: RwLock<Vec<LogEvent>>,
}

/// A single log event.
#[derive(Debug, Clone)]
pub struct LogEvent {
    pub timestamp: i64,
    pub message: String,
    pub id: String,
}

impl LogGroup {
    pub fn new(account: u64, region: &str, name: String) -> Self {
        let arn = format!("arn:aws:logs:{}:{}:log-group:{}", region, account, name);
        Self {
            name,
            arn,
            created: chrono::Utc::now().timestamp_millis() as u64,
            log_streams: RwLock::new(Vec::new()),
            retention_in_days: RwLock::new(None),
            tags: RwLock::new(HashMap::new()),
        }
    }
}

/// The CloudWatch Logs state store.
#[derive(Clone)]
pub struct LogsState {
    pub log_groups: Arc<RwLock<HashMap<String, Arc<LogGroup>>>>,
    pub metric_filters: Arc<RwLock<Vec<serde_json::Value>>>,
    pub subscriptions: Arc<RwLock<Vec<serde_json::Value>>>,
    pub insights: Arc<RwLock<Vec<serde_json::Value>>>,
}

impl LogsState {
    pub fn new() -> Self {
        Self {
            log_groups: Arc::new(RwLock::new(HashMap::new())),
            metric_filters: Arc::new(RwLock::new(Vec::new())),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            insights: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn get_log_group(&self, name: &str) -> Option<Arc<LogGroup>> {
        self.log_groups.read().get(name).cloned()
    }
}

impl Default for LogsState {
    fn default() -> Self { Self::new() }
}
