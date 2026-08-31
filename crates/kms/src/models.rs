//! KMS in-memory state models.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// A KMS key.
#[derive(Debug)]
pub struct Key {
    pub key_id: String,
    pub arn: String,
    pub owner: u64,
    pub region: String,
    pub key_state: RwLock<String>,
    pub key_usage: String,
    pub key_spec: String,
    pub description: RwLock<String>,
    pub enabled: RwLock<bool>,
    pub key_material: Vec<u8>,
    pub created: u64,
    pub tags: RwLock<Vec<serde_json::Value>>,
    pub rotation_enabled: RwLock<bool>,
}

impl Key {
    pub fn new(account: u64, region: String, key_usage: &str, key_spec: &str) -> Self {
        let key_id = uuid::Uuid::new_v4().simple().to_string();
        Self {
            key_id: key_id.clone(),
            arn: format!("arn:aws:kms:{}:{}:key/{}", region, account, key_id),
            owner: account,
            region,
            key_state: RwLock::new("Enabled".to_string()),
            key_usage: key_usage.to_string(),
            key_spec: key_spec.to_string(),
            description: RwLock::new(String::new()),
            enabled: RwLock::new(true),
            key_material: vec![0u8; 32],
            created: chrono::Utc::now().timestamp() as u64,
            tags: RwLock::new(Vec::new()),
            rotation_enabled: RwLock::new(false),
        }
    }
}

/// The KMS state store (per account+region).
#[derive(Clone)]
pub struct KmsState {
    pub keys: Arc<RwLock<HashMap<String, Arc<Key>>>>,
    pub aliases: Arc<RwLock<HashMap<String, String>>>,
    pub tags: Arc<RwLock<HashMap<String, serde_json::Map<String, serde_json::Value>>>>,
}

impl KmsState {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            aliases: Arc::new(RwLock::new(HashMap::new())),
            tags: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_alias(&self, alias_name: &str, key_id: &str) {
        self.aliases.write().insert(alias_name.to_string(), key_id.to_string());
    }

    pub fn delete_alias(&self, alias_name: &str) {
        self.aliases.write().remove(alias_name);
    }

    pub fn list_aliases(&self) -> Vec<(String, String)> {
        self.aliases.read().iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    pub fn get_key(&self, id: &str) -> Option<Arc<Key>> {
        // Try direct lookup, then by ARN, then by alias
        let keys = self.keys.read();
        if let Some(k) = keys.get(id) { return Some(k.clone()); }
        if let Some(k) = keys.values().find(|k| k.arn == id) { return Some(k.clone()); }
        // Try alias resolution
        let aliases = self.aliases.read();
        if let Some(target) = aliases.get(id) {
            if let Some(k) = keys.get(target) { return Some(k.clone()); }
            return keys.values().find(|k| k.arn == *target).cloned();
        }
        None
    }

    pub fn put_key(&self, key: Arc<Key>) {
        self.keys.write().insert(key.key_id.clone(), key);
    }

    pub fn list_keys(&self) -> Vec<Arc<Key>> {
        self.keys.read().values().cloned().collect()
    }

    pub fn delete_key(&self, id: &str) -> Option<Arc<Key>> {
        let key = self.get_key(id)?;
        self.keys.write().remove(&key.key_id)
    }
}

impl Default for KmsState {
    fn default() -> Self { Self::new() }
}
