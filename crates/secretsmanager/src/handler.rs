//! Secrets Manager operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{Secret, SecretVersion, SmState};
use crate::protocol::{AwsRequest, AwsResponse};

pub struct SecretsManagerHandler {
    state: RwLock<HashMap<(u64, String), SmState>>,
}

impl SecretsManagerHandler {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(HashMap::new()),
        }
    }

    fn get_state(&self, account: u64, region: &str) -> SmState {
        let mut states = self.state.write();
        states
            .entry((account, region.to_string()))
            .or_insert_with(SmState::new)
            .clone()
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let operation = req.operation.as_str();
        match operation {
            "CreateSecret" => self.create_secret(&req),
            "GetSecretValue" => self.get_secret_value(&req),
            "PutSecretValue" => self.put_secret_value(&req),
            "UpdateSecret" => self.update_secret(&req),
            "DeleteSecret" => self.delete_secret(&req),
            "RestoreSecret" => self.restore_secret(&req),
            "DescribeSecret" => self.describe_secret(&req),
            "ListSecrets" => self.list_secrets(&req),
            "ListSecretVersionIds" => self.list_secret_version_ids(&req),
            "TagResource" => self.tag_resource(&req),
            "UntagResource" => self.untag_resource(&req),
            "GetResourcePolicy" => self.get_resource_policy(&req),
            "PutResourcePolicy" => self.put_resource_policy(&req),
            "DeleteResourcePolicy" => self.delete_resource_policy(&req),
            "ValidateResourcePolicy" => self.validate_resource_policy(&req),
            "GetRandomPassword" => self.get_random_password(&req),
            "RotateSecret" => self.rotate_secret(&req),
            "CancelRotateSecret" => self.cancel_rotate_secret(&req),
            "UpdateSecretVersionStage" => self.update_secret_version_stage(&req),
            other => AwsResponse::error(
                400,
                "InvalidParameterException",
                &format!("The operation {} is not implemented", other),
            ),
        }
    }

    fn create_secret(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if name.is_empty() {
            return AwsResponse::error(400, "InvalidParameterException", "Name is required");
        }

        let state = self.get_state(req.account, &req.region);
        if state.get_secret(&name).is_some() {
            return AwsResponse::error(409, "ResourceExistsException", "Secret already exists");
        }

        let secret = Arc::new(Secret::new(name.clone(), req.account, req.region.clone()));

        // Create initial version
        let version_id = uuid::Uuid::new_v4().simple().to_string();
        let secret_string = req.params.get("SecretString").and_then(|v| v.as_str()).map(String::from);
        let secret_binary = req.params.get("SecretBinary").and_then(|v| v.as_str()).map(String::from);

        let version = SecretVersion {
            version_id: version_id.clone(),
            secret_string,
            secret_binary,
            stages: vec!["AWSCURRENT".to_string()],
            created_date: chrono::Utc::now().timestamp_millis() as u64,
        };
        secret.versions.write().insert(version_id.clone(), version);

        if let Some(desc) = req.params.get("Description").and_then(|v| v.as_str()) {
            *secret.description.write() = Some(desc.to_string());
        }

        state.put_secret(secret.clone());

        AwsResponse::json(200, json!({
            "ARN": secret.arn,
            "Name": secret.name,
            "VersionId": version_id,
            "CreatedDate": secret.created_date as f64
        }))
    }

    fn get_secret_value(&self, req: &AwsRequest) -> AwsResponse {
        let id = req.params.get("SecretId").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let secret = match state.get_secret(&id) {
            Some(s) => s,
            None => {
                return AwsResponse::error(400, "ResourceNotFoundException",
                    &format!("Secrets Manager can't find the specified secret: {}", id));
            }
        };

        let stage = req.params.get("VersionStage")
            .and_then(|v| v.as_str())
            .unwrap_or("AWSCURRENT");
        let version = match secret.get_version(stage) {
            Some(v) => v,
            None => {
                return AwsResponse::error(400, "InvalidRequestException",
                    "The requested secret has no versions with the specified stage");
            }
        };

        let mut resp = json!({
            "ARN": secret.arn,
            "Name": secret.name,
            "VersionId": version.version_id,
            "CreatedDate": version.created_date as f64
        });
        if let Some(ref s) = version.secret_string {
            resp.as_object_mut().unwrap().insert("SecretString".into(), json!(s));
        }
        if let Some(ref b) = version.secret_binary {
            resp.as_object_mut().unwrap().insert("SecretBinary".into(), json!(b));
        }

        AwsResponse::json(200, resp)
    }

    fn put_secret_value(&self, req: &AwsRequest) -> AwsResponse {
        let id = req.params.get("SecretId").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let secret = match state.get_secret(&id) {
            Some(s) => s,
            None => {
                return AwsResponse::error(400, "ResourceNotFoundException",
                    &format!("Secrets Manager can't find the specified secret: {}", id));
            }
        };

        let version_id = uuid::Uuid::new_v4().simple().to_string();
        let secret_string = req.params.get("SecretString").and_then(|v| v.as_str()).map(String::from);
        let secret_binary = req.params.get("SecretBinary").and_then(|v| v.as_str()).map(String::from);

        // Remove AWSCURRENT from old version
        let mut versions = secret.versions.write();
        for v in versions.values_mut() {
            v.stages.retain(|s| s != "AWSCURRENT");
            if v.stages.is_empty() {
                v.stages.push("AWSPREVIOUS".to_string());
            }
        }

        let created_date = chrono::Utc::now().timestamp_millis() as u64;
        let version = SecretVersion {
            version_id: version_id.clone(),
            secret_string,
            secret_binary,
            stages: vec!["AWSCURRENT".to_string()],
            created_date,
        };
        versions.insert(version_id.clone(), version);

        AwsResponse::json(200, json!({
            "ARN": secret.arn,
            "Name": secret.name,
            "VersionId": version_id,
            "CreatedDate": created_date as f64
        }))
    }

    fn update_secret(&self, req: &AwsRequest) -> AwsResponse {
        let id = req.params.get("SecretId").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let secret = match state.get_secret(&id) {
            Some(s) => s,
            None => {
                return AwsResponse::error(400, "ResourceNotFoundException",
                    &format!("Secrets Manager can't find the specified secret: {}", id));
            }
        };

        let version_id = uuid::Uuid::new_v4().simple().to_string();
        let secret_string = req.params.get("SecretString").and_then(|v| v.as_str()).map(String::from);
        let secret_binary = req.params.get("SecretBinary").and_then(|v| v.as_str()).map(String::from);

        let mut versions = secret.versions.write();
        for v in versions.values_mut() {
            v.stages.retain(|s| s != "AWSCURRENT");
            if v.stages.is_empty() {
                v.stages.push("AWSPREVIOUS".to_string());
            }
        }

        let created_date = chrono::Utc::now().timestamp_millis() as u64;
        let version = SecretVersion {
            version_id: version_id.clone(),
            secret_string,
            secret_binary,
            stages: vec!["AWSCURRENT".to_string()],
            created_date,
        };
        versions.insert(version_id.clone(), version);

        if let Some(desc) = req.params.get("Description").and_then(|v| v.as_str()) {
            *secret.description.write() = Some(desc.to_string());
        }

        AwsResponse::json(200, json!({
            "ARN": secret.arn,
            "Name": secret.name,
            "VersionId": version_id,
            "CreatedDate": created_date as f64
        }))
    }

    fn delete_secret(&self, req: &AwsRequest) -> AwsResponse {
        let id = req.params.get("SecretId").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let secret = match state.get_secret(&id) {
            Some(s) => s,
            None => {
                return AwsResponse::error(400, "ResourceNotFoundException",
                    &format!("Secrets Manager can't find the specified secret: {}", id));
            }
        };

        *secret.removed.write() = true;
        let deletion_date = chrono::Utc::now().timestamp_millis() as u64 + 7 * 24 * 60 * 60 * 1000;
        *secret.deletion_date.write() = Some(deletion_date);

        AwsResponse::json(200, json!({
            "ARN": secret.arn,
            "Name": secret.name,
            "DeletionDate": deletion_date as f64
        }))
    }

    fn restore_secret(&self, req: &AwsRequest) -> AwsResponse {
        let id = req.params.get("SecretId").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let secret = match state.get_secret(&id) {
            Some(s) => s,
            None => {
                return AwsResponse::error(400, "ResourceNotFoundException",
                    &format!("Secrets Manager can't find the specified secret: {}", id));
            }
        };

        *secret.removed.write() = true;
        *secret.deletion_date.write() = None;

        AwsResponse::json(200, json!({
            "ARN": secret.arn,
            "Name": secret.name
        }))
    }

    fn describe_secret(&self, req: &AwsRequest) -> AwsResponse {
        let id = req.params.get("SecretId").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let secret = match state.get_secret(&id) {
            Some(s) => s,
            None => {
                return AwsResponse::error(400, "ResourceNotFoundException",
                    &format!("Secrets Manager can't find the specified secret: {}", id));
            }
        };

        let versions = secret.versions.read();
        let current = versions.values().find(|v| v.stages.iter().any(|s| s == "AWSCURRENT"));

        let mut resp = json!({
            "ARN": secret.arn,
            "Name": secret.name,
            "CreatedDate": secret.created_date as f64,
            "LastChangedDate": secret.created_date as f64,
            "Tags": *secret.tags.read(),
            "VersionIdsToStages": {
                "AWSCURRENT": current.map(|v| v.version_id.clone()).unwrap_or_default()
            }
        });
        if let Some(ref desc) = *secret.description.read() {
            resp.as_object_mut().unwrap().insert("Description".into(), json!(desc));
        }

        AwsResponse::json(200, resp)
    }

    fn list_secrets(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let secrets = state.list_secrets();

        let secret_list: Vec<Value> = secrets.iter().map(|s| {
            let mut obj = json!({
                "ARN": s.arn,
                "Name": s.name,
                "CreatedDate": s.created_date as f64,
                "LastChangedDate": s.created_date as f64,
                "Tags": *s.tags.read()
            });
            if let Some(ref desc) = *s.description.read() {
                obj.as_object_mut().unwrap().insert("Description".into(), json!(desc));
            }
            obj
        }).collect();

        AwsResponse::json(200, json!({
            "SecretList": secret_list
        }))
    }

    fn list_secret_version_ids(&self, req: &AwsRequest) -> AwsResponse {
        let id = req.params.get("SecretId").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let secret = match state.get_secret(&id) {
            Some(s) => s,
            None => {
                return AwsResponse::error(400, "ResourceNotFoundException",
                    &format!("Secrets Manager can't find the specified secret: {}", id));
            }
        };

        let versions = secret.versions.read();
        let mut version_ids = HashMap::new();
        for (vid, v) in versions.iter() {
            version_ids.insert(vid.clone(), v.stages.clone());
        }

        AwsResponse::json(200, json!({
            "ARN": secret.arn,
            "Name": secret.name,
            "VersionIdsToStages": version_ids
        }))
    }

    fn tag_resource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn untag_resource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn get_resource_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "ResourcePolicy": "{}"
        }))
    }

    fn put_resource_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn delete_resource_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn validate_resource_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "Valid": true
        }))
    }

    fn get_random_password(&self, req: &AwsRequest) -> AwsResponse {
        let length = req.params.get("PasswordLength").and_then(|v| v.as_u64()).unwrap_or(32) as usize;
        let charset = req.params.get("ExcludeCharacters").and_then(|v| v.as_str()).unwrap_or("");
        let mut password = String::new();
        let mut rng = rand_thread_local();
        for _ in 0..length {
            let c = (rng % 94) as u8 + 33; // printable ASCII
            if !charset.contains((c as char)) {
                password.push(c as char);
            }
        }
        AwsResponse::json(200, json!({
            "RandomPassword": password
        }))
    }

    fn rotate_secret(&self, req: &AwsRequest) -> AwsResponse {
        let id = req.params.get("SecretId").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        match state.get_secret(&id) {
            Some(s) => AwsResponse::json(200, json!({
                "ARN": s.arn,
                "Name": s.name,
                "VersionId": uuid::Uuid::new_v4().simple().to_string(),
                "ClientRequestToken": uuid::Uuid::new_v4().simple().to_string()
            })),
            None => AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Secrets Manager can't find the specified secret: {}", id)),
        }
    }

    fn cancel_rotate_secret(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn update_secret_version_stage(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }
}

