//! SSM operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{Document, Parameter, SsmState};
use crate::protocol::{AwsRequest, AwsResponse};

pub struct SsmHandler {
    state: RwLock<HashMap<(u64, String), SsmState>>,
}

impl SsmHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> SsmState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(SsmState::new).clone()
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let op = req.operation.as_str();
        match op {
            "PutParameter" => self.put_parameter(&req),
            "GetParameter" => self.get_parameter(&req),
            "GetParameters" => self.get_parameters(&req),
            "DeleteParameter" => self.delete_parameter(&req),
            "GetParametersByPath" => self.get_parameters_by_path(&req),
            "CreateDocument" => self.create_document(&req),
            "GetDocument" => self.get_document(&req),
            "DeleteDocument" => self.delete_document(&req),
            "ListDocuments" => self.list_documents(&req),
            "DescribeParameters" => self.describe_parameters(&req),
            "AddTagsToResource" => self.add_tags(&req),
            "RemoveTagsFromResource" => self.remove_tags(&req),
            "ListTagsForResource" => self.list_tags(&req),
            "CreateActivation" => self.create_activation(&req),
            "DeregisterManagedInstance" => self.deregister_instance(&req),
            "CreateOpsItem" => self.json_stub(&req, "OpsItemId"),
            "GetOpsItem" => self.json_stub(&req, "OpsItemId"),
            "ListOpsItems" => self.json_stub_list(&req, "OpsItems"),
            "CreateMaintenanceWindow" => self.create_maintenance_window(&req),
            "GetMaintenanceWindow" => self.json_stub(&req, "WindowId"),
            "DeleteMaintenanceWindow" => self.json_stub(&req, "WindowId"),
            "DeleteParameters" => self.delete_parameters(&req),
            "LabelParameterVersion" => self.label_parameter_version(&req),
            "GetParameterHistory" => self.get_parameter_history(&req),
            "ListParameterVersions" => self.list_parameter_versions(&req),
            "GetParametersByPath" => self.json_stub_list(&req, "Parameters"),
            "CreatePatchBaseline" => self.json_stub(&req, "BaselineId"),
            "GetPatchBaseline" => self.json_stub(&req, "BaselineId"),
            "ListPatchBaselines" => self.json_stub_list(&req, "PatchBaselines"),
            "SendCommand" => self.send_command(&req),
            "ListCommands" => self.json_stub_list(&req, "Commands"),
            "CreateAssociation" => self.json_stub(&req, "AssociationVersion"),
            "GetAssociation" => self.json_stub(&req, "AssociationVersion"),
            "ListAssociations" => self.json_stub_list(&req, "Associations"),
            "CreateDocument" => self.json_stub(&req, "DocumentId"),
            "GetDocument" => self.json_stub(&req, "DocumentId"),
            "ListDocuments" => self.json_stub_list(&req, "Documents"),
            "AddTagsToResource" => self.json_stub(&req, "ResourceId"),
            "DescribeInstanceInformation" => self.json_stub_list(&req, "InstanceInformationList"),
            "GetInventory" => self.json_stub_list(&req, "Entities"),
            // Gap stubs - return empty/valid responses
            "DescribeAutomationExecutions" => self.json_stub_list(&req, "AutomationExecutionMetadataList"),
            "ListComplianceItems" => self.json_stub_list(&req, "ComplianceItems"),
            "ListComplianceSummaries" => self.json_stub_list(&req, "ComplianceSummaryItems"),
            "ListResourceComplianceSummaries" => self.json_stub_list(&req, "ResourceComplianceSummaryItems"),
            "DescribeOpsItems" => self.json_stub_list(&req, "OpsItemSummaries"),
            "DeleteOpsItem" => self.json_stub(&req, "{}"),
            "AssociateOpsItemRelatedItem" => self.json_stub(&req, "{}"),
            "CreateOpsMetadata" => self.json_stub(&req, "OpsMetadata"),
            "CreateResourceDataSync" => self.json_stub(&req, "{}"),
            "DeleteInventory" => self.json_stub(&req, "{}"),
            "DescribeActivations" => self.json_stub_list(&req, "Activations"),
            "DescribeAvailablePatches" => self.json_stub_list(&req, "Patches"),
            "DescribeDocument" => self.json_stub(&req, "Document"),
            "DescribeDocumentPermission" => self.json_stub(&req, "Permissions"),
            "DescribeEffectiveInstanceAssociations" => self.json_stub_list(&req, "InstanceAssociations"),
            "DescribeEffectivePatchesForPatchBaseline" => self.json_stub_list(&req, "Patches"),
            "DescribeInstanceAssociationsStatus" => self.json_stub_list(&req, "InstanceAssociationStatuses"),
            "DescribeInstancePatches" => self.json_stub(&req, "InstancePatchList"),
            "DescribeInstancePatchStates" => self.json_stub_list(&req, "InstancePatchStates"),
            "DescribeInstancePatchStatesForPatchGroup" => self.json_stub_list(&req, "InstancePatchStates"),
            "DescribeInstanceProperties" => self.json_stub_list(&req, "InstancePropertiesList"),
            "DescribeInventoryDeletions" => self.json_stub_list(&req, "InventoryDeletions"),
            "DescribeMaintenanceWindowExecutions" => self.json_stub_list(&req, "Executions"),
            "DescribeMaintenanceWindows" => self.json_stub_list(&req, "Items"),
            "DescribeMaintenanceWindowSchedule" => self.json_stub(&req, "Schedule"),
            "DescribeMaintenanceWindowsForTarget" => self.json_stub_list(&req, "Items"),
            "DescribePatchBaselines" => self.json_stub_list(&req, "PatchBaselines"),
            "DescribePatchGroups" => self.json_stub_list(&req, "PatchGroups"),
            "DescribePatchGroupState" => self.json_stub(&req, "PatchGroupState"),
            "DescribePatchProperties" => self.json_stub(&req, "Properties"),
            "DescribeSessions" => self.json_stub_list(&req, "Sessions"),
            "GetAccessToken" => self.json_stub(&req, "AccessToken"),
            "GetConnectionStatus" => self.json_stub(&req, "ConnectionStatus"),
            "GetDefaultPatchBaseline" => self.json_stub(&req, "PatchBaseline"),
            "GetDeployablePatchSnapshotForInstance" => self.json_stub(&req, "SnapshotId"),
            "GetInventorySchema" => self.json_stub_list(&req, "SchemaAttributes"),
            "GetOpsSummary" => self.json_stub(&req, "OpsSummary"),
            "GetResourcePolicies" => self.json_stub_list(&req, "ResourcePolicies"),
            "GetServiceSetting" => self.json_stub(&req, "Setting"),
            "ListCommandInvocations" => self.json_stub_list(&req, "CommandInvocations"),
            "ListDocumentMetadataHistory" => self.json_stub_list(&req, "DocumentVersionInfoList"),
            "ListDocumentVersions" => self.json_stub_list(&req, "DocumentVersions"),
            "ListInventoryEntries" => self.json_stub_list(&req, "Entries"),
            "ListNodes" => self.json_stub_list(&req, "Nodes"),
            "ListNodesByAssociation" => self.json_stub_list(&req, "Nodes"),
            "ListNodesSummary" => self.json_stub_list(&req, "Nodes"),
            "ListOpsItemEvents" => self.json_stub_list(&req, "Events"),
            "ListOpsMetadata" => self.json_stub_list(&req, "OpsMetadata"),
            "ListResourceDataSync" => self.json_stub_list(&req, "ResourceDataSyncList"),
            "ModifyDocumentPermission" => self.json_stub(&req, "{}"),
            "PutComplianceItems" => self.json_stub(&req, "{}"),
            "PutInventory" => self.json_stub(&req, "{}"),
            "PutResourcePolicy" => self.json_stub(&req, "{}"),
            "RegisterDefaultPatchBaseline" => self.json_stub(&req, "{}"),
            "StartAccessRequest" => self.json_stub(&req, "SessionId"),
            "StartAssociationsOnce" => self.json_stub_list(&req, "Associations"),
            "StartAutomationExecution" => self.json_stub(&req, "AutomationExecutionId"),
            "StartChangeRequestExecution" => self.json_stub(&req, "ChangeRequestId"),
            "StartExecutionPreview" => self.json_stub(&req, "ExecutionId"),
            "StartSession" => self.json_stub(&req, "SessionId"),
            "UpdateDocument" => self.json_stub(&req, "DocumentVersion"),
            "UpdateDocumentMetadata" => self.json_stub(&req, "{}"),
            "UpdateManagedInstanceRole" => self.json_stub(&req, "{}"),
            "UpdateOpsItem" => self.json_stub(&req, "OpsItemId"),
            "UpdateServiceSetting" => self.json_stub(&req, "{}"),
            "CreateAssociationBatch" => self.json_stub_list(&req, "AssociationDescriptions"),
            "DeleteResourceDataSync" => self.json_stub(&req, "{}"),
            "ResetServiceSetting" => self.json_stub(&req, "{}"),
            "ResumeSession" => self.json_stub(&req, "SessionId"),
            "TerminateSession" => self.json_stub(&req, "{}"),
            "UnlabelParameterVersion" => self.json_stub(&req, "{}"),
            "UpdateDocumentDefaultVersion" => self.json_stub(&req, "DocumentVersion"),
            other => AwsResponse::error(400, "InvalidParameterException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn param_response(p: &Parameter) -> Value {
        json!({
            "Name": p.name,
            "Value": *p.value.read(),
            "Type": p.parameter_type,
            "Version": *p.version.read(),
            "CreatedDate": p.created as f64,
            "LastModifiedDate": *p.modified.read() as f64,
            "LastModifiedBy": p.last_modified_by,
            "ARN": format!("arn:aws:ssm:{}:123456789012:parameter/{}", "us-east-1", p.name),
        })
    }

    fn put_parameter(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let value = req.params.get("Value").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let param_type = req.params.get("Type").and_then(|v| v.as_str()).unwrap_or("String").to_string();

        if name.is_empty() || !name.starts_with('/') {
            return AwsResponse::error(400, "InvalidParameterException",
                "Parameter name must start with /");
        }

        let state = self.get_state(req.account, &req.region);
        let now = chrono::Utc::now().timestamp() as u64;

        // Check Overwrite parameter
        let overwrite = req.params.get("Overwrite").and_then(|v| v.as_bool()).unwrap_or(false);
        if !overwrite && state.get_parameter(&name).is_some() {
            return AwsResponse::error(400, "ParameterAlreadyExists",
                &format!("An error occurred (ParameterAlreadyExists) when calling the PutParameter operation: Parameter {} already exists", name));
        }

        if let Some(existing) = state.get_parameter(&name) {
            let new_version = *existing.version.read() + 1;
            // Record history
            existing.history.write().push(crate::models::ParameterVersion {
                version: new_version,
                value: value.clone(),
                timestamp: now,
                label: req.params.get("Label").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            });
            *existing.value.write() = value;
            *existing.version.write() = new_version;
            *existing.modified.write() = now;
            if let Some(tags) = req.params.get("Tags").and_then(|v| v.as_array()).cloned() {
                *existing.tags.write() = tags;
            }
            return AwsResponse::json(200, json!({
                "Version": new_version,
                "Tier": "Standard"
            }));
        }

        let param = Arc::new(Parameter::new(name, value, param_type));
        if let Some(tags) = req.params.get("Tags").and_then(|v| v.as_array()).cloned() {
            *param.tags.write() = tags;
        }
        state.put_parameter(param);
        AwsResponse::json(200, json!({
            "Version": 1,
            "Tier": "Standard"
        }))
    }

    fn get_parameter(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("");
        let with_decryption = req.params.get("WithDecryption")
            .and_then(|v| v.as_bool()).unwrap_or(false);

        let state = self.get_state(req.account, &req.region);
        match state.get_parameter(name) {
            Some(param) => {
                let mut resp = Self::param_response(&param);
                if with_decryption {
                    // No-op for String type
                }
                AwsResponse::json(200, json!({ "Parameter": resp }))
            }
            None => AwsResponse::error(400, "ParameterNotFound",
                &format!("Parameter {name} not found")),
        }
    }

    fn get_parameters(&self, req: &AwsRequest) -> AwsResponse {
        let names: Vec<String> = req.params.get("Names")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let state = self.get_state(req.account, &req.region);
        let mut found = Vec::new();
        let mut invalid = Vec::new();
        for name in &names {
            match state.get_parameter(name) {
                Some(p) => found.push(Self::param_response(&p)),
                None => invalid.push(name.clone()),
            }
        }

        let mut resp = json!({
            "Parameters": found,
            "InvalidParameters": invalid
        });
        AwsResponse::json(200, resp)
    }

    fn delete_parameter(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        match state.delete_parameter(name) {
            Some(_) => AwsResponse::json(200, json!({})),
            None => AwsResponse::error(400, "ParameterNotFound",
                &format!("Parameter {name} not found")),
        }
    }

    fn get_parameters_by_path(&self, req: &AwsRequest) -> AwsResponse {
        let path = req.params.get("Path").and_then(|v| v.as_str()).unwrap_or("/");
        let recursive = req.params.get("Recursive").and_then(|v| v.as_bool()).unwrap_or(false);
        let state = self.get_state(req.account, &req.region);

        // Get all parameters and filter by path
        let all_params = state.all_parameters();
        let path_prefix = if path.ends_with('/') { path.to_string() } else { format!("{}/", path) };

        let params: Vec<_> = if recursive {
            all_params.iter().filter(|p| p.name.starts_with(&path_prefix)).collect()
        } else {
            // Non-recursive: only direct children
            all_params.iter().filter(|p| {
                if !p.name.starts_with(&path_prefix) { return false; }
                let rest = &p.name[path_prefix.len()..];
                !rest.contains('/')
            }).collect()
        };

        let max_results = req.params.get("MaxResults").and_then(|v| v.as_u64()).unwrap_or(10);
        let mut param_list: Vec<Value> = params.iter().map(|p| Self::param_response(p)).collect();
        let mut next_token = Value::Null;
        if param_list.len() > max_results as usize {
            next_token = json!(format!("offset-{}", max_results));
            param_list.truncate(max_results as usize);
        }
        AwsResponse::json(200, json!({
            "Parameters": param_list,
            "NextToken": next_token
        }))
    }

    fn create_document(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let content = req.params.get("Content").and_then(|v| v.as_str()).unwrap_or("{}").to_string();
        let doc_type = req.params.get("DocumentType").and_then(|v| v.as_str()).unwrap_or("Command").to_string();

        let state = self.get_state(req.account, &req.region);
        let now = chrono::Utc::now().timestamp() as u64;
        let doc = Arc::new(Document {
            name: name.clone(),
            content,
            document_type: doc_type,
            version: 1,
            created: now,
            status: "Active".to_string(),
        });
        state.documents.write().insert(name.clone(), doc.clone());

        AwsResponse::json(200, json!({
            "DocumentDescription": {
                "Name": name,
                "Version": 1,
                "DocumentType": doc.document_type,
                "DocumentStatus": doc.status,
                "CreationDate": doc.created as f64,
                "CreatedDate": doc.created as f64,
                "DocumentDescription": "",
                "Owner": "123456789012",
                "Hash": "0000000000000000000000000000000000000000000000000000000000000000",
                "HashType": "SHA256",
                "TargetType": ""
            }
        }))
    }

    fn get_document(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        let doc = state.documents.read().get(name).cloned();
        match doc {
            Some(doc) => AwsResponse::json(200, json!({
                "Content": doc.content,
                "DocumentVersion": doc.version.to_string(),
                "DocumentType": doc.document_type,
                "DocumentName": doc.name,
                "DocumentStatus": doc.status,
                "DocumentDescription": ""
            })),
            None => AwsResponse::error(400, "InvalidDocument",
                &format!("The document {name} does not exist")),
        }
    }

    fn delete_document(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        if state.documents.write().remove(name).is_none() {
            return AwsResponse::error(400, "InvalidDocument",
                &format!("The document {name} does not exist"));
        }
        AwsResponse::json(200, json!({}))
    }

    fn list_documents(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let docs: Vec<Arc<crate::models::Document>> = state.documents.read().values().cloned().collect();
        let doc_list: Vec<Value> = docs.iter().map(|d| json!({
            "Name": d.name,
            "Version": d.version,
            "DocumentType": d.document_type,
            "DocumentStatus": d.status,
            "CreationDate": d.created as f64,
            "CreatedDate": d.created as f64,
            "DocumentDescription": "",
            "Owner": "123456789012"
        })).collect();
        let identifiers: Vec<Value> = docs.iter().map(|d| json!({
            "Name": d.name,
            "Owner": "123456789012",
            "PlatformTypes": ["Not Implemented (moto)"],
            "DocumentVersion": format!("{}", d.version),
            "DocumentType": d.document_type,
            "SchemaVersion": "2.2",
            "DocumentFormat": "JSON"
        })).collect();
        AwsResponse::json(200, json!({
            "DocumentIdentifiers": identifiers,
            "NextToken": null
        }))
    }

    fn describe_parameters(&self, req: &AwsRequest) -> AwsResponse {
        let name_prefix = req.params.get("NamePrefix").and_then(|v| v.as_str()).unwrap_or("/");
        let state = self.get_state(req.account, &req.region);
        let params = state.list_parameters(name_prefix);
        let param_list: Vec<Value> = params.iter().map(|p| {
            json!({
                "Name": p.name,
                "Type": p.parameter_type,
                "Version": *p.version.read(),
                "LastModifiedDate": *p.modified.read() as f64,
                "LastModifiedBy": p.last_modified_by,
                "CreatedDate": p.created as f64,
            })
        }).collect();
        AwsResponse::json(200, json!({
            "Parameters": param_list,
            "NextToken": null
        }))
    }

    fn add_tags(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("ResourceId").and_then(|v| v.as_str())
            .or_else(|| req.params.get("ResourceName").and_then(|v| v.as_str()))
            .unwrap_or("");
        let tags = req.params.get("Tags").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(param) = state.get_parameter(&name) {
            let mut existing = param.tags.write().clone();
            existing.extend(tags);
            *param.tags.write() = existing;
        }
        AwsResponse::json(200, json!({}))
    }

    fn remove_tags(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("ResourceId").and_then(|v| v.as_str())
            .or_else(|| req.params.get("ResourceName").and_then(|v| v.as_str()))
            .unwrap_or("");
        let keys: Vec<String> = req.params.get("TagKeys")
            .or_else(|| req.params.get("Keys"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(param) = state.get_parameter(&name) {
            param.tags.write().retain(|t| {
                t.get("Key").and_then(|k| k.as_str())
                    .map(|k| !keys.contains(&k.to_string()))
                    .unwrap_or(true)
            });
        }
        AwsResponse::json(200, json!({}))
    }

    fn list_tags(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("ResourceId").and_then(|v| v.as_str())
            .or_else(|| req.params.get("ResourceName").and_then(|v| v.as_str()))
            .unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        let tags = state.get_parameter(name)
            .map(|p| p.tags.read().clone())
            .unwrap_or_default();
        AwsResponse::json(200, json!({ "TagList": tags }))
    }

    fn create_activation(&self, _req: &AwsRequest) -> AwsResponse {
        let activation_id = uuid::Uuid::new_v4().simple().to_string();
        let now = chrono::Utc::now().timestamp() as u64;
        AwsResponse::json(200, json!({
            "Activation": {
                "ActivationId": activation_id,
                "IamRole": "ssm-activation-role",
                "Description": "",
                "Name": "activation",
                "ActivationCode": "robotocore-activation-code",
                "ExpirationDate": (now + 365*24*3600*1000) as f64
            }
        }))
    }

    fn deregister_instance(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn get_parameter_history(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        let param = match state.get_parameter(name) {
            Some(p) => p,
            None => return AwsResponse::error(400, "ParameterNotFound",
                &format!("Parameter {} not found", name)),
        };
        let history = param.history.read();
        let mut versions: Vec<Value> = history.iter().map(|v| {
            let mut entry = json!({
                "Name": param.name,
                "Value": v.value,
                "Type": param.parameter_type,
                "Version": v.version,
                "LastModifiedDate": v.timestamp as f64,
                "LastModifiedBy": param.last_modified_by,
                "ARN": format!("arn:aws:ssm:{}:123456789012:parameter/{}", req.region, param.name),
            });
            if !v.label.is_empty() {
                let labels: Vec<&str> = v.label.split(',').collect();
                entry["Labels"] = json!(labels);
            }
            entry
        }).collect();
        let max_results = req.params.get("MaxResults").and_then(|v| v.as_u64()).unwrap_or(10);
        let mut next_token = Value::Null;
        if versions.len() > max_results as usize {
            next_token = json!(format!("offset-{}", max_results));
            versions.truncate(max_results as usize);
        }
        AwsResponse::json(200, json!({ "Parameters": versions, "NextToken": next_token }))
    }

    fn list_parameter_versions(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        let param = match state.get_parameter(name) {
            Some(p) => p,
            None => return AwsResponse::error(400, "ParameterNotFound",
                &format!("Parameter {} not found", name)),
        };
        let history = param.history.read();
        let versions: Vec<Value> = history.iter().map(|v| {
            let mut entry = json!({
                "Name": param.name,
                "Type": param.parameter_type,
                "Version": v.version,
                "LastModifiedDate": v.timestamp as f64,
                "LastModifiedBy": param.last_modified_by,
            });
            if !v.label.is_empty() {
                let labels: Vec<&str> = v.label.split(',').collect();
                entry["Labels"] = json!(labels);
            }
            entry
        }).collect();
        AwsResponse::json(200, json!({ "Parameters": versions }))
    }

    fn delete_parameters(&self, req: &AwsRequest) -> AwsResponse {
        let names: Vec<String> = req.params.get("Names")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut deleted = Vec::new();
        let mut invalid = serde_json::Map::new();
        for name in &names {
            if state.get_parameter(name).is_some() {
                state.delete_parameter(name);
                deleted.push(name.clone());
            } else {
                invalid.insert(name.clone(), json!(json!([])));
            }
        }
        AwsResponse::json(200, json!({
            "DeletedParameters": deleted,
            "InvalidParameters": invalid
        }))
    }

    fn create_maintenance_window(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("mw").to_string();
        let window_id = format!("mw-{}", uuid::Uuid::new_v4().simple().to_string().chars().take(20).collect::<String>());
        AwsResponse::json(200, json!({ "WindowId": window_id }))
    }

    fn label_parameter_version(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("");
        let version = req.params.get("ParameterVersion").and_then(|v| v.as_u64()).unwrap_or(0);
        let labels: Vec<String> = req.params.get("Labels")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(param) = state.get_parameter(name) {
            if let Some(v) = param.history.write().iter_mut().find(|v| v.version == version) {
                v.label = labels.join(",");
            }
        }
        // Return the parameter with labels
        match state.get_parameter(name) {
            Some(p) => {
                let mut resp = Self::param_response(&p);
                if !labels.is_empty() {
                    resp["Labels"] = json!(labels);
                }
                AwsResponse::json(200, json!({ "Parameter": resp }))
            }
            None => AwsResponse::error(400, "ParameterNotFound",
                &format!("Parameter {} not found", name)),
        }
    }

    // ---- JSON stub helpers ----
    fn json_stub(&self, _req: &AwsRequest, id_field: &str) -> AwsResponse {
        let mut obj = serde_json::Map::new();
        obj.insert(id_field.to_string(), serde_json::json!("stub-id"));
        AwsResponse::json(200, serde_json::Value::Object(obj))
    }
    fn json_stub_list(&self, _req: &AwsRequest, list_field: &str) -> AwsResponse {
        let mut obj = serde_json::Map::new();
        obj.insert(list_field.to_string(), serde_json::json!([]));
        AwsResponse::json(200, serde_json::Value::Object(obj))
    }
    fn json_stub_tags(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, serde_json::json!({"Tags": []}))
    }

    fn send_command(&self, req: &AwsRequest) -> AwsResponse {
        let doc_name = req.params.get("DocumentName")
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        let cmd_id = uuid::Uuid::new_v4().to_string();
        let status = json!({
            "CommandId": cmd_id,
            "DocumentName": doc_name,
            "Status": "Pending",
            "StatusDetails": "Started",
            "Comment": "",
            "Expires": chrono::Utc::now().timestamp_millis() + 3600000,
            "OutputS3Region": req.region,
            "OutputS3BucketName": "",
            "OutputS3KeyPrefix": "",
            "MaxErrors": "0",
            "MaxConcurrency": "1",
        });
        AwsResponse::json(200, json!({ "Command": status }))
    }
}

impl Default for SsmHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "ssm".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_put_and_get_parameter() {
        let handler = SsmHandler::new();
        handler.handle(make_req("PutParameter", json!({
            "Name": "/app/env",
            "Value": "production",
            "Type": "String"
        })));

        let resp = handler.handle(make_req("GetParameter", json!({
            "Name": "/app/env"
        })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("production"));
    }

    #[test]
    fn test_get_parameters_by_path() {
        let handler = SsmHandler::new();
        handler.handle(make_req("PutParameter", json!({
            "Name": "/app/env", "Value": "prod", "Type": "String"
        })));
        handler.handle(make_req("PutParameter", json!({
            "Name": "/app/db", "Value": "postgres", "Type": "String"
        })));

        let resp = handler.handle(make_req("GetParametersByPath", json!({
            "Path": "/app/"
        })));
        assert_eq!(resp.status, 200);
        let params: Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(params["Parameters"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_delete_parameter() {
        let handler = SsmHandler::new();
        handler.handle(make_req("PutParameter", json!({
            "Name": "/tmp", "Value": "val", "Type": "String"
        })));
        handler.handle(make_req("DeleteParameter", json!({ "Name": "/tmp" })));

        let resp = handler.handle(make_req("GetParameter", json!({ "Name": "/tmp" })));
        assert_eq!(resp.status, 400);
    }

    #[test]
    fn test_create_and_get_document() {
        let handler = SsmHandler::new();
        handler.handle(make_req("CreateDocument", json!({
            "Name": "my-doc",
            "Content": "{}",
            "DocumentType": "Command"
        })));

        let resp = handler.handle(make_req("GetDocument", json!({
            "Name": "my-doc"
        })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("my-doc"));
    }
}
