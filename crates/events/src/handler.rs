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
        json!({
            "Name": r.name,
            "Arn": r.arn,
            "EventPattern": r.event_pattern,
            "State": r.state,
            "CreatedBy": "robotocore",
            "Description": r.description
        })
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
            "DeleteArchive" => self.json_stub(&req, "{}"),
            "DescribeArchive" => self.json_stub(&req, "Archive"),
            "DescribeEventBus" => self.json_stub(&req, "EventBusName"),
            "DescribeRule" => self.json_stub(&req, "Name"),
            "StartArchive" => self.json_stub(&req, "State"),
            "StopArchive" => self.json_stub(&req, "State"),
            "PutPartnerEvents" => self.json_stub(&req, "{}"),
                "CancelReplay" => { AwsResponse::json(200, json!({"ReplayArn": "", "State": "", "StateReason": ""})) },
    "CreateApiDestination" => { AwsResponse::json(200, json!({"ApiDestinationArn": "", "ApiDestinationState": "", "CreationTime": Value::Null, "LastModifiedTime": Value::Null})) },
    "CreateArchive" => { AwsResponse::json(200, json!({"ArchiveArn": "", "State": "", "StateReason": "", "CreationTime": Value::Null})) },
    "DeauthorizeConnection" => { AwsResponse::json(200, json!({"ConnectionArn": "", "ConnectionState": "", "CreationTime": Value::Null, "LastModifiedTime": Value::Null, "LastAuthorizedTime": Value::Null})) },
    "DeleteApiDestination" => { AwsResponse::json(200, json!({})) },
    "DescribeApiDestination" => { AwsResponse::json(200, json!({"ApiDestinationArn": "", "Name": "", "Description": "", "ApiDestinationState": "", "ConnectionArn": "", "InvocationEndpoint": "", "HttpMethod": "", "InvocationRateLimitPerSecond": 0, "CreationTime": Value::Null, "LastModifiedTime": Value::Null})) },
    "DescribeConnection" => { AwsResponse::json(200, json!({"ConnectionArn": "", "Name": "", "Description": "", "InvocationConnectivityParameters": json!({"ResourceParameters": json!({"ResourceConfigurationArn": "", "ResourceAssociationArn": ""})}), "ConnectionState": "", "StateReason": "", "AuthorizationType": "", "SecretArn": "", "KmsKeyIdentifier": "", "AuthParameters": json!({"BasicAuthParameters": json!({"Username": ""}), "OAuthParameters": json!({"ClientParameters": json!({"ClientID": ""}), "AuthorizationEndpoint": "", "HttpMethod": "", "OAuthHttpParameters": json!({"HeaderParameters": json!([]), "QueryStringParameters": json!([]), "BodyParameters": json!([])})}), "ApiKeyAuthParameters": json!({"ApiKeyName": ""}), "InvocationHttpParameters": json!({"HeaderParameters": json!([]), "QueryStringParameters": json!([]), "BodyParameters": json!([])}), "ConnectivityParameters": json!({"ResourceParameters": json!({"ResourceConfigurationArn": "", "ResourceAssociationArn": ""})})}), "CreationTime": Value::Null, "LastModifiedTime": Value::Null, "LastAuthorizedTime": Value::Null})) },
    "DescribeEndpoint" => { AwsResponse::json(200, json!({"Name": "", "Description": "", "Arn": "", "RoutingConfig": json!({"FailoverConfig": json!({"Primary": json!({"HealthCheck": ""}), "Secondary": json!({"Route": ""})})}), "ReplicationConfig": json!({"State": ""}), "EventBuses": json!([]), "RoleArn": "", "EndpointId": "", "EndpointUrl": "", "State": "", "StateReason": "", "CreationTime": Value::Null, "LastModifiedTime": Value::Null})) },
    "DescribeEventSource" => { AwsResponse::json(200, json!({"Arn": "", "CreatedBy": "", "CreationTime": Value::Null, "ExpirationTime": Value::Null, "Name": "", "State": ""})) },
    "DescribePartnerEventSource" => { AwsResponse::json(200, json!({"Arn": "", "Name": ""})) },
    "DescribeReplay" => { AwsResponse::json(200, json!({"ReplayName": "", "ReplayArn": "", "Description": "", "State": "", "StateReason": "", "EventSourceArn": "", "Destination": json!({"Arn": "", "FilterArns": json!([])}), "EventStartTime": Value::Null, "EventEndTime": Value::Null, "EventLastReplayedTime": Value::Null, "ReplayStartTime": Value::Null, "ReplayEndTime": Value::Null})) },
    "DisableRule" => { AwsResponse::json(200, json!({})) },
    "EnableRule" => { AwsResponse::json(200, json!({})) },
    "ListApiDestinations" => { AwsResponse::json(200, json!({"ApiDestinations": json!([]), "NextToken": ""})) },
    "ListArchives" => { AwsResponse::json(200, json!({"Archives": json!([]), "NextToken": ""})) },
    "ListEndpoints" => { AwsResponse::json(200, json!({"Endpoints": json!([]), "NextToken": ""})) },
    "ListEventSources" => { AwsResponse::json(200, json!({"EventSources": json!([]), "NextToken": ""})) },
    "ListPartnerEventSourceAccounts" => { AwsResponse::json(200, json!({"PartnerEventSourceAccounts": json!([]), "NextToken": ""})) },
    "ListPartnerEventSources" => { AwsResponse::json(200, json!({"PartnerEventSources": json!([]), "NextToken": ""})) },
    "ListReplays" => { AwsResponse::json(200, json!({"Replays": json!([]), "NextToken": ""})) },
    "ListRuleNamesByTarget" => { AwsResponse::json(200, json!({"RuleNames": json!([]), "NextToken": ""})) },
    "ListTagsForResource" => { AwsResponse::json(200, json!({"Tags": json!([])})) },
    "PutPermission" => { AwsResponse::json(200, json!({})) },
    "RemovePermission" => { AwsResponse::json(200, json!({})) },
    "StartReplay" => { AwsResponse::json(200, json!({"ReplayArn": "", "State": "", "StateReason": "", "ReplayStartTime": Value::Null})) },
    "TagResource" => { AwsResponse::json(200, json!({})) },
    "UntagResource" => { AwsResponse::json(200, json!({})) },
    "UpdateApiDestination" => { AwsResponse::json(200, json!({"ApiDestinationArn": "", "ApiDestinationState": "", "CreationTime": Value::Null, "LastModifiedTime": Value::Null})) },
    "UpdateArchive" => { AwsResponse::json(200, json!({"ArchiveArn": "", "State": "", "StateReason": "", "CreationTime": Value::Null})) },
    "UpdateEventBus" => { AwsResponse::json(200, json!({"Arn": "", "Name": "", "KmsKeyIdentifier": "", "Description": "", "DeadLetterConfig": json!({"Arn": ""}), "LogConfig": json!({"IncludeDetail": "", "Level": ""})})) },
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
