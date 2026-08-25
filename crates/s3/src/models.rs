//! S3 in-memory state models.

use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// A single S3 object.
#[derive(Debug)]
pub struct Object {
    pub key: String,
    pub data: Vec<u8>,
    pub content_type: String,
    pub metadata: HashMap<String, String>,
    pub etag: String,
    pub size: usize,
    pub last_modified: u64,
    pub storage_class: String,
    pub version_id: String,
    pub tags: RwLock<HashMap<String, String>>,
}

impl Object {
    pub fn new(key: String, data: Vec<u8>, content_type: String) -> Self {
        let size = data.len();
        let etag = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&data);
            hex::encode(hasher.finalize())
        };
        Self {
            key,
            data,
            content_type,
            metadata: HashMap::new(),
            etag,
            size,
            last_modified: Utc::now().timestamp() as u64,
            storage_class: "STANDARD".to_string(),
            version_id: uuid::Uuid::new_v4().to_string(),
            tags: RwLock::new(HashMap::new()),
        }
    }
}

/// A single S3 bucket.
#[derive(Debug)]
pub struct Bucket {
    pub name: String,
    pub region: String,
    pub created: u64,
    pub versioning: RwLock<bool>,
    pub policy: RwLock<Option<String>>,
    pub cors_rules: RwLock<Vec<serde_json::Value>>,
    pub lifecycle_rules: RwLock<Vec<serde_json::Value>>,
    pub tags: RwLock<HashMap<String, String>>,
    pub website: RwLock<Option<serde_json::Value>>,
    pub objects: RwLock<HashMap<String, Object>>,
    pub multipart_uploads: RwLock<HashMap<String, MultipartUpload>>,
    pub acl: String,
    pub object_lock_enabled: RwLock<bool>,
    pub object_lock_mode: RwLock<Option<String>>,
}

impl Bucket {
    pub fn new(name: String, region: String) -> Self {
        Self {
            name,
            region,
            created: Utc::now().timestamp() as u64,
            versioning: RwLock::new(false),
            policy: RwLock::new(None),
            cors_rules: RwLock::new(Vec::new()),
            lifecycle_rules: RwLock::new(Vec::new()),
            tags: RwLock::new(HashMap::new()),
            website: RwLock::new(None),
            objects: RwLock::new(HashMap::new()),
            multipart_uploads: RwLock::new(HashMap::new()),
            acl: "private".to_string(),
            object_lock_enabled: RwLock::new(false),
            object_lock_mode: RwLock::new(None),
        }
    }
}

/// A multipart upload in progress.
#[derive(Debug)]
pub struct MultipartUpload {
    pub upload_id: String,
    pub key: String,
    pub initiated: u64,
    pub parts: RwLock<HashMap<u32, MultipartPart>>,
    pub content_type: String,
    pub metadata: HashMap<String, String>,
}

impl MultipartUpload {
    pub fn new(key: String, content_type: String) -> Self {
        Self {
            upload_id: uuid::Uuid::new_v4().to_string(),
            key,
            initiated: Utc::now().timestamp() as u64,
            parts: RwLock::new(HashMap::new()),
            content_type,
            metadata: HashMap::new(),
        }
    }
}

/// A single part of a multipart upload.
#[derive(Debug, Clone)]
pub struct MultipartPart {
    pub part_number: u32,
    pub etag: String,
    pub size: usize,
    pub data: Vec<u8>,
    pub last_modified: u64,
}

/// The S3 state store (per account+region).
#[derive(Clone)]
pub struct S3State {
    pub buckets: Arc<RwLock<HashMap<String, Arc<Bucket>>>>,
}

impl S3State {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_bucket(&self, name: &str) -> Option<Arc<Bucket>> {
        self.buckets.read().get(name).cloned()
    }

    pub fn put_bucket(&self, bucket: Arc<Bucket>) {
        self.buckets.write().insert(bucket.name.clone(), bucket);
    }

    pub fn delete_bucket(&self, name: &str) -> Option<Arc<Bucket>> {
        self.buckets.write().remove(name)
    }

    pub fn list_buckets(&self) -> Vec<(String, u64)> {
        self.buckets
            .read()
            .iter()
            .map(|(name, b)| (name.clone(), b.created))
            .collect()
    }
}

impl Default for S3State {
    fn default() -> Self {
        Self::new()
    }
}
