//! Step Functions operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::StepfunctionsState;
use crate::protocol::{AwsRequest, AwsResponse};

pub struct StepfunctionsHandler {
    state: RwLock<HashMap<(u64, String), StepfunctionsState>>,
}

impl StepfunctionsHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> StepfunctionsState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(StepfunctionsState::new).clone()
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
            "CreateStateMachine" => self.create_sm(&req),
            "DeleteStateMachine" => self.delete_sm(&req),
            "DescribeStateMachine" => self.describe_sm(&req),
            "ListStateMachines" => self.list_sms(&req),
            "StartExecution" => self.start_execution(&req),
            "GetExecution" => self.get_execution(&req),
            "ListExecutions" => self.list_executions(&req),
            "StopExecution" => self.stop_execution(&req),
            "SendTaskSuccess" => self.send_task_success(&req),
            "SendTaskFailure" => self.send_task_failure(&req),
            "UpdateStateMachine" => self.update_sm(&req),
            "ListTagsForResource" => self.list_tags(&req),
            "TagResource" => self.tag_resource(&req),
            "UntagResource" => self.untag_resource(&req),
            "ListActivities" => self.list_activities(&req),
            "CreateActivity" => self.create_activity(&req),
            "DeleteActivity" => self.delete_activity(&req),
            "GetActivityTask" => self.get_activity_task(&req),
            "ListStateMachines" => self.list_sms(&req),
                        "DescribeActivity" => self.json_stub(&req, "Activity"),
            "DescribeExecution" => self.describe_exec(&req),
            "DescribeMapRun" => self.json_stub(&req, "MapRun"),
            "DescribeStateMachineForExecution" => self.json_stub(&req, "StateMachineForExecution"),
            "GetExecutionHistory" => self.get_execution_history(&req),
            "ListMapRuns" => self.json_stub_list(&req, "MapRuns"),
            "ListStateMachineAliases" => self.json_stub_list(&req, "StateMachineAliases"),
            "ListStateMachineVersions" => self.json_stub_list(&req, "StateMachineVersions"),
            "PublishStateMachineVersion" => self.json_stub(&req, "PublishStateMachineVersion"),
            "SendTaskHeartbeat" => self.json_stub(&req, "SendTaskHeartbeat"),
            "UpdateMapRun" => self.json_stub(&req, "MapRun"),
            "ValidateStateMachineDefinition" => self.json_stub(&req, "ValidateStateMachineDefinition"),
