//! Lambda in-memory state models.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// A Lambda function.
#[derive(Debug)]
pub struct LambdaFunction {
    pub function_name: String,
    pub function_arn: String,
    pub runtime: RwLock<String>,
    pub handler: RwLock<String>,
    pub role: RwLock<String>,
    pub code_size: u64,
    pub description: RwLock<String>,
    pub timeout: RwLock<u32>,
    pub memory_size: RwLock<u32>,
    pub last_modified: RwLock<u64>,
    pub state: RwLock<String>,
    pub reason: RwLock<String>,
    pub version: String,
    pub tags: RwLock<HashMap<String, String>>,
    pub code_sha256: String,
    pub environment: RwLock<Option<serde_json::Value>>,
    pub layers: RwLock<Vec<String>>,
    pub vpc_config: RwLock<Option<serde_json::Value>>,
}

impl LambdaFunction {
    pub fn new(account: u64, region: &str, function_name: String) -> Self {
        let arn = format!("arn:aws:lambda:{}:{}:function:{}", region, account, function_name);
        Self {
            function_name,
            function_arn: arn,
            runtime: RwLock::new("python3.12".to_string()),
            handler: RwLock::new("index.handler".to_string()),
            role: RwLock::new(format!("arn:aws:iam::{}:role/lambda-role", account)),
            code_size: 0,
            description: RwLock::new(String::new()),
            timeout: RwLock::new(3),
            memory_size: RwLock::new(128),
            last_modified: RwLock::new(chrono::Utc::now().timestamp_millis() as u64),
            state: RwLock::new("Active".to_string()),
            reason: RwLock::new(String::new()),
            version: "1".to_string(),
            tags: RwLock::new(HashMap::new()),
            code_sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            environment: RwLock::new(None),
            layers: RwLock::new(Vec::new()),
            vpc_config: RwLock::new(None),
        }
    }
}

/// A Lambda alias.
#[derive(Debug)]
pub struct LambdaAlias {
    pub name: String,
    pub function_arn: String,
    pub function_version: RwLock<String>,
    pub description: RwLock<String>,
    pub created: u64,
    pub modified: RwLock<u64>,
    pub routing_config: serde_json::Value,
}

/// An event source mapping.
#[derive(Debug)]
pub struct EventSourceMapping {
    pub id: String,
    pub function_arn: String,
    pub event_source_arn: String,
    pub state: String,
    pub last_modified: u64,
    pub batch_size: u32,
    pub enabled: bool,
}

/// A Lambda layer.
#[derive(Debug)]
pub struct LambdaLayer {
    pub layer_arn: String,
    pub layer_name: String,
    pub version: u32,
    pub description: String,
    pub created: u64,
    pub content_size: u64,
    pub compatible_runtimes: Vec<String>,
}

/// The Lambda state store (per account+region).
#[derive(Clone)]
pub struct LambdaState {
    pub functions: Arc<RwLock<HashMap<String, Arc<LambdaFunction>>>>,
    pub aliases: Arc<RwLock<HashMap<(String, String), Arc<LambdaAlias>>>>,
    pub event_source_mappings: Arc<RwLock<HashMap<String, Arc<EventSourceMapping>>>>,
    pub layers: Arc<RwLock<HashMap<String, Vec<Arc<LambdaLayer>>>>>,
    pub permissions: Arc<RwLock<Vec<serde_json::Value>>>,
}

impl LambdaState {
    pub fn new() -> Self {
        Self {
            functions: Arc::new(RwLock::new(HashMap::new())),
            aliases: Arc::new(RwLock::new(HashMap::new())),
            event_source_mappings: Arc::new(RwLock::new(HashMap::new())),
            layers: Arc::new(RwLock::new(HashMap::new())),
            permissions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn get_function(&self, name: &str) -> Option<Arc<LambdaFunction>> {
        let funcs = self.functions.read();
        if let Some(f) = funcs.get(name) { return Some(f.clone()); }
        funcs.values().find(|f| f.function_arn == name).cloned()
    }
}

impl Default for LambdaState {
    fn default() -> Self { Self::new() }
}
