//! EventBridge operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{EventsState, Rule};
use crate::protocol::{AwsRequest, AwsResponse};

pub struct EventsHandler {
    state: RwLock<HashMap<(u64, String), EventsState>>,
}

impl EventsHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> EventsState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(EventsState::new).clone()
    }

    fn rule_value(r: &Rule) -> Value {
        let mut resp = json!({
            "Name": r.name,
            "Arn": r.arn,
            "State": r.state,
            "Description": r.description,
            "EventBusName": "default",
        });
        if !r.event_pattern.is_empty() {
            resp.as_object_mut().unwrap().insert("EventPattern".into(), json!(r.event_pattern));
        }
        if let Some(ref sched) = r.schedule_expression {
            resp.as_object_mut().unwrap().insert("ScheduleExpression".into(), json!(sched));
        }
        resp
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
            "PutRule" => self.put_rule(&req),
            "GetRule" => self.get_rule(&req),
            "DeleteRule" => self.delete_rule(&req),
            "ListRules" => self.list_rules(&req),
            "PutTargets" => self.put_targets(&req),
            "RemoveTargets" => self.remove_targets(&req),
            "ListTargetsByRule" => self.list_targets_by_rule(&req),
            "PutEvents" => self.put_events(&req),
            "TestEventPattern" => self.test_event_pattern(&req),
            "CreateEventBus" => self.create_event_bus(&req),
            "DeleteEventBus" => self.delete_event_bus(&req),
            "ListEventBuses" => self.list_event_buses(&req),
            "ActivateEventSource" => self.json_stub(&req, "{}"),
            "DeactivateEventSource" => self.json_stub(&req, "{}"),
            "CreateConnection" => self.json_stub(&req, "ConnectionArn"),
            "DeleteConnection" => self.json_stub(&req, "{}"),
            "UpdateConnection" => self.json_stub(&req, "ConnectionArn"),
            "TestEventConnection" => self.json_stub(&req, "{}"),
            "ListConnections" => self.json_stub_list(&req, "ConnectionList"),
            "CreateEndpoint" => self.json_stub(&req, "EndpointArn"),
            "DeleteEndpoint" => self.json_stub(&req, "{}"),
            "UpdateEndpoint" => self.json_stub(&req, "EndpointArn"),
            "TestEventSource" => self.json_stub(&req, "{}"),
            "CreatePartnerEventSource" => self.json_stub(&req, "PartnerEventSource"),
            "DeletePartnerEventSource" => self.json_stub(&req, "{}"),
            "ListPartnerEventSourceFactories" => self.json_stub_list(&req, "PartnerEventSourceFactories"),
            "CreateArchive" => self.create_archive(&req),
            "DeleteArchive" => self.json_stub(&req, "{}"),
            "DescribeArchive" => self.describe_archive(&req),
            "ListArchives" => self.list_archives(&req),
            "DescribeEventBus" => self.describe_event_bus(&req),
            "DescribeRule" => self.describe_rule(&req),
            "StartArchive" => self.json_stub(&req, "State"),
            "StopArchive" => self.json_stub(&req, "State"),
            "PutPartnerEvents" => self.json_stub(&req, "{}"),
                "DisableRule" => self.json_stub(&req, "{}"),
    "EnableRule" => self.json_stub(&req, "{}"),
    "TagResource" => self.json_stub(&req, "{}"),
    "UntagResource" => self.json_stub(&req, "{}"),
    "ListTagsForResource" => self.json_stub(&req, "{}"),
    "DeleteRule" => self.json_stub(&req, "{}"),
    "PutTargets" => self.json_stub(&req, "{}"),
    "RemoveTargets" => self.json_stub(&req, "{}"),
other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn put_rule(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if name.is_empty() {
            return AwsResponse::error(400, "ValidationException", "Name is required");
        }
        let state = self.get_state(req.account, &req.region);
        let rule = Arc::new(Rule {
            name: name.clone(),
            arn: format!("arn:aws:events:{}:{}:rule/{}", req.region, req.account, name),
            description: req.params.get("Description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            event_pattern: req.params.get("EventPattern").and_then(|v| v.as_str()).unwrap_or("{}").to_string(),
            state: "ENABLED".to_string(),
            role_arn: req.params.get("RoleArn").and_then(|v| v.as_str()).map(String::from),
            schedule_expression: req.params.get("ScheduleExpression").and_then(|v| v.as_str()).map(String::from),
            created: chrono::Utc::now().timestamp() as u64,
            targets: RwLock::new(Vec::new()),
            tags: RwLock::new(Vec::new()),
            inputs: RwLock::new(Vec::new()),
        });
        state.rules.write().insert(name.clone(), rule.clone());
        AwsResponse::json(200, json!({ "RuleArn": rule.arn }))
    }

    fn get_rule(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        match state.get_rule(name) {
            Some(rule) => AwsResponse::json(200, Self::rule_value(&rule)),
            None => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Rule not found: {}", name)),
        }
    }

    fn delete_rule(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if state.rules.write().remove(name).is_none() {
            return AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Rule not found: {}", name));
        }
        AwsResponse::json(200, json!({}))
    }

    fn list_rules(&self, req: &AwsRequest) -> AwsResponse {
        let name_prefix = req.params.get("NamePrefix").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        let rules: Vec<Value> = state.rules.read().values()
            .filter(|r| r.name.starts_with(name_prefix))
            .map(|r| Self::rule_value(r.as_ref()))
            .collect::<Vec<Value>>();
        AwsResponse::json(200, json!({ "Rules": rules }))
    }

    fn put_targets(&self, req: &AwsRequest) -> AwsResponse {
        let rule_name = req.params.get("Rule").and_then(|v| v.as_str()).unwrap_or_default();
        let targets: Vec<Value> = req.params.get("Targets")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(rule) = state.get_rule(rule_name) {
            let mut existing = rule.targets.write().clone();
            existing.extend(targets);
            *rule.targets.write() = existing;
        }
        AwsResponse::json(200, json!({
            "FailedEntryCount": 0,
            "FailedEntries": []
        }))
    }

    fn remove_targets(&self, req: &AwsRequest) -> AwsResponse {
        let rule_name = req.params.get("Rule").and_then(|v| v.as_str()).unwrap_or_default();
        let ids: Vec<String> = req.params.get("Ids")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(rule) = state.get_rule(rule_name) {
            rule.targets.write().retain(|t| {
                t.get("Id").and_then(|i| i.as_str())
                    .map(|i| !ids.contains(&i.to_string()))
                    .unwrap_or(true)
            });
        }
        AwsResponse::json(200, json!({
            "FailedEntryCount": 0,
            "FailedEntries": []
        }))
    }

    fn list_targets_by_rule(&self, req: &AwsRequest) -> AwsResponse {
        let rule_name = req.params.get("Rule").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let targets = state.get_rule(rule_name)
            .map(|r| r.targets.read().clone())
            .unwrap_or_default();
        AwsResponse::json(200, json!({
            "Targets": targets,
            "NextToken": null
        }))
    }

    fn put_events(&self, req: &AwsRequest) -> AwsResponse {
        let entries: Vec<Value> = req.params.get("Entries")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut failed: Vec<Value> = Vec::new();
        for _ in &entries {
            // All events succeed
        }
        AwsResponse::json(200, json!({
            "FailedEntryCount": failed.len(),
            "Entries": entries.iter().map(|_| json!({
                "EventId": uuid::Uuid::new_v4().to_string()
            })).collect::<Vec<_>>()
        }))
    }

    fn test_event_pattern(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "Result": true
        }))
    }

    fn create_event_bus(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let arn = format!("arn:aws:events:{}:{}:event-bus/{}", req.region, req.account, name);
        let state = self.get_state(req.account, &req.region);
        state.event_buses.write().insert(name.clone(), arn.clone());
        AwsResponse::json(200, json!({ "EventBusName": name, "EventBusArn": arn }))
    }

    fn delete_event_bus(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if state.event_buses.write().remove(name).is_none() {
            return AwsResponse::error(404, "ResourceNotFoundException",
                &format!("Event bus not found: {}", name));
        }
        AwsResponse::json(200, json!({}))
    }

    fn list_event_buses(&self, _req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(_req.account, &_req.region);
        let buses: Vec<Value> = state.event_buses.read().iter()
            .map(|(name, arn)| json!({
                "Name": name,
                "Arn": arn
            }))
            .collect();
        // Always include the default bus
        let mut all = vec![json!({
            "Name": "default",
            "Arn": format!("arn:aws:events:{}:123456789012:event-bus/default", _req.region)
        })];
        all.extend(buses);
        AwsResponse::json(200, json!({
            "EventBuses": all,
            "NextToken": null
        }))
    }

    fn describe_rule(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let state = self.get_state(req.account, &req.region);
        let rule = {
            let rules = state.rules.read();
            match rules.get(&name) {
                Some(r) => r.clone(),
                None => {
                    return AwsResponse::error(400, "ResourceNotFoundException",
                        &format!("Rule not found: {}", name));
                }
            }
        };
        let mut resp = json!({
            "Name": rule.name,
            "Arn": rule.arn,
            "State": rule.state,
            "EventBusName": "default",
        });
        if let Some(ref sched) = rule.schedule_expression {
            resp.as_object_mut().unwrap().insert("ScheduleExpression".into(), json!(sched));
        }
        if !rule.event_pattern.is_empty() {
            resp.as_object_mut().unwrap().insert("EventPattern".into(), json!(rule.event_pattern));
        }
        if !rule.description.is_empty() {
            resp.as_object_mut().unwrap().insert("Description".into(), json!(rule.description));
        }
        AwsResponse::json(200, resp)
    }

    fn describe_event_bus(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("Name").and_then(|v| v.as_str()).unwrap_or("default").to_string();
        let arn = format!("arn:aws:events:{}:{}:event-bus/{}", req.region, req.account, name);
        AwsResponse::json(200, json!({
            "Name": name,
            "Arn": arn
        }))
    }

    fn create_archive(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("ArchiveName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let arn = format!("arn:aws:events:{}:{}:archive/{}", req.region, req.account, name);
        AwsResponse::json(200, json!({ "ArchiveArn": arn }))
    }

    fn describe_archive(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("ArchiveName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let arn = format!("arn:aws:events:{}:{}:archive/{}", req.region, req.account, name);
        AwsResponse::json(200, json!({
            "ArchiveName": name,
            "ArchiveArn": arn,
            "EventSourceArn": format!("arn:aws:events:{}:{}:event-bus/default", req.region, req.account),
            "State": "ENABLED",
            "CreationTime": chrono::Utc::now().to_rfc3339()
        }))
    }

    fn list_archives(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({ "Archives": Vec::<Value>::new() }))
    }
}

impl Default for EventsHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "events".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_put_and_list_rules() {
        let handler = EventsHandler::new();
        handler.handle(make_req("PutRule", json!({
            "Name": "my-rule",
            "EventPattern": "{}"
        })));
        let resp = handler.handle(make_req("ListRules", json!({})));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("my-rule"));
    }

    #[test]
    fn test_put_events() {
        let handler = EventsHandler::new();
        let resp = handler.handle(make_req("PutEvents", json!({
            "Entries": [
                {"EventBusName": "default", "Detail": "{}", "DetailType": "test", "EventSource": "test"},
                {"EventBusName": "default", "Detail": "{}", "DetailType": "test", "EventSource": "test"}
            ]
        })));
        assert_eq!(resp.status, 200);
        let result: Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(result["FailedEntryCount"], 0);
    }
}
