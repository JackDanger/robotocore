//! DynamoDB in-memory state models.

use chrono::Utc;
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// An AttributeValue in DynamoDB format.
/// DynamoDB uses a specific JSON format for attribute values:
/// { "S": "string" }, { "N": "number" }, { "B": "base64" },
/// { "BOOL": true }, { "NULL": true }, { "M": {...} }, { "L": [...] },
/// { "SS": [...] }, { "NS": [...] }, { "BS": [...] }
pub type AttrValue = Value;

/// A single DynamoDB item.
#[derive(Debug, Clone)]
pub struct Item {
    pub attributes: HashMap<String, AttrValue>,
}

impl Item {
    pub fn new(attributes: HashMap<String, AttrValue>) -> Self {
        Self { attributes }
    }
}

/// A DynamoDB table definition.
#[derive(Debug)]
pub struct Table {
    pub table_id: String,
    pub name: String,
    pub account: u64,
    pub region: String,
    pub status: RwLock<String>,
    pub created_at: u64,
    pub key_schema: Vec<KeySchema>,
    pub attribute_definitions: Vec<AttributeDefinition>,
    pub billing_mode: String,
    pub items: RwLock<HashMap<String, Arc<Item>>>,
    pub global_secondary_indexes: Vec<IndexDefinition>,
    pub local_secondary_indexes: Vec<IndexDefinition>,
    pub stream_enabled: RwLock<bool>,
    pub stream_view_type: RwLock<Option<String>>,
    pub tags: RwLock<HashMap<String, String>>,
    pub ttl_enabled: RwLock<bool>,
    pub ttl_attribute: RwLock<String>,
}

impl Table {
    pub fn new(
        name: String,
        account: u64,
        region: String,
        key_schema: Vec<KeySchema>,
        attribute_definitions: Vec<AttributeDefinition>,
        billing_mode: String,
    ) -> Self {
        Self {
            table_id: uuid::Uuid::new_v4().simple().to_string(),
            name,
            account,
            region,
            status: RwLock::new("ACTIVE".to_string()),
            created_at: Utc::now().timestamp() as u64,
            key_schema,
            attribute_definitions,
            billing_mode,
            items: RwLock::new(HashMap::new()),
            global_secondary_indexes: Vec::new(),
            local_secondary_indexes: Vec::new(),
            stream_enabled: RwLock::new(false),
            stream_view_type: RwLock::new(None),
            tags: RwLock::new(HashMap::new()),
            ttl_enabled: RwLock::new(false),
            ttl_attribute: RwLock::new(String::new()),
        }
    }

    /// Compute the primary key string for an item.
    pub fn compute_key(&self, item: &Item) -> Option<String> {
        let mut parts = Vec::new();
        for key in &self.key_schema {
            match key.key_type.as_str() {
                "HASH" => {
                    let val = item.attributes.get(&key.attribute_name)?;
                    parts.push(format!("{}={:?}", key.attribute_name, val));
                }
                "RANGE" => {
                    let val = item.attributes.get(&key.attribute_name)?;
                    parts.push(format!("{}={:?}", key.attribute_name, val));
                }
                _ => return None,
            }
        }
        Some(parts.join(","))
    }
}

/// Key schema element.
#[derive(Debug, Clone)]
pub struct KeySchema {
    pub attribute_name: String,
    pub key_type: String, // HASH or RANGE
}

/// Attribute definition.
#[derive(Debug, Clone)]
pub struct AttributeDefinition {
    pub attribute_name: String,
    pub attribute_type: String, // S, N, or B
}

/// Index definition.
#[derive(Debug, Clone)]
pub struct IndexDefinition {
    pub name: String,
    pub key_schema: Vec<KeySchema>,
    pub projection: String,
}

/// The DynamoDB state store (per account+region).
#[derive(Clone)]
pub struct DynamoDbState {
    pub tables: Arc<RwLock<HashMap<String, Arc<Table>>>>,
}

impl DynamoDbState {
    pub fn new() -> Self {
        Self {
            tables: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_table(&self, name: &str) -> Option<Arc<Table>> {
        self.tables.read().get(name).cloned()
    }

    pub fn put_table(&self, table: Arc<Table>) {
        self.tables.write().insert(table.name.clone(), table);
    }

    pub fn delete_table(&self, name: &str) -> Option<Arc<Table>> {
        self.tables.write().remove(name)
    }

    pub fn list_tables(&self) -> Vec<String> {
        self.tables.read().keys().cloned().collect()
    }
}

impl Default for DynamoDbState {
    fn default() -> Self {
        Self::new()
    }
}
