//! SSM in-memory state models.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// A version of an SSM parameter.
#[derive(Debug)]
pub struct ParameterVersion {
    pub version: u64,
    pub value: String,
    pub timestamp: u64,
    pub label: String,
}

/// An SSM parameter.
#[derive(Debug)]
pub struct Parameter {
    pub name: String,
    pub value: RwLock<String>,
    pub parameter_type: String,
    pub description: RwLock<String>,
    pub allowed_values: RwLock<Vec<String>>,
    pub tags: RwLock<Vec<serde_json::Value>>,
    pub version: RwLock<u64>,
    pub created: u64,
    pub modified: RwLock<u64>,
    pub last_modified_by: String,
    pub history: RwLock<Vec<ParameterVersion>>,
}

impl Parameter {
    pub fn new(name: String, value: String, param_type: String) -> Self {
        Self {
            name,
            value: RwLock::new(value.clone()),
            parameter_type: param_type,
            description: RwLock::new(String::new()),
            allowed_values: RwLock::new(Vec::new()),
            tags: RwLock::new(Vec::new()),
            version: RwLock::new(1),
            created: chrono::Utc::now().timestamp() as u64,
            modified: RwLock::new(chrono::Utc::now().timestamp() as u64),
            last_modified_by: "robotocore".to_string(),
            history: RwLock::new(vec![ParameterVersion {
                version: 1,
                value: value.clone(),
                timestamp: chrono::Utc::now().timestamp() as u64,
                label: String::new(),
            }]),
        }
    }
}

/// An SSM document.
#[derive(Debug)]
pub struct Document {
    pub name: String,
    pub content: String,
    pub document_type: String,
    pub version: u64,
    pub created: u64,
    pub status: String,
}

/// An SSM activation.
#[derive(Debug)]
pub struct Activation {
    pub activation_id: String,
    pub iam_role: String,
    pub created: u64,
    pub active: bool,
}

/// The SSM state store (per account+region).
#[derive(Clone)]
pub struct SsmState {
    pub parameters: Arc<RwLock<HashMap<String, Arc<Parameter>>>>,
    pub documents: Arc<RwLock<HashMap<String, Arc<Document>>>>,
    pub activations: Arc<RwLock<HashMap<String, Arc<Activation>>>>,
}

impl SsmState {
    pub fn new() -> Self {
        Self {
            parameters: Arc::new(RwLock::new(HashMap::new())),
            documents: Arc::new(RwLock::new(HashMap::new())),
            activations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_parameter(&self, name: &str) -> Option<Arc<Parameter>> {
        self.parameters.read().get(name).cloned()
    }

    pub fn put_parameter(&self, param: Arc<Parameter>) {
        self.parameters.write().insert(param.name.clone(), param);
    }

    pub fn delete_parameter(&self, name: &str) -> Option<Arc<Parameter>> {
        self.parameters.write().remove(name)
    }

    pub fn all_parameters(&self) -> Vec<Arc<Parameter>> {
        self.parameters.read().values().cloned().collect()
    }

    pub fn list_parameters(&self, prefix: &str) -> Vec<Arc<Parameter>> {
        self.parameters
            .read()
            .iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .map(|(_, p)| p.clone())
            .collect()
    }
}

impl Default for SsmState {
    fn default() -> Self { Self::new() }
}
