//! KMS operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{Key, KmsState};
use crate::protocol::{AwsRequest, AwsResponse};

pub struct KmsHandler {
    state: RwLock<HashMap<(u64, String), KmsState>>,
}

impl KmsHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> KmsState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(KmsState::new).clone()
    }

    fn json_stub(&self, _req: &AwsRequest, field: &str) -> AwsResponse {
        AwsResponse::json(200, json!({ field: "" }))
    }

    fn json_stub_list(&self, _req: &AwsRequest, field: &str) -> AwsResponse {
        AwsResponse::json(200, json!({ field: [] }))
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let op = req.operation.as_str();
        match op {
            "CreateKey" => self.create_key(&req),
            "DeleteKey" => self.delete_key(&req),
            "DescribeKey" => self.describe_key(&req),
            "ListKeys" => self.list_keys(&req),
            "ListAliases" => self.list_aliases(&req),
            "EnableKey" => self.enable_key(&req),
            "DisableKey" => self.disable_key(&req),
            "CreateAlias" => self.create_alias(&req),
            "DeleteAlias" => self.delete_alias(&req),
            "TagResource" => self.tag_resource(&req),
            "UntagResource" => self.untag_resource(&req),
            "ListResourceTags" => self.list_resource_tags(&req),
            "Encrypt" => self.encrypt(&req),
            "Decrypt" => self.decrypt(&req),
            "GenerateDataKey" => self.generate_data_key(&req),
            "GenerateRandom" => self.generate_random(&req),
            "PutKeyPolicy" => self.put_key_policy(&req),
            "GetKeyPolicy" => self.get_key_policy(&req),
            "ScheduleKeyDeletion" => self.schedule_key_deletion(&req),
            "CancelKeyDeletion" => self.cancel_key_deletion(&req),
            "UpdateKeyDescription" => self.update_key_description(&req),
            "GetPublicKey" => self.get_public_key(&req),
            "ReEncrypt" => self.re_encrypt(&req),
            "GetKeyRotationStatus" => self.get_key_rotation_status(&req),
            "EnableKeyRotation" => self.enable_key_rotation(&req),
            "DisableKeyRotation" => self.disable_key_rotation(&req),
            "ListResourceTags" => self.list_resource_tags(&req),
            "TagResource" => self.tag_resource(&req),
            "UntagResource" => self.untag_resource(&req),
            "PutKeyPolicy" => self.put_key_policy(&req),
            "GetKeyPolicy" => self.get_key_policy(&req),
            "DeleteAlias" => self.delete_alias(&req),
            "ListAliases" => self.list_aliases(&req),
            "CreateAlias" => self.create_alias(&req),
            "CreateGrant" => AwsResponse::json(200, json!({
                "GrantId": uuid::Uuid::new_v4().simple().to_string(),
                "GrantToken": uuid::Uuid::new_v4().simple().to_string()
            })),
            "ListGrants" => AwsResponse::json(200, json!({ "Grants": [], "NextMarker": null })),
            "RetireGrant" => AwsResponse::json(200, json!({})),
            "RevokeGrant" => AwsResponse::json(200, json!({})),
            "DescribeKey" => self.describe_key(&req),
                        "CreateCustomKeyStore" => self.json_stub(&req, "CustomKeyStore"),
            "DeriveSharedSecret" => self.json_stub(&req, "DeriveSharedSecret"),
            "DescribeCustomKeyStores" => self.json_stub(&req, "CustomKeyStores"),
            "GenerateDataKeyPair" => self.json_stub(&req, "GenerateDataKeyPair"),
            "GenerateDataKeyPairWithoutPlaintext" => self.json_stub(&req, "GenerateDataKeyPairWithoutPlaintext"),
            "GenerateDataKeyWithoutPlaintext" => self.json_stub(&req, "GenerateDataKeyWithoutPlaintext"),
            "GenerateMac" => self.json_stub(&req, "GenerateMac"),
            "GetParametersForImport" => self.json_stub(&req, "ParametersForImport"),
            "ListKeyPolicies" => self.json_stub_list(&req, "KeyPolicies"),
            "ListKeyRotations" => self.json_stub_list(&req, "KeyRotations"),
            "ListRetirableGrants" => self.json_stub_list(&req, "RetirableGrants"),
            "ReplicateKey" => self.json_stub(&req, "ReplicateKey"),
            "RotateKeyOnDemand" => self.json_stub(&req, "RotateKeyOnDemand"),
            "Sign" => self.json_stub(&req, "Sign"),
            "UpdateAlias" => self.json_stub(&req, "Alias"),
            "UpdatePrimaryRegion" => self.json_stub(&req, "PrimaryRegion"),
other => AwsResponse::error(400, "InvalidException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn create_key(&self, req: &AwsRequest) -> AwsResponse {
        let key_usage = req.params.get("KeyUsage").and_then(|v| v.as_str()).unwrap_or("ENCRYPT_DECRYPT");
        let key_spec = req.params.get("KeySpec").and_then(|v| v.as_str()).unwrap_or("SYMMETRIC_DEFAULT");
        let key = Arc::new(Key::new(req.account, req.region.clone(), key_usage, key_spec));
        if let Some(desc) = req.params.get("Description").and_then(|v| v.as_str()) {
            *key.description.write() = desc.to_string();
        }
        let state = self.get_state(req.account, &req.region);
        state.put_key(key.clone());
        AwsResponse::json(200, json!({
            "KeyMetadata": {
                "KeyId": key.key_id,
                "Arn": key.arn,
                "CreationDate": key.created as f64,
                "Enabled": true,
                "KeyUsage": key.key_usage,
                "KeyState": "Enabled",
                "KeyManager": "CUSTOMER_MANAGED",
                "CustomerMasterKeyId": key.key_id,
                "EncryptionAlgorithms": ["SYMMETRIC_DEFAULT"],
                "KeySpec": key.key_spec,
                "Description": *key.description.read(),
                "Origin": "AWS_KMS",
                "KeyManagerType": "CUSTOMER"
            }
        }))
    }

    fn delete_key(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        match state.delete_key(key_id) {
            Some(key) => AwsResponse::json(200, json!({
                "KeyMetadata": {
                    "KeyId": key.key_id,
                    "Arn": key.arn,
                    "KeyState": "PendingDeletion",
                    "DeletionDate": (key.created + 30*24*3600*1000) as f64
                }
            })),
            None => AwsResponse::error(400, "NotFoundException",
                "The operation references an alias that does not exist"),
        }
    }

    fn describe_key(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        match state.get_key(key_id) {
            Some(key) => AwsResponse::json(200, json!({
                "KeyMetadata": {
                    "KeyId": key.key_id,
                    "Arn": key.arn,
                    "CreationDate": key.created as f64,
                    "Enabled": *key.enabled.read(),
                    "KeyUsage": key.key_usage,
                    "KeyState": *key.key_state.read(),
                    "KeyManager": "CUSTOMER_MANAGED",
                    "CustomerMasterKeyId": key.key_id,
                    "EncryptionAlgorithms": ["SYMMETRIC_DEFAULT"],
                    "KeySpec": key.key_spec,
                    "Description": *key.description.read(),
                    "Origin": "AWS_KMS",
                    "KeyManagerType": "CUSTOMER",
                    "KeyRotationEnabled": false,
                    "RotationPeriodInDays": 365,
                    "BypassKeyReplicationCheck": false
                }
            })),
            None => AwsResponse::error(400, "NotFoundException",
                "The operation references an alias that does not exist"),
        }
    }

    fn list_keys(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let keys = state.list_keys();
        let key_list: Vec<Value> = keys.iter().map(|k| json!({
            "KeyId": k.key_id,
            "Arn": k.arn
        })).collect();
        AwsResponse::json(200, json!({ "Keys": key_list }))
    }

    fn list_aliases(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let aliases = state.list_aliases();
        let alias_list: Vec<Value> = aliases.iter().map(|(name, key_id)| {
            let key = state.get_key(key_id);
            json!({
                "AliasName": name,
                "AliasArn": format!("arn:aws:kms:us-east-1:123456789012:alias/{}", name.trim_start_matches("alias/")),
                "ResourceArn": key.map(|k| k.arn.clone()).unwrap_or_default(),
                "CreationDate": 0.0,
                "TargetKeyId": key_id
            })
        }).collect();
        AwsResponse::json(200, json!({ "Aliases": alias_list, "NextMarker": null }))
    }

    fn enable_key(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        match state.get_key(key_id) {
            Some(key) => {
                *key.enabled.write() = true;
                *key.key_state.write() = "Enabled".to_string();
                AwsResponse::json(200, json!({}))
            }
            None => AwsResponse::error(400, "NotFoundException", "Key not found"),
        }
    }

    fn disable_key(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        match state.get_key(key_id) {
            Some(key) => {
                *key.enabled.write() = false;
                *key.key_state.write() = "Disabled".to_string();
                AwsResponse::json(200, json!({}))
            }
            None => AwsResponse::error(400, "NotFoundException", "Key not found"),
        }
    }

    fn create_alias(&self, req: &AwsRequest) -> AwsResponse {
        let alias_name = req.params.get("AliasName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let key_id = req.params.get("TargetKeyId").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let state = self.get_state(req.account, &req.region);
        if state.get_key(&key_id).is_none() {
            return AwsResponse::error(400, "NotFoundException", "Key not found");
        }
        state.create_alias(&alias_name, &key_id);
        AwsResponse::json(200, json!({}))
    }

    fn delete_alias(&self, req: &AwsRequest) -> AwsResponse {
        let alias_name = req.params.get("AliasName").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        state.delete_alias(alias_name);
        AwsResponse::json(200, json!({}))
    }

    fn tag_resource(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let tags: Vec<Value> = req.params.get("Tags")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut all_tags = state.tags.write();
        let entry = all_tags.entry(key_id.to_string()).or_insert_with(|| serde_json::Map::new());
        for tag in &tags {
            let key = tag.get("TagKey").and_then(|v| v.as_str()).unwrap_or("");
            let value = tag.get("TagValue").and_then(|v| v.as_str()).unwrap_or("");
            entry.insert(key.to_string(), json!(value));
        }
        AwsResponse::json(200, json!({}))
    }

    fn untag_resource(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let tag_keys: Vec<String> = req.params.get("TagKeys")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut all_tags = state.tags.write();
        if let Some(entry) = all_tags.get_mut(key_id) {
            for key in &tag_keys {
                entry.remove(key);
            }
        }
        AwsResponse::json(200, json!({}))
    }

    fn list_resource_tags(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        let tags = state.tags.read().get(key_id)
            .cloned()
            .unwrap_or_else(|| serde_json::Map::new());
        let tags_list: Vec<Value> = tags.iter()
            .map(|(k, v)| json!({ "TagKey": k, "TagValue": v.as_str().unwrap_or("") }))
            .collect();
        AwsResponse::json(200, json!({ "Tags": tags_list }))
    }

    fn get_key_rotation_status(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        if state.get_key(key_id).is_some() {
            AwsResponse::json(200, json!({
                "RotationEnabled": false,
                "RotationPeriodInDays": 30
            }))
        } else {
            AwsResponse::error(400, "NotFoundException", "Key not found")
        }
    }

    fn enable_key_rotation(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn disable_key_rotation(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }


    fn encrypt(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        if state.get_key(key_id).is_none() {
            return AwsResponse::error(400, "NotFoundException", "Key not found");
        }
        let plaintext = req.params.get("Plaintext").and_then(|v| v.as_str()).unwrap_or("");
        let ciphertext = base64::encode(format!("encrypted:{}", plaintext));
        AwsResponse::json(200, json!({
            "CiphertextBlob": ciphertext,
            "KeyId": key_id,
            "KeyId2": ""
        }))
    }

    fn decrypt(&self, req: &AwsRequest) -> AwsResponse {
        let ciphertext_b64 = req.params.get("CiphertextBlob").and_then(|v| v.as_str()).unwrap_or("");
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        // Decode the ciphertext (base64 -> "encrypted:{original_b64}")
        let ciphertext = base64::decode(ciphertext_b64).unwrap_or_default();
        let ciphertext_str = String::from_utf8_lossy(&ciphertext);
        let plaintext_b64 = ciphertext_str.strip_prefix("encrypted:").unwrap_or(&ciphertext_str);
        AwsResponse::json(200, json!({
            "Plaintext": plaintext_b64,
            "KeyId": key_id
        }))
    }

    fn generate_data_key(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        if state.get_key(key_id).is_none() {
            return AwsResponse::error(400, "NotFoundException", "Key not found");
        }
        let key_b64 = base64::encode(vec![0u8; 32]);
        let enc_b64 = base64::encode(format!("encrypted:{}", uuid::Uuid::new_v4().simple()).into_bytes());
        AwsResponse::json(200, json!({
            "Plaintext": key_b64,
            "CiphertextBlob": enc_b64,
            "KeyId": key_id
        }))
    }

    fn generate_random(&self, req: &AwsRequest) -> AwsResponse {
        let len = req.params.get("NumberOfBytes").and_then(|v| v.as_u64()).unwrap_or(16) as usize;
        let mut data = vec![0u8; len];
        for b in data.iter_mut() {
            *b = (chrono::Utc::now().timestamp_subsec_nanos() % 256) as u8;
        }
        let b64 = base64::encode(data);
        AwsResponse::json(200, json!({ "Plaintext": b64 }))
    }

    fn put_key_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn get_key_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({ "Policy": "{}" }))
    }

    fn schedule_key_deletion(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        match state.get_key(key_id) {
            Some(key) => AwsResponse::json(200, json!({
                "KeyId": key.key_id,
                "KeyState": "PendingDeletion",
                "DeletionDate": (key.created + 7*24*3600*1000) as f64
            })),
            None => AwsResponse::error(400, "NotFoundException", "Key not found"),
        }
    }

    fn cancel_key_deletion(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        match state.get_key(key_id) {
            Some(key) => {
                *key.key_state.write() = "Enabled".to_string();
                AwsResponse::json(200, json!({ "KeyId": key.key_id, "KeyState": "Enabled" }))
            }
            None => AwsResponse::error(400, "NotFoundException", "Key not found"),
        }
    }

    fn update_key_description(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let desc = req.params.get("Description").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        match state.get_key(key_id) {
            Some(key) => {
                *key.description.write() = desc.to_string();
                AwsResponse::json(200, json!({}))
            }
            None => AwsResponse::error(400, "NotFoundException", "Key not found"),
        }
    }

    fn get_public_key(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        match state.get_key(key_id) {
            Some(key) => AwsResponse::json(200, json!({
                "PublicKey": base64::encode(vec![0u8; 256]),
                "KeyMetadata": {
                    "KeyId": key.key_id,
                    "KeySpec": key.key_spec,
                    "KeyUsage": key.key_usage,
                    "CustomerMasterKeyId": key.key_id,
                    "Arn": key.arn,
                    "KeyManager": "CUSTOMER_MANAGED"
                }
            })),
            None => AwsResponse::error(400, "NotFoundException", "Key not found"),
        }
    }

    fn re_encrypt(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = req.params.get("KeyId").and_then(|v| v.as_str()).unwrap_or("");
        let ciphertext = req.params.get("CiphertextBlob").and_then(|v| v.as_str()).unwrap_or("");
        AwsResponse::json(200, json!({
            "Plaintext": base64::encode("decrypted"),
            "CiphertextBlob": ciphertext,
            "KeyId": key_id
        }))
    }
}

impl Default for KmsHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "kms".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_create_and_describe_key() {
        let handler = KmsHandler::new();
        let resp = handler.handle(make_req("CreateKey", json!({})));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("KeyId"));

        let key_id = serde_json::from_str::<Value>(&resp.body)
            .unwrap()["KeyMetadata"]["KeyId"].as_str().unwrap().to_string();

        let resp = handler.handle(make_req("DescribeKey", json!({"KeyId": key_id})));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("Enabled"));
    }

    #[test]
    fn test_list_keys() {
        let handler = KmsHandler::new();
        handler.handle(make_req("CreateKey", json!({})));
        handler.handle(make_req("CreateKey", json!({})));
        let resp = handler.handle(make_req("ListKeys", json!({})));
        assert_eq!(resp.status, 200);
        let keys: Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(keys["Keys"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_enable_disable_key() {
        let handler = KmsHandler::new();
        let resp = handler.handle(make_req("CreateKey", json!({})));
        let key_id = serde_json::from_str::<Value>(&resp.body)
            .unwrap()["KeyMetadata"]["KeyId"].as_str().unwrap().to_string();

        handler.handle(make_req("DisableKey", json!({"KeyId": key_id})));
        let resp = handler.handle(make_req("DescribeKey", json!({"KeyId": key_id})));
        assert!(resp.body.contains("Disabled"));

        handler.handle(make_req("EnableKey", json!({"KeyId": key_id})));
        let resp = handler.handle(make_req("DescribeKey", json!({"KeyId": key_id})));
        assert!(resp.body.contains("Enabled"));
    }
}