other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn create_sm(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("name").or_else(|| req.params.get("stateMachineName"))
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if name.is_empty() {
            return AwsResponse::error(400, "ValidationException", "name required");
        }
        let state = self.get_state(req.account, &req.region);
        let mut sms = state.state_machines.write();
        if sms.values().any(|sm| sm.get("name").and_then(|n| n.as_str()) == Some(&name)) {
            return AwsResponse::error(400, "StateMachineAlreadyExists",
                &format!("State machine {name} already exists"));
        }
        let arn = format!("arn:aws:states:{}:{}:stateMachine:{}", req.region, req.account, name);
        let sm = json!({
            "name": name,
            "arn": arn,
            "status": "ACTIVE",
            "type": req.params.get("type").and_then(|v| v.as_str()).unwrap_or("STANDARD"),
            "creationDate": chrono::Utc::now().timestamp() as u64,
            "roleArn": req.params.get("roleArn")
                .and_then(|v| v.as_str()).unwrap_or(""),
            "definition": req.params.get("definition")
                .and_then(|v| v.as_str()).unwrap_or("{}"),
        });
        sms.insert(name.clone(), sm.clone());
        AwsResponse::json(200, json!({
            "stateMachineArn": arn,
            "creationDate": sm["creationDate"]
        }))
    }

    fn delete_sm(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("stateMachineArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut sms = state.state_machines.write();
        let key = sms.iter()
            .find(|(_, sm)| sm.get("arn").and_then(|a| a.as_str()) == Some(arn))
            .map(|(k, _)| k.clone());
        if let Some(k) = key {
            sms.remove(&k);
            AwsResponse::json(200, json!({}))
        } else {
            AwsResponse::error(400, "StateMachineDoesNotExist",
                &format!("State machine {arn} not found"))
        }
    }

    fn describe_sm(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("stateMachineArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let sms = state.state_machines.read();
        if let Some((name, sm)) = sms.iter().find(|(_, sm)| {
            sm.get("arn").and_then(|a| a.as_str()) == Some(arn)
        }) {
            let mut v = sm.clone();
            v["stateMachineArn"] = sm.get("arn").cloned().unwrap_or(Value::Null);
            v["name"] = json!(name);
            v["status"] = json!("ACTIVE");
            v["type"] = sm.get("type").cloned().unwrap_or(json!("STANDARD"));
            v["creationDate"] = json!(chrono::Utc::now().to_rfc3339());
            AwsResponse::json(200, v)
        } else {
            AwsResponse::error(400, "StateMachineDoesNotExist",
                &format!("State machine {arn} not found"))
        }
    }

    fn list_sms(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let sms = state.state_machines.read();
        let items: Vec<Value> = sms.iter().map(|(name, sm)| {
            json!({
                "stateMachineArn": sm.get("arn").cloned().unwrap_or(Value::Null),
                "name": name,
                "type": sm.get("type").cloned().unwrap_or(json!("STANDARD")),
                "creationDate": sm.get("creationDate").cloned().unwrap_or(Value::Null),
                "status": sm.get("status").cloned().unwrap_or(json!("ACTIVE")),
                "roleArn": sm.get("roleArn").cloned().unwrap_or(Value::Null),
                "definition": sm.get("definition").cloned().unwrap_or(Value::Null),
            })
        }).collect();
        let next_token = req.params.get("nextToken").cloned().unwrap_or(Value::Null);
        AwsResponse::json(200, json!({
            "stateMachines": items,
            "nextToken": next_token
        }))
    }

    fn start_execution(&self, req: &AwsRequest) -> AwsResponse {
        let sm_arn = req.params.get("stateMachineArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let sms = state.state_machines.read();
        if !sms.values().any(|sm| sm.get("arn").and_then(|a| a.as_str()) == Some(sm_arn)) {
            return AwsResponse::error(400, "StateMachineDoesNotExist",
                &format!("State machine {sm_arn} not found"));
        }
        let exec_name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or_default();
        let exec_arn = if !exec_name.is_empty() {
            format!("{}:execution:{}", sm_arn, exec_name)
        } else {
            format!("{}:execution:{}", sm_arn, uuid::Uuid::new_v4().simple())
        };
        let now = chrono::Utc::now().timestamp() as u64;
        let exec_name_for_storage: String = if !exec_name.is_empty() {
            exec_name.to_string()
        } else {
            exec_arn.rsplit(':').next().unwrap_or("").to_string()
        };
        let exec = json!({
            "executionArn": exec_arn,
            "stateMachineArn": sm_arn,
            "name": exec_name_for_storage,
            "status": "SUCCEEDED",
            "startDate": now,
            "stopDate": now,
            "completed": true,
            "input": req.params.get("input").cloned().unwrap_or(Value::Null),
            "output": req.params.get("input").cloned().unwrap_or(Value::Null),
        });
        drop(sms);
        state.executions.write().insert(exec_arn.clone(), exec.clone());
        AwsResponse::json(200, exec)
    }

    fn get_execution(&self, req: &AwsRequest) -> AwsResponse {
        let exec_arn = req.params.get("executionArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let execs = state.executions.read();
        if let Some(exec) = execs.get(exec_arn) {
            AwsResponse::json(200, exec.clone())
        } else {
            AwsResponse::error(400, "ExecutionDoesNotExist",
                &format!("Execution {exec_arn} not found"))
        }
    }

    fn list_executions(&self, req: &AwsRequest) -> AwsResponse {
        let sm_arn = req.params.get("stateMachineArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let execs = state.executions.read();
        let items: Vec<Value> = execs.values()
            .filter(|exec| exec.get("stateMachineArn").and_then(|a| a.as_str()) == Some(sm_arn))
            .cloned()
            .collect();
        AwsResponse::json(200, json!({
            "executions": items,
            "nextToken": Value::Null
        }))
    }

    fn stop_execution(&self, req: &AwsRequest) -> AwsResponse {
        let exec_arn = req.params.get("executionArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut execs = state.executions.write();
        if let Some(exec) = execs.get_mut(exec_arn) {
            exec["status"] = json!("ABORTED");
            exec["stopDate"] = json!(chrono::Utc::now().timestamp() as u64);
            AwsResponse::json(200, json!({}))
        } else {
            AwsResponse::error(400, "ExecutionDoesNotExist",
                &format!("Execution {exec_arn} not found"))
        }
    }

    fn send_task_success(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn send_task_failure(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn update_sm(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("stateMachineArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut sms = state.state_machines.write();
        if let Some(sm) = sms.values_mut().find(|sm| {
            sm.get("arn").and_then(|a| a.as_str()) == Some(arn)
        }) {
            if let Some(def) = req.params.get("definition") {
                sm["definition"] = def.clone();
            }
            if let Some(role) = req.params.get("roleArn") {
                sm["roleArn"] = role.clone();
            }
        }
        let now = chrono::Utc::now().timestamp() as u64;
        AwsResponse::json(200, json!({
            "stateMachineArn": arn,
            "updateDate": now
        }))
    }

    fn list_tags(&self, req: &AwsRequest) -> AwsResponse {
        let resource_arn = req.params.get("resourceArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let tags = state.tags.read().get(resource_arn).cloned().unwrap_or_default();
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

    fn list_activities(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "activities": [],
            "nextToken": Value::Null
        }))
    }

    fn create_activity(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("name")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let arn = format!("arn:aws:states:{}:{}:activity:{}", req.region, req.account, name);
        AwsResponse::json(200, json!({ "activityArn": arn }))
    }

    fn delete_activity(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("activityArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        AwsResponse::json(200, json!({}))
    }

    fn get_activity_task(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "taskToken": uuid::Uuid::new_v4().simple().to_string()
        }))
    }

    fn describe_exec(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("executionArn").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let state = self.get_state(req.account, &req.region);
        let execs = state.executions.read();
        // Try to find by ARN or by name (last part of ARN)
        let name = arn.rsplit(':').next().unwrap_or(&arn).to_string();
        if let Some(exec) = execs.get(&arn).or_else(|| execs.get(&name)) {
            return AwsResponse::json(200, exec.clone());
        }
        // Check if any execution has this ARN
        for (key, exec) in execs.iter() {
            if exec.get("executionArn").and_then(|v| v.as_str()) == Some(&arn) {
                return AwsResponse::json(200, exec.clone());
            }
        }
        AwsResponse::error(400, "ExecutionDoesNotExist",
            &format!("Execution {arn} does not exist"))
    }

    fn get_execution_history(&self, req: &AwsRequest) -> AwsResponse {
        let execution_arn = req.params.get("executionArn").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        // Return a simple execution history with a few events
        let events: Vec<Value> = vec![
            json!({
                "id": 1,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "type": "ExecutionStarted",
                "detail": {
                    "executionArn": execution_arn,
                    "stateMachineArn": "",
                    "input": "{}"
                }
            }),
            json!({
                "id": 2,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "type": "ExecutionSucceeded",
                "detail": {
                    "executionArn": execution_arn,
                    "output": "{}"
                }
            })
        ];
        AwsResponse::json(200, json!({
            "events": events
        }))
    }
}

impl Default for StepfunctionsHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "stepfunctions".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_create_and_list_sms() {
        let handler = StepfunctionsHandler::new();
        handler.handle(make_req("CreateStateMachine", json!({
            "stateMachineName": "my-sm",
            "roleArn": "arn:aws:iam::123456789012:role/sfn-role",
            "definition": "{}"
        })));
        let resp = handler.handle(make_req("ListStateMachines", json!({})));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("my-sm"));
    }

    #[test]
    fn test_start_execution() {
        let handler = StepfunctionsHandler::new();
        handler.handle(make_req("CreateStateMachine", json!({
            "stateMachineName": "exec-sm",
            "roleArn": "arn:aws:iam::123456789012:role/sfn-role",
            "definition": "{}"
        })));
        let resp = handler.handle(make_req("StartExecution", json!({
            "stateMachineArn": "arn:aws:states:us-east-1:123456789012:stateMachine:exec-sm"
        })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("executionArn"));
    }
}
