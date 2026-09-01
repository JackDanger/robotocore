//! Kinesis in-memory state models.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// A Kinesis stream shard.
#[derive(Debug, Clone)]
pub struct Shard {
    pub shard_id: String,
    pub sequence_number: String,
    pub parent_id: Option<String>,
    pub hash_range: String,
    pub created: u64,
}

/// A Kinesis stream.
#[derive(Debug)]
pub struct KinesisStream {
    pub stream_name: String,
    pub stream_arn: String,
    pub stream_status: RwLock<String>,
    pub shard_count: u32,
    pub retention_period_hours: parking_lot::RwLock<u32>,
    pub shard_level_metrics: RwLock<Vec<String>>,
    pub stream_mode: String,
    pub created: u64,
    pub shards: RwLock<Vec<Shard>>,
    pub tags: RwLock<Vec<serde_json::Value>>,
}

impl KinesisStream {
    pub fn new(account: u64, region: &str, stream_name: String, shard_count: u32) -> Self {
        let arn = format!("arn:aws:kinesis:{}:{}:stream/{}", region, account, stream_name);
        let shards: Vec<Shard> = (0..shard_count)
            .map(|i| Shard {
                shard_id: format!("shardId-000000000000.{}.000000000000", i),
                sequence_number: format!("4959033827149996696551807410074587034031434670931271809"),
                parent_id: None,
                hash_range: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                created: chrono::Utc::now().timestamp_millis() as u64,
            })
            .collect();
        Self {
            stream_name,
            stream_arn: arn,
            stream_status: RwLock::new("ACTIVE".to_string()),
            shard_count,
            retention_period_hours: parking_lot::RwLock::new(24),
            shard_level_metrics: RwLock::new(Vec::new()),
            stream_mode: "ON_DEMAND".to_string(),
            created: chrono::Utc::now().timestamp_millis() as u64,
            shards: RwLock::new(shards),
            tags: RwLock::new(vec![]),
        }
    }
}

/// The Kinesis state store.
#[derive(Clone)]
pub struct KinesisState {
    pub streams: Arc<RwLock<HashMap<String, Arc<KinesisStream>>>>,
}

impl KinesisState {
    pub fn new() -> Self {
        Self { streams: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub fn get_stream(&self, name: &str) -> Option<Arc<KinesisStream>> {
        self.streams.read().get(name).cloned()
    }
}

impl Default for KinesisState {
    fn default() -> Self { Self::new() }
}

impl Default for Shard {
    fn default() -> Self {
        Self {
            shard_id: "shardId-default".to_string(),
            sequence_number: "0".to_string(),
            parent_id: None,
            hash_range: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            created: 0,
        }
    }
}
