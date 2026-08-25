//! In-memory state store for AWS resources.
//!
//! Provides a generic registry of per-account-region state stores.
//! Each store holds named tables of resources (S3 buckets, SQS queues, etc.).

use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::core::account::AccountRegion;

/// In-memory table of resources, keyed by resource name.
pub type ResourceTable = Arc<RwLock<HashMap<String, Value>>>;

/// Global registry of state stores, scoped by (account, region).
pub struct StateStore {
    stores: RwLock<HashMap<AccountRegion, HashMap<String, ResourceTable>>>,
}

impl StateStore {
    /// Create a new empty StateStore.
    pub fn new() -> Self {
        Self {
            stores: RwLock::new(HashMap::new()),
        }
    }

    /// Get or create a table for a resource type in the given account/region.
    pub fn table(&self, ar: &AccountRegion, table_name: &str) -> ResourceTable {
        let mut stores = self.stores.write();
        let tables = stores.entry(ar.clone()).or_default();

        tables
            .entry(table_name.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(HashMap::new())))
            .clone()
    }

    /// Get a value from a table.
    pub fn get(&self, ar: &AccountRegion, table: &str, key: &str) -> Option<Value> {
        let tbl = self.table(ar, table);
        let guard = tbl.read();
        guard.get(key).cloned()
    }

    /// Put a value into a table.
    pub fn put(&self, ar: &AccountRegion, table: &str, key: String, value: Value) {
        let tbl = self.table(ar, table);
        tbl.write().insert(key, value);
    }

    /// Delete a value from a table.
    pub fn delete(&self, ar: &AccountRegion, table: &str, key: &str) -> Option<Value> {
        let tbl = self.table(ar, table);
        let mut guard = tbl.write();
        guard.remove(key)
    }

    /// Scan all keys in a table.
    pub fn scan(&self, ar: &AccountRegion, table: &str) -> Vec<(String, Value)> {
        let tbl = self.table(ar, table);
        let guard = tbl.read();
        guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// List all table names for a given account/region.
    pub fn list_tables(&self, ar: &AccountRegion) -> Vec<String> {
        let stores = self.stores.read();
        match stores.get(ar) {
            Some(tables) => tables.keys().cloned().collect(),
            None => Vec::new(),
        }
    }
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_and_get() {
        let store = StateStore::new();
        let ar = AccountRegion::new(123456789012, "us-east-1".to_string());

        let value = Value::String("test".to_string());
        store.put(&ar, "s3", "bucket1".to_string(), value.clone());

        let retrieved = store.get(&ar, "s3", "bucket1");
        assert_eq!(retrieved, Some(value));
    }

    #[test]
    fn test_get_nonexistent() {
        let store = StateStore::new();
        let ar = AccountRegion::new(123456789012, "us-east-1".to_string());

        let retrieved = store.get(&ar, "s3", "nonexistent");
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_delete() {
        let store = StateStore::new();
        let ar = AccountRegion::new(123456789012, "us-east-1".to_string());

        let value = Value::String("test".to_string());
        store.put(&ar, "s3", "bucket1".to_string(), value);

        let deleted = store.delete(&ar, "s3", "bucket1");
        assert!(deleted.is_some());

        let retrieved = store.get(&ar, "s3", "bucket1");
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_account_isolation() {
        let store = StateStore::new();
        let ar1 = AccountRegion::new(111111111111, "us-east-1".to_string());
        let ar2 = AccountRegion::new(222222222222, "us-east-1".to_string());

        let value1 = Value::String("account1".to_string());
        let value2 = Value::String("account2".to_string());

        store.put(&ar1, "s3", "bucket".to_string(), value1.clone());
        store.put(&ar2, "s3", "bucket".to_string(), value2.clone());

        assert_eq!(store.get(&ar1, "s3", "bucket"), Some(value1));
        assert_eq!(store.get(&ar2, "s3", "bucket"), Some(value2));
    }

    #[test]
    fn test_scan() {
        let store = StateStore::new();
        let ar = AccountRegion::new(123456789012, "us-east-1".to_string());

        store.put(
            &ar,
            "s3",
            "bucket1".to_string(),
            Value::String("b1".to_string()),
        );
        store.put(
            &ar,
            "s3",
            "bucket2".to_string(),
            Value::String("b2".to_string()),
        );

        let items = store.scan(&ar, "s3");
        assert_eq!(items.len(), 2);
    }
}
