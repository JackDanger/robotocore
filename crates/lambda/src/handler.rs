//! Lambda operation handler (rest-json protocol).

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{EventSourceMapping, LambdaAlias, LambdaFunction, LambdaState};
use crate::protocol::{AwsRequest, AwsResponse};

pub struct LambdaHandler {
    state: RwLock<HashMap<(u64, String), LambdaState>>,
}

impl LambdaHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> LambdaState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(LambdaState::new).clone()
    }

    fn func_config(f: &LambdaFunction) -> Value {
        json!({
            "FunctionName": f.function_name,
            "FunctionArn": f.function_arn,
            "Runtime": *f.runtime.read(),
            "Role": *f.role.read(),
            "Handler": *f.handler.read(),
            "Description": *f.description.read(),
            "Timeout": *f.timeout.read(),
            "MemorySize": *f.memory_size.read(),
            "CodeSize": f.code_size,
            "LastModified": *f.last_modified.read() as f64,
            "State": *f.state.read(),
            "Reason": *f.reason.read(),
            "Version": f.version,
            "CodeSha256": f.code_sha256,
            "Environment": f.environment.read().as_ref().cloned(),
            "TracingConfig": { "Mode": "PassThrough" },
            "RevisionId": uuid::Uuid::new_v4().simple().to_string(),
            "EphemeralStorage": { "Size": 512 },
            "PackageType": "Zip",
            "Architectures": ["x86_64"],
        })
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let op = req.operation.as_str();
        match op {
            "CreateFunction" => self.create_function(&req),
            "GetFunction" => self.get_function(&req),
            "GetFunctionConfiguration" => self.get_function_configuration(&req),
            "DeleteFunction" => self.delete_function(&req),
            "ListFunctions" => self.list_functions(&req),
            "UpdateFunctionCode" => self.update_function_code(&req),
            "UpdateFunctionConfiguration" => self.update_function_configuration(&req),
            "Invoke" => self.invoke(&req),
            "InvokeAsync" => self.invoke_async(&req),
            "AddPermission" => self.add_permission(&req),
            "RemovePermission" => self.remove_permission(&req),
            "GetPolicy" => self.get_policy(&req),
            "CreateAlias" => self.create_alias(&req),
            "GetAlias" => self.get_alias(&req),
            "UpdateAlias" => self.update_alias(&req),
            "DeleteAlias" => self.delete_alias(&req),
            "ListAliases" => self.list_aliases(&req),
            "PublishVersion" => self.publish_version(&req),
            "ListVersionsByFunction" => self.list_versions_by_function(&req),
            "CreateEventSourceMapping" => self.create_event_source_mapping(&req),
            "GetEventSourceMapping" => self.get_event_source_mapping(&req),
            "UpdateEventSourceMapping" => self.update_event_source_mapping(&req),
            "DeleteEventSourceMapping" => self.delete_event_source_mapping(&req),
            "ListEventSourceMappings" => self.list_event_source_mappings(&req),
            "TagResource" => self.tag_resource(&req),
            "UntagResource" => self.untag_resource(&req),
            "ListTags" => self.list_tags(&req),
            "PutFunctionConcurrency" => self.put_function_concurrency(&req),
            "GetFunctionConcurrency" => self.get_function_concurrency(&req),
            "DeleteFunctionConcurrency" => self.delete_function_concurrency(&req),
            "PublishLayerVersion" => self.publish_layer_version(&req),
            "GetLayerVersion" => self.get_layer_version(&req),
            "GetLayerVersionByArn" => self.get_layer_version_by_arn(&req),
            "DeleteLayerVersion" => self.delete_layer_version(&req),
            "ListLayerVersions" => self.list_layer_versions(&req),
            "ListLayers" => self.list_layers(&req),
            "AddLayerVersionPermission" => self.add_layer_version_permission(&req),
            "RemoveLayerVersionPermission" => self.remove_layer_version_permission(&req),
            "GetLayerVersionPolicy" => self.get_layer_version_policy(&req),
            "GetAccountSettings" => self.get_account_settings(&req),
            "CreateFunctionUrlConfig" => self.create_function_url_config(&req),
            "GetFunctionUrlConfig" => self.get_function_url_config(&req),
            "UpdateFunctionUrlConfig" => self.update_function_url_config(&req),
            "DeleteFunctionUrlConfig" => self.delete_function_url_config(&req),
            "ListFunctionUrlConfigs" => self.list_function_url_configs(&req),
            "PutProvisionedConcurrencyConfig" => self.put_provisioned_concurrency(&req),
            "GetProvisionedConcurrencyConfig" => self.get_provisioned_concurrency(&req),
            "DeleteProvisionedConcurrencyConfig" => self.delete_provisioned_concurrency(&req),
            "ListProvisionedConcurrencyConfigs" => self.list_provisioned_concurrency_configs(&req),
            "AddPermission" => self.json_stub(&req, "Statement"),
            "RemovePermission" => self.json_stub(&req, "{}"),
            "ListPermissions" => self.json_stub_list(&req, "Statements"),
            "CreateEventSourceMapping" => self.json_stub(&req, "UUID"),
            "UpdateEventSourceMapping" => self.json_stub(&req, "UUID"),
            "DeleteEventSourceMapping" => self.json_stub(&req, "{}"),
            "CreateFunctionUrlConfig" => self.json_stub(&req, "FunctionArn"),
            "GetFunctionUrlConfig" => self.json_stub(&req, "FunctionArn"),
            "UpdateFunctionUrlConfig" => self.json_stub(&req, "FunctionArn"),
            "DeleteFunctionUrlConfig" => self.json_stub(&req, "{}"),
            "GetLayerVersionByArn" => self.json_stub(&req, "LayerArn"),
            "GetCodeSigningConfig" => self.json_stub(&req, "CodeSigningConfigArn"),
            "PutCodeSigningConfig" => self.json_stub(&req, "CodeSigningConfigArn"),
            "DeleteCodeSigningConfig" => self.json_stub(&req, "{}"),
            "CreateCapacityProvider" => self.json_stub(&req, "CapacityProviderName"),
            "GetCapacityProvider" => self.json_stub(&req, "CapacityProviderName"),
            "UpdateCapacityProvider" => self.json_stub(&req, "CapacityProviderName"),
            "DeleteCapacityProvider" => self.json_stub(&req, "{}"),
            "ListCapacityProviders" => self.json_stub_list(&req, "CapacityProviderNames"),
            "CreateDurableExecution" => self.json_stub(&req, "ExecutionId"),
            "GetDurableExecution" => self.json_stub(&req, "ExecutionId"),
            "GetDurableExecutionHistory" => self.json_stub_list(&req, "Events"),
            "ListDurableExecutions" => self.json_stub_list(&req, "Executions"),
            "GetFunctionRecursionConfiguration" => self.json_stub(&req, "FunctionName"),
            "PutFunctionRecursionConfiguration" => self.json_stub(&req, "FunctionName"),
            "DeleteFunctionRecursionConfiguration" => self.json_stub(&req, "{}"),
            other => AwsResponse::error(400, "ResourceNotFoundException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn create_function(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("FunctionName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if name.is_empty() {
            return AwsResponse::error(400, "InvalidParameterValueException", "FunctionName is required");
        }
        let state = self.get_state(req.account, &req.region);
        if state.get_function(&name).is_some() {
            return AwsResponse::error(409, "ResourceConflictException",
                &format!("The function already exists: {}", name));
        }
        let func = Arc::new(LambdaFunction::new(req.account, &req.region, name));
        if let Some(runtime) = req.params.get("Runtime").and_then(|v| v.as_str()) {
            *func.runtime.write() = runtime.to_string();
        }
        if let Some(handler) = req.params.get("Handler").and_then(|v| v.as_str()) {
            *func.handler.write() = handler.to_string();
        }
        if let Some(role) = req.params.get("Role").and_then(|v| v.as_str()) {
            *func.role.write() = role.to_string();
        }
        if let Some(desc) = req.params.get("Description").and_then(|v| v.as_str()) {
            *func.description.write() = desc.to_string();
        }
        if let Some(timeout) = req.params.get("Timeout").and_then(|v| v.as_u64()) {
            *func.timeout.write() = timeout as u32;
        }
        if let Some(mem) = req.params.get("MemorySize").and_then(|v| v.as_u64()) {
            *func.memory_size.write() = mem as u32;
        }
        // Store environment variables
        if let Some(env) = req.params.get("Environment").and_then(|v| v.as_object()).cloned() {
            let env_value = serde_json::to_value(env).unwrap_or(Value::Null);
            *func.environment.write() = Some(env_value);
        }
        state.functions.write().insert(func.function_name.clone(), func.clone());
        AwsResponse::json(201, Self::func_config(&func))
    }

    fn get_function(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => AwsResponse::json(200, json!({
                "Configuration": Self::func_config(&func),
                "Code": {
                    "RepositoryType": "S3",
                    "Location": format!("https://s3.amazonaws.com/lambda-functions/{}/code.zip", func.function_arn),
                    "S3Bucket": "lambda-functions",
                    "S3Key": format!("{}/code.zip", func.function_arn),
                    "S3ObjectArn": format!("arn:aws:s3:::lambda-functions/{}/code.zip", func.function_arn)
                }
            })),
            None => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Function not found: {}", name)),
        }
    }

    fn get_function_configuration(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => AwsResponse::json(200, Self::func_config(&func)),
            None => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Function not found: {}", name)),
        }
    }

    fn delete_function(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => {
                state.functions.write().remove(&func.function_name);
                AwsResponse::json(202, json!({
                    "FunctionName": func.function_name,
                    "FunctionArn": func.function_arn,
                    "State": "Pending",
                    "LastModified": *func.last_modified.read() as f64,
                    "CodeSha256": func.code_sha256,
                    "Reason": ""
                }))
            }
            None => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Function not found: {}", name)),
        }
    }

    fn list_functions(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let funcs = state.functions.read().values().cloned().collect::<Vec<_>>();
        let configs: Vec<Value> = funcs.iter().map(|f| Self::func_config(f)).collect();
        AwsResponse::json(200, json!({
            "Functions": configs,
            "NextMarker": null
        }))
    }

    fn update_function_code(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => {
                *func.last_modified.write() = chrono::Utc::now().timestamp_millis() as u64;
                *func.state.write() = "Active".to_string();
                AwsResponse::json(200, Self::func_config(&func))
            }
            None => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Function not found: {}", name)),
        }
    }

    fn update_function_configuration(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => {
                if let Some(desc) = req.params.get("Description").and_then(|v| v.as_str()) {
                    *func.description.write() = desc.to_string();
                }
                if let Some(timeout) = req.params.get("Timeout").and_then(|v| v.as_u64()) {
                    *func.timeout.write() = timeout as u32;
                }
                if let Some(mem) = req.params.get("MemorySize").and_then(|v| v.as_u64()) {
                    *func.memory_size.write() = mem as u32;
                }
                if let Some(role) = req.params.get("Role").and_then(|v| v.as_str()) {
                    *func.role.write() = role.to_string();
                }
                if let Some(env) = req.params.get("Environment").and_then(|v| v.as_object()).cloned() {
                    let env_value = serde_json::to_value(env).unwrap_or(Value::Null);
                    *func.environment.write() = Some(env_value);
                }
                *func.last_modified.write() = chrono::Utc::now().timestamp_millis() as u64;
                AwsResponse::json(200, Self::func_config(&func))
            }
            None => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Function not found: {}", name)),
        }
    }

    fn invoke(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => {
                let payload = req.params.get("Payload")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}")
                    .to_string();

                // Check for dry run
                let is_dry_run = req.params.get("DryRun")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if is_dry_run {
                    return AwsResponse::raw(204, "application/json", String::new());
                }

                // Check for event response type
                let response_type = req.params.get("ResponseType")
                    .and_then(|v| v.as_str())
                    .unwrap_or("REQUEST_RESPONSE");
                if response_type == "EVENT" {
                    return AwsResponse::raw(202, "application/json", String::new());
                }

                // Echo the payload back as the function result
                AwsResponse::raw(200, "application/json", payload)
            }
            None => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Function not found: {}", name)),
        }
    }

    fn invoke_async(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(_) => AwsResponse::json(202, json!({
                "Status": 202,
                "RequestId": uuid::Uuid::new_v4().to_string()
            })),
            None => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Function not found: {}", name)),
        }
    }

    fn add_permission(&self, req: &AwsRequest) -> AwsResponse {
        let stmt_id = req.params.get("StatementId").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let action = req.params.get("Action").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let principal = req.params.get("Principal").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let sid = uuid::Uuid::new_v4().simple().to_string();
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        if let Some(func) = state.get_function(&name) {
            state.permissions.write().push(json!({
                "Sid": sid,
                "FunctionArn": func.function_arn,
                "Action": action,
                "Principal": principal,
                "StatementId": stmt_id
            }));
            return AwsResponse::json(201, json!({
                "Statement": format!("{{\"Sid\":\"{}\",\"Action\":\"{}\",\"Principal\":\"{}\"}}", sid, action, principal)
            }));
        }
        AwsResponse::error(404, "ResourceNotFoundException", "Function not found")
    }

    fn remove_permission(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let stmt_id = req.params.get("StatementId").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(func) = state.get_function(&name) {
            state.permissions.write().retain(|p| {
                !(p.get("FunctionArn").and_then(|a| a.as_str()) == Some(func.function_arn.as_str())
                    && p.get("StatementId").and_then(|s| s.as_str()) == Some(stmt_id))
            });
            return AwsResponse::json(204, Value::Null);
        }
        AwsResponse::error(404, "ResourceNotFoundException", "Function not found")
    }

    fn get_policy(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => {
                let stmts: Vec<Value> = state.permissions.read().iter()
                    .filter(|p| p.get("FunctionArn").and_then(|a| a.as_str()) == Some(func.function_arn.as_str()))
                    .cloned().collect();
                let stmt_str: Vec<String> = stmts.iter().map(|s| {
                    format!("{{\"Sid\":\"{}\",\"Action\":\"{}\",\"Principal\":\"{}\"}}",
                        s.get("StatementId").and_then(|v| v.as_str()).unwrap_or(""),
                        s.get("Action").and_then(|v| v.as_str()).unwrap_or(""),
                        s.get("Principal").and_then(|v| v.as_str()).unwrap_or(""))
                }).collect();
                let policy = format!(
                    "{{\"Version\":\"2012-10-17\",\"Statement\":[{}]}}",
                    stmt_str.join(",")
                );
                AwsResponse::json(200, json!({
                    "Policy": policy,
                    "FunctionArn": func.function_arn
                }))
            }
            None => AwsResponse::error(404, "ResourceNotFoundException", "Function not found"),
        }
    }

    fn create_alias(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let alias_name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let version = req.params.get("FunctionVersion").and_then(|v| v.as_str()).unwrap_or("$LATEST").to_string();
        let state = self.get_state(req.account, &req.region);
        let func = match state.get_function(&name) {
            Some(f) => f,
            None => return AwsResponse::error(404, "ResourceNotFoundException", "Function not found"),
        };
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let alias = Arc::new(LambdaAlias {
            name: alias_name.clone(),
            function_arn: func.function_arn.clone(),
            function_version: RwLock::new(version.clone()),
            description: RwLock::new(req.params.get("Description").and_then(|v| v.as_str()).unwrap_or("").to_string()),
            created: now,
            modified: RwLock::new(now),
            routing_config: json!({}),
        });
        state.aliases.write().insert((func.function_name.clone(), alias_name), alias.clone());
        AwsResponse::json(201, json!({
            "Name": alias.name,
            "FunctionArn": alias.function_arn,
            "FunctionVersion": *alias.function_version.read(),
            "Description": *alias.description.read(),
            "CreatedAt": alias.created as f64,
            "ModifiedAt": *alias.modified.read() as f64
        }))
    }

    fn get_alias(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        // The alias name is in the path: /2015-03-31/aliases/{AliasName}
        let alias_name = req.path.rsplit('/').next().unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let aliases = state.aliases.read();
        let alias = aliases.get(&(name.clone(), alias_name.clone())).cloned()
            .or_else(|| aliases.values().find(|a| a.name == alias_name).cloned());
        drop(aliases);
        match alias {
            Some(alias) => AwsResponse::json(200, json!({
                "Name": alias.name,
                "FunctionArn": alias.function_arn,
                "FunctionVersion": *alias.function_version.read(),
                "Description": *alias.description.read(),
                "CreatedAt": alias.created as f64,
                "ModifiedAt": *alias.modified.read() as f64
            })),
            None => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Alias not found: {}", alias_name)),
        }
    }

    fn update_alias(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let alias_name = req.path.rsplit('/').next().unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let aliases = state.aliases.read();
        let alias = aliases.get(&(name.clone(), alias_name.clone())).cloned()
            .or_else(|| aliases.values().find(|a| a.name == alias_name).cloned());
        drop(aliases);
        match alias {
            Some(alias) => {
                if let Some(desc) = req.params.get("Description").and_then(|v| v.as_str()) {
                    *alias.description.write() = desc.to_string();
                }
                if let Some(version) = req.params.get("FunctionVersion").and_then(|v| v.as_str()) {
                    *alias.function_version.write() = version.to_string();
                }
                *alias.modified.write() = chrono::Utc::now().timestamp_millis() as u64;
                AwsResponse::json(200, json!({
                    "Name": alias.name,
                    "FunctionArn": alias.function_arn,
                    "FunctionVersion": *alias.function_version.read(),
                    "Description": *alias.description.read(),
                    "ModifiedAt": *alias.modified.read() as f64
                }))
            }
            None => AwsResponse::error(404, "ResourceNotFoundException", "Alias not found"),
        }
    }

    fn delete_alias(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let alias_name = req.path.rsplit('/').next().unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let removed = state.aliases.write().remove(&(name, alias_name.clone())).is_some();
        if !removed {
            state.aliases.write().retain(|_, a| a.name != alias_name);
        }
        if removed {
            return AwsResponse::json(202, json!({
                "Name": alias_name,
                "Type": "ALIAS",
                "Operation": "Delete",
                "State": "Successful"
            }));
        }
        AwsResponse::error(404, "ResourceNotFoundException", "Alias not found")
    }

    fn list_aliases(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        let aliases: Vec<Value> = state.aliases.read().values()
            .filter(|a| {
                state.functions.read().get(&name)
                    .map(|f| a.function_arn == f.function_arn)
                    .unwrap_or(false)
            })
            .map(|a| json!({
                "Name": a.name,
                "FunctionArn": a.function_arn,
                "FunctionVersion": *a.function_version.read(),
                "Description": *a.description.read(),
                "CreatedAt": a.created as f64,
                "ModifiedAt": *a.modified.read() as f64
            }))
            .collect();
        AwsResponse::json(200, json!({
            "Items": aliases,
            "NextMarker": null
        }))
    }

    fn publish_version(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => AwsResponse::json(201, json!({
                "FunctionName": func.function_name,
                "FunctionArn": format!("{}:{}" , func.function_arn, "1"),
                "Runtime": *func.runtime.read(),
                "Role": *func.role.read(),
                "Handler": *func.handler.read(),
                "Description": *func.description.read(),
                "Timeout": *func.timeout.read(),
                "MemorySize": *func.memory_size.read(),
                "CodeSize": func.code_size,
                "PublishDate": *func.last_modified.read() as f64,
                "Version": "1",
                "State": "Active",
                "Reason": "",
                "CodeSha256": func.code_sha256,
                "RevisionId": uuid::Uuid::new_v4().simple().to_string()
            })),
            None => AwsResponse::error(404, "ResourceNotFoundException", "Function not found"),
        }
    }

    fn list_versions_by_function(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => {
                let mut v = Self::func_config(&func);
                v["Version"] = json!("$LATEST");
                AwsResponse::json(200, json!({
                    "Versions": [v],
                    "NextMarker": null
                }))
            }
            None => AwsResponse::error(404, "ResourceNotFoundException", "Function not found"),
        }
    }

    fn create_event_source_mapping(&self, req: &AwsRequest) -> AwsResponse {
        let function_name = req.params.get("FunctionName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let event_source_arn = req.params.get("EventSourceArn").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let id = uuid::Uuid::new_v4().simple().to_string();
        let state = self.get_state(req.account, &req.region);
        let func = match state.get_function(&function_name) {
            Some(f) => f,
            None => return AwsResponse::error(404, "ResourceNotFoundException", "Function not found"),
        };
        let em = Arc::new(EventSourceMapping {
            id: id.clone(),
            function_arn: func.function_arn.clone(),
            event_source_arn: event_source_arn.clone(),
            state: "Creating".to_string(),
            last_modified: chrono::Utc::now().timestamp_millis() as u64,
            batch_size: req.params.get("BatchSize").and_then(|v| v.as_u64()).unwrap_or(10) as u32,
            enabled: req.params.get("Enabled").and_then(|v| v.as_bool()).unwrap_or(true),
        });
        state.event_source_mappings.write().insert(id.clone(), em.clone());
        AwsResponse::json(202, json!({
            "ID": em.id,
            "UUID": em.id,
            "FunctionArn": em.function_arn,
            "EventSourceArn": em.event_source_arn,
            "State": em.state,
            "LastModified": em.last_modified as f64,
            "BatchSize": em.batch_size,
            "Enabled": em.enabled
        }))
    }

    fn get_event_source_mapping(&self, req: &AwsRequest) -> AwsResponse {
        let id = req.path.rsplit('/').next().unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let em = state.event_source_mappings.read().get(&id).cloned();
        match em {
            Some(em) => AwsResponse::json(200, json!({
                "ID": em.id,
                "FunctionArn": em.function_arn,
                "EventSourceArn": em.event_source_arn,
                "State": em.state,
                "LastModified": em.last_modified as f64,
                "BatchSize": em.batch_size,
                "Enabled": em.enabled
            })),
            None => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Event source mapping not found: {}", id)),
        }
    }

    fn update_event_source_mapping(&self, req: &AwsRequest) -> AwsResponse {
        let id = req.path.rsplit('/').next().unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let em = state.event_source_mappings.read().get(&id).cloned();
        match em {
            Some(em) => AwsResponse::json(200, json!({
                "ID": em.id,
                "FunctionArn": em.function_arn,
                "EventSourceArn": em.event_source_arn,
                "State": em.state,
                "LastModified": em.last_modified as f64,
                "BatchSize": em.batch_size,
                "Enabled": em.enabled
            })),
            None => AwsResponse::error(404, "ResourceNotFoundException", "Event source mapping not found"),
        }
    }

    fn delete_event_source_mapping(&self, req: &AwsRequest) -> AwsResponse {
        let id = req.path.rsplit('/').next().unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        let em = state.event_source_mappings.write().remove(&id);
        match em {
            Some(em) => AwsResponse::json(202, json!({
                "ID": em.id,
                "FunctionArn": em.function_arn,
                "EventSourceArn": em.event_source_arn,
                "State": "Deleting",
                "LastModified": em.last_modified as f64
            })),
            None => AwsResponse::error(404, "ResourceNotFoundException", "Event source mapping not found"),
        }
    }

    fn list_event_source_mappings(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let ems: Vec<Value> = state.event_source_mappings.read().values()
            .map(|em| json!({
                "ID": em.id,
                "FunctionArn": em.function_arn,
                "EventSourceArn": em.event_source_arn,
                "State": em.state,
                "LastModified": em.last_modified as f64,
                "BatchSize": em.batch_size,
                "Enabled": em.enabled
            }))
            .collect();
        AwsResponse::json(200, json!({
            "EventSourceMappings": ems,
            "NextToken": null
        }))
    }

    fn tag_resource(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        if let Some(func) = state.get_function(&name) {
            if let Some(tags) = req.params.get("Tags").and_then(|v| v.as_object()) {
                for (k, v) in tags {
                    func.tags.write().insert(k.clone(), v.as_str().unwrap_or("").to_string());
                }
            }
        }
        AwsResponse::json(204, Value::Null)
    }

    fn untag_resource(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        if let Some(func) = state.get_function(&name) {
            let keys: Vec<String> = req.params.get("TagKeys")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            func.tags.write().retain(|k, _| !keys.contains(k));
        }
        AwsResponse::json(204, Value::Null)
    }

    fn list_tags(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        let tags = state.get_function(&name)
            .map(|f| f.tags.read().clone())
            .unwrap_or_default();
        let tag_obj: serde_json::Map<String, Value> = tags.into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        AwsResponse::json(200, Value::Object(tag_obj))
    }

    fn put_function_concurrency(&self, req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(204, Value::Null)
    }

    fn get_function_concurrency(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => AwsResponse::json(200, json!({
                "FunctionArn": func.function_arn,
                "ReservedConcurrentExecutions": 100
            })),
            None => AwsResponse::error(404, "ResourceNotFoundException", "Function not found"),
        }
    }

    fn delete_function_concurrency(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(204, Value::Null)
    }

    fn publish_layer_version(&self, req: &AwsRequest) -> AwsResponse {
        let layer_name = req.params.get("LayerName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let state = self.get_state(req.account, &req.region);
        let arn = format!("arn:aws:lambda:{}:{}:layer:{}:1", req.region, req.account, layer_name);
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let layer = Arc::new(crate::models::LambdaLayer {
            layer_arn: arn.clone(),
            layer_name: layer_name.clone(),
            version: 1,
            description: req.params.get("Description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            created: now,
            content_size: 0,
            compatible_runtimes: vec!["python3.12".to_string()],
        });
        state.layers.write().entry(layer_name).or_default().push(layer.clone());
        AwsResponse::json(201, json!({
            "LayerArn": layer.layer_arn,
            "LayerVersionArn": format!("{}:1", layer.layer_arn),
            "LayerVersion": 1,
            "Description": layer.description,
            "CreatedDate": layer.created as f64,
            "ContentSize": layer.content_size,
            "CompatibleRuntimes": layer.compatible_runtimes
        }))
    }

    fn get_layer_version(&self, req: &AwsRequest) -> AwsResponse {
        // Path: /2018-01-01/layers/{LayerName}/versions/{VersionNumber}
        let parts: Vec<&str> = req.path.split('/').collect();
        let layer_name = parts.get(3).unwrap_or(&"").to_string();
        let state = self.get_state(req.account, &req.region);
        let layers = state.layers.read().get(&layer_name).cloned();
        match layers {
            Some(versions) if !versions.is_empty() => {
                let layer = &versions[0];
                AwsResponse::json(200, json!({
                    "LayerArn": layer.layer_arn,
                    "LayerVersionArn": format!("{}:{}", layer.layer_arn, layer.version),
                    "LayerVersion": layer.version,
                    "Description": layer.description,
                    "CreatedDate": layer.created as f64,
                    "ContentSize": layer.content_size,
                    "CompatibleRuntimes": layer.compatible_runtimes
                }))
            }
            _ => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Layer not found: {}", layer_name)),
        }
    }

    fn get_layer_version_by_arn(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.path.rsplit('/').next().unwrap_or("").to_string();
        let state = self.get_state(req.account, &req.region);
        for layers in state.layers.read().values() {
            for layer in layers {
                if layer.layer_arn.contains(&arn) || format!("{}:{}", layer.layer_arn, layer.version) == arn {
                    return AwsResponse::json(200, json!({
                        "LayerArn": layer.layer_arn,
                        "LayerVersionArn": format!("{}:{}", layer.layer_arn, layer.version),
                        "LayerVersion": layer.version,
                        "Description": layer.description,
                        "CreatedDate": layer.created as f64,
                        "ContentSize": layer.content_size,
                        "CompatibleRuntimes": layer.compatible_runtimes
                    }));
                }
            }
        }
        AwsResponse::error(404, "ResourceNotFoundException", "Layer not found")
    }

    fn delete_layer_version(&self, req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(204, Value::Null)
    }

    fn list_layer_versions(&self, req: &AwsRequest) -> AwsResponse {
        let parts: Vec<&str> = req.path.split('/').collect();
        let layer_name = parts.get(3).unwrap_or(&"").to_string();
        let state = self.get_state(req.account, &req.region);
        let layers = state.layers.read().get(&layer_name).cloned().unwrap_or_default();
        let items: Vec<Value> = layers.iter().map(|l| json!({
            "LayerArn": l.layer_arn,
            "LayerVersionArn": format!("{}:{}", l.layer_arn, l.version),
            "LayerVersion": l.version,
            "Description": l.description,
            "CreatedDate": l.created as f64,
            "ContentSize": l.content_size,
            "CompatibleRuntimes": l.compatible_runtimes
        })).collect();
        AwsResponse::json(200, json!({
            "Items": items,
            "NextMarker": null
        }))
    }

    fn list_layers(&self, _req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(_req.account, &_req.region);
        let layers: Vec<Value> = state.layers.read().iter()
            .filter_map(|(_, versions)| versions.first())
            .map(|l| json!({
                "LayerArn": l.layer_arn,
                "LayerName": l.layer_name,
                "Description": l.description,
                "CreatedDate": l.created as f64
            }))
            .collect();
        AwsResponse::json(200, json!({
            "Layers": layers,
            "NextMarker": null
        }))
    }

    fn add_layer_version_permission(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(201, Value::Null)
    }

    fn remove_layer_version_permission(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(204, Value::Null)
    }

    fn get_layer_version_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({ "Policy": "" }))
    }

    fn get_account_settings(&self, req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "AccountID": req.account.to_string(),
            "TotalTracedFunctions": 0,
            "TracedFunctions": []
        }))
    }

    fn create_function_url_config(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => {
                let url = format!("https://{}.{}.lambda-url.amazonaws.com/",
                    uuid::Uuid::new_v4().simple(), req.region);
                AwsResponse::json(201, json!({
                    "Url": url,
                    "FunctionArn": func.function_arn,
                    "FunctionUrlAuthType": req.params.get("AuthType").and_then(|v| v.as_str()).unwrap_or("AWS_IAM"),
                    "CreationTime": chrono::Utc::now().timestamp_millis() as f64
                }))
            }
            None => AwsResponse::error(404, "ResourceNotFoundException", "Function not found"),
        }
    }

    fn get_function_url_config(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => {
                let url = format!("https://{}.{}.lambda-url.amazonaws.com/",
                    "0000000000000000", req.region);
                AwsResponse::json(200, json!({
                    "Url": url,
                    "FunctionArn": func.function_arn,
                    "FunctionUrlAuthType": "AWS_IAM"
                }))
            }
            None => AwsResponse::error(404, "ResourceNotFoundException", "Function not found"),
        }
    }

    fn update_function_url_config(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "Url": "https://0000000000000000.us-east-1.lambda-url.amazonaws.com/",
            "FunctionArn": "",
            "FunctionUrlAuthType": "AWS_IAM"
        }))
    }

    fn delete_function_url_config(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(204, Value::Null)
    }

    fn list_function_url_configs(&self, req: &AwsRequest) -> AwsResponse {
        let name = self.extract_name(req);
        let state = self.get_state(req.account, &req.region);
        match state.get_function(&name) {
            Some(func) => AwsResponse::json(200, json!({
                "FunctionUrlConfigs": [{
                    "Url": format!("https://0000000000000000.{}.lambda-url.amazonaws.com/", req.region),
                    "FunctionArn": func.function_arn,
                    "FunctionUrlAuthType": "AWS_IAM"
                }],
                "NextToken": null
            })),
            None => AwsResponse::error(404, "ResourceNotFoundException", "Function not found"),
        }
    }

    fn put_provisioned_concurrency(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(202, json!({
            "FunctionArn": "",
            "RequestedProvisionedConcurrentExecutions": 1,
            "ProvisionedConcurrentExecutions": 0,
            "TargetProvisionedConcurrentExecutions": 1,
            "State": "InProgress",
            "StatusReason": "",
            "LastModified": 0.0
        }))
    }

    fn get_provisioned_concurrency(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "FunctionArn": "",
            "RequestedProvisionedConcurrentExecutions": 1,
            "ProvisionedConcurrentExecutions": 1,
            "TargetProvisionedConcurrentExecutions": 1,
            "State": "Ready",
            "StatusReason": "",
            "LastModified": 0.0
        }))
    }

    fn delete_provisioned_concurrency(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(202, json!({
            "FunctionArn": "",
            "RequestedProvisionedConcurrentExecutions": 0,
            "ProvisionedConcurrentExecutions": 0,
            "TargetProvisionedConcurrentExecutions": 0,
            "State": "InProgress",
            "StatusReason": "",
            "LastModified": 0.0
        }))
    }

    fn list_provisioned_concurrency_configs(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "ProvisionedConcurrencyConfigs": [],
            "NextToken": null
        }))
    }

    /// Extract the function name from the request path or params.
    fn json_stub(&self, _req: &AwsRequest, field: &str) -> AwsResponse {
        AwsResponse::json(200, json!({ field: "" }))
    }

    fn json_stub_list(&self, _req: &AwsRequest, field: &str) -> AwsResponse {
        AwsResponse::json(200, json!({ field: [] }))
    }

    fn extract_name(&self, req: &AwsRequest) -> String {
        // rest-json: path is like /2015-03-31/functions/{FunctionName}
        // or /2015-03-31/functions/{FunctionName}/invoke
        if let Some(idx) = req.path.rfind("/functions/") {
            let rest = &req.path[idx + "/functions/".len()..];
            let name = rest.split('/').next().unwrap_or("");
            if !name.is_empty() {
                return name.to_string();
            }
        }
        // Fallback to params
        req.params.get("FunctionName")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }
}