fn rand_thread_local() -> u32 {
    // Simple pseudo-random for password generation
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    (nanos % 10000) as u32
}

impl Default for SecretsManagerHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "secretsmanager".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_create_and_get_secret() {
        let handler = SecretsManagerHandler::new();

        let req = make_req("CreateSecret", json!({
            "Name": "db/password",
            "SecretString": "hunter2"
        }));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("db/password"));

        let req = make_req("GetSecretValue", json!({
            "SecretId": "db/password"
        }));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("hunter2"));
    }

    #[test]
    fn test_list_secrets() {
        let handler = SecretsManagerHandler::new();
        handler.handle(make_req("CreateSecret", json!({
            "Name": "secret1",
            "SecretString": "value1"
        })));
        handler.handle(make_req("CreateSecret", json!({
            "Name": "secret2",
            "SecretString": "value2"
        })));

        let req = make_req("ListSecrets", json!({}));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("secret1"));
        assert!(resp.body.contains("secret2"));
    }

    #[test]
    fn test_delete_and_restore_secret() {
        let handler = SecretsManagerHandler::new();
        handler.handle(make_req("CreateSecret", json!({
            "Name": "to-delete",
            "SecretString": "temp"
        })));

        let req = make_req("DeleteSecret", json!({"SecretId": "to-delete"}));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);

        let req = make_req("RestoreSecret", json!({"SecretId": "to-delete"}));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
    }
}
