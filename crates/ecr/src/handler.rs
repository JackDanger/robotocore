//! ECR operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::EcrState;
use crate::protocol::{AwsRequest, AwsResponse};

pub struct EcrHandler {
    state: RwLock<HashMap<(u64, String), EcrState>>,
}

impl EcrHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> EcrState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(EcrState::new).clone()
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
            "CreateRepository" => self.create_repository(&req),
            "DeleteRepository" => self.delete_repository(&req),
            "DescribeRepositories" => self.describe_repositories(&req),
            "ListImages" => self.list_images(&req),
            "PutImage" => self.put_image(&req),
            "BatchGetImage" => self.batch_get_image(&req),
            "BatchDeleteImage" => self.batch_delete_image(&req),
            "GetAuthorizationToken" => self.get_authorization_token(&req),
            "ListTagsForResource" => self.list_tags(&req),
            "TagResource" => self.tag_resource(&req),
            "UntagResource" => self.untag_resource(&req),
            "GetRepositoryPolicy" => self.get_repository_policy(&req),
            "SetRepositoryPolicy" => self.set_repository_policy(&req),
            "DeleteRepositoryPolicy" => self.delete_repository_policy(&req),
            "StartImageScan" => self.start_image_scan(&req),
            "DescribeImageScanFindings" => self.describe_scan_findings(&req),
            "DescribeImageSigningStatus" => self.describe_signing_status(&req),
            "PutImageScanningConfiguration" => self.put_image_scanning_config(&req),
            "PutImageTagMutability" => self.put_image_tag_mutability(&req),
            "BatchCheckLayerAvailability" => self.batch_check_layer(&req),
            "InitiateLayerUpload" => self.initiate_layer_upload(&req),
            "UploadLayerPart" => self.upload_layer_part(&req),
            "CompleteLayerUpload" => self.complete_layer_upload(&req),
            "GetDownloadUrlForLayer" => self.get_download_url(&req),
            "DescribeRepositories" => self.describe_repositories(&req),
            "DescribeRegistry" => self.describe_registry(&req),
            "PutLifecyclePolicy" => self.put_lifecycle_policy(&req),
            "GetLifecyclePolicy" => self.get_lifecycle_policy(&req),
            "DeleteLifecyclePolicy" => self.delete_lifecycle_policy(&req),
            "PutRegistryScanningConfiguration" => self.put_registry_scanning(&req),
            "GetRegistryScanningConfiguration" => self.get_registry_scanning(&req),
            "BatchGetRepositoryScanningConfiguration" => self.batch_get_scanning_config(&req),
                        "CreatePullThroughCacheRule" => self.json_stub(&req, "PullThroughCacheRule"),
            "CreateRepositoryCreationTemplate" => self.json_stub(&req, "RepositoryCreationTemplate"),
            "DescribeImages" => self.json_stub(&req, "Images"),
            "DescribePullThroughCacheRules" => self.json_stub(&req, "PullThroughCacheRules"),
            "GetSigningConfiguration" => self.json_stub(&req, "SigningConfiguration"),
            "ListImageReferrers" => self.json_stub_list(&req, "ImageReferrers"),
            "ListPullTimeUpdateExclusions" => self.json_stub_list(&req, "PullTimeUpdateExclusions"),
            "PutAccountSetting" => self.json_stub(&req, "AccountSetting"),
            "PutRegistryPolicy" => self.json_stub(&req, "RegistryPolicy"),
            "PutReplicationConfiguration" => self.json_stub(&req, "ReplicationConfiguration"),
            "PutSigningConfiguration" => self.json_stub(&req, "SigningConfiguration"),
            "UpdateImageStorageClass" => self.json_stub(&req, "ImageStorageClass"),
