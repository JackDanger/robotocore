//! Secrets Manager in-memory state models.

use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// A secret version.
#[derive(Debug, Clone)]
pub struct SecretVersion {
    pub version_id: String,
    pub secret_string: Option<String>,
    pub secret_binary: Option<String>,
    pub stages: Vec<String>,
    pub created_date: f64,
}

/// A secret.
#[derive(Debug)]
pub struct Secret {
    pub name: String,
    pub arn: String,
    pub owner: u64,
    pub region: String,
    pub created_date: f64,
    pub versions: RwLock<HashMap<String, SecretVersion>>,
    pub tags: RwLock<Vec<Value>>,
    pub description: RwLock<Option<String>>,
    pub deletion_date: RwLock<Option<f64>>,
    pub removed: RwLock<bool>,
}

impl Secret {
    pub fn new(name: String, account: u64, region: String) -> Self {
        let arn = format!("arn:aws:secretsmanager:{}:{}:secret:{}", region, account, name);
        Self {
            name,
            arn,
            owner: account,
            region,
            created_date: chrono::Utc::now().timestamp() as f64,
            versions: RwLock::new(HashMap::new()),
            tags: RwLock::new(Vec::new()),
            description: RwLock::new(None),
            deletion_date: RwLock::new(None),
            removed: RwLock::new(false),
        }
    }

    pub fn get_version(&self, stage: &str) -> Option<SecretVersion> {
        self.versions.read().values().find(|v| v.stages.iter().any(|s| s == stage)).cloned()
    }
}

/// The Secrets Manager state store (per account+region).
#[derive(Clone)]
pub struct SmState {
    pub secrets: Arc<RwLock<HashMap<String, Arc<Secret>>>>,
}

impl SmState {
    pub fn new() -> Self {
        Self {
            secrets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_secret(&self, name: &str) -> Option<Arc<Secret>> {
        self.secrets.read().get(name).cloned()
    }

    pub fn put_secret(&self, secret: Arc<Secret>) {
        self.secrets.write().insert(secret.name.clone(), secret);
    }

    pub fn delete_secret(&self, name: &str) -> Option<Arc<Secret>> {
        self.secrets.write().remove(name)
    }

    pub fn list_secrets(&self) -> Vec<Arc<Secret>> {
        self.secrets.read().values().filter(|s| !*s.removed.read()).cloned().collect()
    }
}

impl Default for SmState {
    fn default() -> Self {
        Self::new()
    }
}