impl Default for LambdaHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value, path: &str) -> AwsRequest {
        AwsRequest {
            service: "lambda".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
            method: "POST".to_string(),
            path: path.to_string(),
            query_string: String::new(),
            headers: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_create_and_get_function() {
        let handler = LambdaHandler::new();
        handler.handle(make_req("CreateFunction", json!({
            "FunctionName": "test-func",
            "Runtime": "python3.12",
            "Handler": "index.handler",
            "Role": "arn:aws:iam::123456789012:role/lambda"
        }), "/2015-03-31/functions"));

        let resp = handler.handle(make_req("GetFunction", json!({}),
            "/2015-03-31/functions/test-func"));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("test-func"));
    }

    #[test]
    fn test_list_functions() {
        let handler = LambdaHandler::new();
        handler.handle(make_req("CreateFunction", json!({
            "FunctionName": "func1"
        }), "/2015-03-31/functions"));
        handler.handle(make_req("CreateFunction", json!({
            "FunctionName": "func2"
        }), "/2015-03-31/functions"));
        let resp = handler.handle(make_req("ListFunctions", json!({}),
            "/2015-03-31/functions"));
        assert_eq!(resp.status, 200);
        let funcs: Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(funcs["Functions"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_invoke() {
        let handler = LambdaHandler::new();
        handler.handle(make_req("CreateFunction", json!({
            "FunctionName": "inv-func"
        }), "/2015-03-31/functions"));
        let resp = handler.handle(make_req("Invoke", json!({
            "Payload": "{}"
        }), "/2015-03-31/functions/inv-func/invocations"));
        assert_eq!(resp.status, 200);
    }

    #[test]
    fn test_aliases() {
        let handler = LambdaHandler::new();
        handler.handle(make_req("CreateFunction", json!({
            "FunctionName": "alias-func"
        }), "/2015-03-31/functions"));
        handler.handle(make_req("CreateAlias", json!({
            "Name": "prod",
            "FunctionVersion": "$LATEST"
        }), "/2015-03-31/functions/alias-func/aliases"));
        let resp = handler.handle(make_req("ListAliases", json!({}),
            "/2015-03-31/functions/alias-func/aliases"));
        assert_eq!(resp.status, 200);
        let items: Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(items["Items"].as_array().unwrap().len(), 1);
    }
}