other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn create_repository(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if name.is_empty() {
            return AwsResponse::error(400, "ValidationException", "repositoryName required");
        }
        let state = self.get_state(req.account, &req.region);
        let mut repos = state.repositories.write();
        if repos.contains_key(&name) {
            return AwsResponse::error(400, "RepositoryAlreadyExistsException",
                &format!("Repository {name} already exists"));
        }
        let arn = format!("arn:aws:ecr:{}:{}:repository/{}", req.region, req.account, name);
        let repo = json!({
            "repositoryName": name,
            "repositoryArn": arn,
            "repositoryUri": format!("{}.dkr.ecr.{}.amazonaws.com/{}", req.account, req.region, name),
            "createdAt": chrono::Utc::now().to_rfc3339(),
            "imageTagMutability": req.params.get("imageTagMutability")
                .and_then(|v| v.as_str()).unwrap_or("MUTABLE"),
            "imageScanningConfiguration": req.params.get("imageScanningConfiguration")
                .cloned().unwrap_or(json!({ "scanOnPush": true })),
            "encryptionType": "AES256",
        });
        repos.insert(name.clone(), repo.clone());
        AwsResponse::json(200, json!({ "repository": repo }))
    }

    fn delete_repository(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut repos = state.repositories.write();
        if repos.remove(name).is_none() {
            return AwsResponse::error(400, "RepositoryNotFoundException",
                &format!("Repository {name} not found"));
        }
        state.images.write().remove(name);
        AwsResponse::json(200, json!({
            "repository": {
                "repositoryName": name,
                "repositoryArn": format!("arn:aws:ecr:{}:{}:repository/{}", req.region, req.account, name),
                "repositoryUri": format!("{}.dkr.ecr.{}.amazonaws.com/{}", req.account, req.region, name),
            }
        }))
    }

    fn describe_repositories(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let repos = state.repositories.read();
        let repo_names: Vec<String> = req.params.get("repositoryNames")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let repos: Vec<Value> = if repo_names.is_empty() {
            repos.values().cloned().collect()
        } else {
            repo_names.iter()
                .filter_map(|n| repos.get(n).cloned())
                .collect()
        };
        AwsResponse::json(200, json!({ "repositories": repos }))
    }

    fn list_images(&self, req: &AwsRequest) -> AwsResponse {
        let repo_name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let images = state.images.read();
        let repo_images: Vec<Value> = images.get(repo_name)
            .cloned()
            .unwrap_or_default();
        let image_ids: Vec<Value> = repo_images.iter()
            .map(|img| json!({
                "imageDigest": img.get("imageDigest").cloned().unwrap_or(Value::Null),
                "imageTag": img.get("imageTag").cloned().unwrap_or(Value::Null),
            }))
            .collect();
        AwsResponse::json(200, json!({
            "imageIds": image_ids,
            "nextToken": Value::Null
        }))
    }

    fn put_image(&self, req: &AwsRequest) -> AwsResponse {
        let repo_name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let state = self.get_state(req.account, &req.region);
        if !state.repositories.read().contains_key(&repo_name) {
            return AwsResponse::error(400, "RepositoryNotFoundException",
                &format!("Repository {repo_name} not found"));
        }
        let digest = format!("sha256:{}", &uuid::Uuid::new_v4().simple().to_string()[..32]);
        let image = json!({
            "repositoryName": repo_name,
            "imageDigest": digest,
            "imageTag": req.params.get("imageTag").cloned().unwrap_or(Value::Null),
            "imageUrl": format!("{}.dkr.ecr.{}.amazonaws.com/{}:{}", req.account, req.region, repo_name,
                req.params.get("imageTag").and_then(|v| v.as_str()).unwrap_or("latest")),
            "imageSizeInBytes": 0,
            "imagePushedAt": chrono::Utc::now().to_rfc3339(),
        });
        let mut images = state.images.write();
        images.entry(repo_name).or_insert_with(Vec::new).push(image.clone());
        AwsResponse::json(200, json!({
            "image": image
        }))
    }

    fn batch_get_image(&self, req: &AwsRequest) -> AwsResponse {
        let repo_name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let image_ids: Vec<Value> = req.params.get("imageIds")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let images = state.images.read();
        let repo_images: Vec<Value> = images.get(repo_name)
            .cloned()
            .unwrap_or_default();
        let images: Vec<Value> = image_ids.iter()
            .filter_map(|id| repo_images.iter().find(|img| {
                img.get("imageDigest").and_then(|d| d.as_str())
                    == id.get("imageDigest").and_then(|d| d.as_str())
                    || img.get("imageTag").and_then(|t| t.as_str())
                        == id.get("imageTag").and_then(|t| t.as_str())
            })).cloned().collect();
        AwsResponse::json(200, json!({
            "images": images,
            "failures": []
        }))
    }

    fn batch_delete_image(&self, req: &AwsRequest) -> AwsResponse {
        let repo_name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let image_ids: Vec<Value> = req.params.get("imageIds")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut images = state.images.write();
        let repo_images = images.get_mut(repo_name);
        let mut deleted = Vec::new();
        if let Some(imgs) = repo_images {
            imgs.retain(|img| {
                let found = image_ids.iter().any(|id| {
                    img.get("imageDigest").and_then(|d| d.as_str())
                        == id.get("imageDigest").and_then(|d| d.as_str())
                        || img.get("imageTag").and_then(|t| t.as_str())
                            == id.get("imageTag").and_then(|t| t.as_str())
                });
                if found {
                    deleted.push(img.clone());
                }
                !found
            });
        }
        AwsResponse::json(200, json!({
            "imageIds": deleted.iter().map(|img| json!({
                "imageDigest": img.get("imageDigest").cloned().unwrap_or(Value::Null),
                "imageTag": img.get("imageTag").cloned().unwrap_or(Value::Null),
            })).collect::<Vec<_>>(),
            "failures": []
        }))
    }

    fn get_authorization_token(&self, _req: &AwsRequest) -> AwsResponse {
        let token = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "AWS:test".as_bytes()
        );
        AwsResponse::json(200, json!({
            "authorizationData": [{
                "authorizationToken": token,
                "expiresAt": (chrono::Utc::now() + chrono::Duration::hours(12)).to_rfc3339(),
                "proxyEndpoint": "https://123456789012.dkr.ecr.us-east-1.amazonaws.com"
            }]
        }))
    }

    fn list_tags(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("resourceArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let tags = state.tags.read().get(arn).cloned().unwrap_or_default();
        AwsResponse::json(200, json!({ "tags": tags }))
    }

    fn tag_resource(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("resourceArn")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let tags: Vec<Value> = req.params.get("tags")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut all_tags = state.tags.write();
        let entry = all_tags.entry(arn).or_insert_with(Vec::new);
        entry.extend(tags);
        AwsResponse::json(200, json!({}))
    }

    fn untag_resource(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("resourceArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let keys: Vec<String> = req.params.get("tagKeys")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut all_tags = state.tags.write();
        if let Some(tags) = all_tags.get_mut(arn) {
            tags.retain(|t| {
                t.get("key").and_then(|k| k.as_str())
                    .map(|k| !keys.contains(&k.to_string()))
                    .unwrap_or(true)
            });
        }
        AwsResponse::json(200, json!({}))
    }

    fn get_repository_policy(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let policy = state.policies.read().get(name).cloned();
        match policy {
            Some(p) => AwsResponse::json(200, json!({
                "repositoryName": name,
                "policyText": p
            })),
            None => AwsResponse::error(400, "RepositoryPolicyNotFoundException",
                &format!("Repository {name} has no policy")),
        }
    }

    fn set_repository_policy(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let policy = req.params.get("policyText")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let state = self.get_state(req.account, &req.region);
        state.policies.write().insert(name.clone(), policy.clone());
        AwsResponse::json(200, json!({
            "repositoryName": name,
            "policyText": policy
        }))
    }

    fn delete_repository_policy(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        state.policies.write().remove(name);
        AwsResponse::json(200, json!({}))
    }

    fn start_image_scan(&self, req: &AwsRequest) -> AwsResponse {
        let repo_name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let image_id = req.params.get("imageId").cloned().unwrap_or(Value::Null);
        let state = self.get_state(req.account, &req.region);
        let images = state.images.read();
        let repo_images: Vec<Value> = images.get(repo_name).cloned().unwrap_or_default();
        let image = if !image_id.is_null() {
            repo_images.iter().find(|img| {
                img.get("imageDigest").and_then(|d| d.as_str())
                    == image_id.get("imageDigest").and_then(|d| d.as_str())
            }).cloned()
        } else {
            repo_images.last().cloned()
        };
        match image {
            Some(img) => AwsResponse::json(200, json!({
                "imageId": {
                    "imageDigest": img.get("imageDigest").cloned().unwrap_or(Value::Null),
                    "imageTag": img.get("imageTag").cloned().unwrap_or(Value::Null),
                },
                "initiateTime": chrono::Utc::now().timestamp_millis(),
                "imageScanStatus": "IN_PROGRESS"
            })),
            None => AwsResponse::error(400, "ImageNotFoundException", "Image not found"),
        }
    }

    fn describe_scan_findings(&self, req: &AwsRequest) -> AwsResponse {
        let repo_name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let images = state.images.read();
        let repo_images: Vec<Value> = images.get(repo_name).cloned().unwrap_or_default();
        let findings: Vec<Value> = repo_images.iter()
            .map(|img| json!({
                "imageId": {
                    "imageDigest": img.get("imageDigest").cloned().unwrap_or(Value::Null),
                    "imageTag": img.get("imageTag").cloned().unwrap_or(Value::Null),
                },
                "imageScanStatus": "COMPLETE",
                "findingSeverityCounts": {},
            }))
            .collect();
        AwsResponse::json(200, json!({ "imageScanSummaries": findings }))
    }

    fn describe_signing_status(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({ "signingConfiguration": Value::Null }))
    }

    fn put_image_scanning_config(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let state = self.get_state(req.account, &req.region);
        let config = req.params.get("imageScanningConfiguration").cloned().unwrap_or(Value::Null);
        state.scanning_configs.write().insert(name, config);
        AwsResponse::json(200, json!({}))
    }

    fn put_image_tag_mutability(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn batch_check_layer(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "layers": [{ "digest": "sha256:abc123", "layerAvailability": true }],
            "failures": []
        }))
    }

    fn initiate_layer_upload(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "uploadId": uuid::Uuid::new_v4().simple().to_string(),
            "partSize": 8388608
        }))
    }

    fn upload_layer_part(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn complete_layer_upload(&self, _req: &AwsRequest) -> AwsResponse {
        let digest = format!("sha256:{}", uuid::Uuid::new_v4().simple());
        AwsResponse::json(200, json!({ "layerDigest": digest }))
    }

    fn get_download_url(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "downloadUrl": format!("https://123456789012.dkr.ecr.us-east-1.amazonaws.com/layer/{}", uuid::Uuid::new_v4().simple())
        }))
    }

    fn describe_registry(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "registryId": "123456789012",
            "replicationConfiguration": Value::Null,
            "registryType": "DEFAULT"
        }))
    }

    fn put_lifecycle_policy(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let policy = req.params.get("lifecyclePolicyText")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let state = self.get_state(req.account, &req.region);
        state.policies.write().insert(name.clone(), policy.clone());
        AwsResponse::json(200, json!({
            "repositoryName": name,
            "lifecyclePolicyText": policy
        }))
    }

    fn get_lifecycle_policy(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let policy = state.policies.read().get(name).cloned().unwrap_or_default();
        AwsResponse::json(200, json!({
            "repositoryName": name,
            "lifecyclePolicyText": policy
        }))
    }

    fn delete_lifecycle_policy(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("repositoryName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        state.policies.write().remove(name);
        AwsResponse::json(200, json!({}))
    }

    fn put_registry_scanning(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn get_registry_scanning(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "scanningConfiguration": {
                "scanOnPush": true,
                "imageSignatureWatchingEnabled": false
            }
        }))
    }

    fn batch_get_scanning_config(&self, req: &AwsRequest) -> AwsResponse {
        let names: Vec<String> = req.params.get("repositoryNames")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let configs = state.scanning_configs.read();
        let results: Vec<Value> = names.iter().map(|n| {
            json!({
                "repositoryName": n,
                "imageScanningConfiguration": configs.get(n).cloned().unwrap_or(json!({"scanOnPush": true}))
            })
        }).collect();
        AwsResponse::json(200, json!({ "imageScanningConfigurationDescriptions": results }))
    }
}

fn req_layers_param() -> Vec<Value> {
    Vec::new()
}

impl Default for EcrHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "ecr".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_create_and_list_repos() {
        let handler = EcrHandler::new();
        handler.handle(make_req("CreateRepository", json!({
            "repositoryName": "my-app"
        })));
        let resp = handler.handle(make_req("DescribeRepositories", json!({})));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("my-app"));
    }

    #[test]
    fn test_put_and_list_images() {
        let handler = EcrHandler::new();
        handler.handle(make_req("CreateRepository", json!({
            "repositoryName": "test-repo"
        })));
        handler.handle(make_req("PutImage", json!({
            "repositoryName": "test-repo",
            "imageTag": "v1"
        })));
        let resp = handler.handle(make_req("ListImages", json!({
            "repositoryName": "test-repo"
        })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("v1"));
    }
}
