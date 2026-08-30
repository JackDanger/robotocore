//! CloudWatch Logs operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{LogEvent, LogGroup, LogStream, LogsState};
use crate::protocol::{AwsRequest, AwsResponse};

pub struct LogsHandler {
    state: RwLock<HashMap<(u64, String), LogsState>>,
}

impl LogsHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> LogsState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(LogsState::new).clone()
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
            "CreateLogGroup" => self.create_log_group(&req),
            "DeleteLogGroup" => self.delete_log_group(&req),
            "DescribeLogGroups" => self.describe_log_groups(&req),
            "GetLogGroup" => self.get_log_group(&req),
            "PutRetentionPolicy" => self.put_retention_policy(&req),
            "DescribeLogGroups" | "ListTagsLogGroup" => self.describe_log_groups(&req),
            "CreateLogStream" => self.create_log_stream(&req),
            "DeleteLogStream" => self.delete_log_stream(&req),
            "DescribeLogStreams" => self.describe_log_streams(&req),
            "PutLogEvents" => self.put_log_events(&req),
            "GetLogEvents" => self.get_log_events(&req),
            "FilterLogEvents" => self.filter_log_events(&req),
            "PutMetricFilter" => self.put_metric_filter(&req),
            "DescribeMetricFilters" => self.describe_metric_filters(&req),
            "DeleteMetricFilter" => self.delete_metric_filter(&req),
            "PutSubscriptionFilter" => self.put_subscription_filter(&req),
            "DescribeSubscriptionFilters" => self.describe_subscription_filters(&req),
            "DeleteSubscriptionFilter" => self.delete_subscription_filter(&req),
            "PutQueryDefinition" => self.put_query_definition(&req),
            "GetQueryResults" => self.get_query_results(&req),
            "StartQuery" => self.start_query(&req),
            "ListTagsLogGroup" => self.list_tags(&req),
            "CreateLogGroup" => self.create_log_group(&req),
            "TagLogGroup" => self.tag_log_group(&req),
            "UntagLogGroup" => self.untag_log_group(&req),
            "AssociateKmsKey" => self.json_stub(&req, "{}"),
            "CancelImportTask" => self.json_stub(&req, "{}"),
            "CreateExportTask" => self.json_stub(&req, "taskId"),
            "CreateImportTask" => self.json_stub(&req, "taskId"),
            "CreateLogAnomalyDetector" => self.json_stub(&req, "logGroupIdentifier"),
            "CreateScheduledQuery" => self.json_stub(&req, "scheduledQueryName"),
            "DeleteRetentionPolicy" => self.json_stub(&req, "{}"),
            "DescribeAccountPolicies" => self.json_stub_list(&req, "policies"),
            "DescribeConfigurationTemplates" => self.json_stub_list(&req, "configurationTemplates"),
            "DescribeDeliveries" => self.json_stub_list(&req, "deliveries"),
            "DescribeDeliveryDestinations" => self.json_stub_list(&req, "destinations"),
            "DescribeDeliverySources" => self.json_stub_list(&req, "sources"),
            "DescribeExportTasks" => self.json_stub_list(&req, "tasks"),
            "DescribeFieldIndexes" => self.json_stub_list(&req, "fieldIndexes"),
            "DescribeImportTaskBatches" => self.json_stub_list(&req, "taskBatches"),
            "DescribeImportTasks" => self.json_stub_list(&req, "tasks"),
            "DescribeIndexPolicies" => self.json_stub_list(&req, "indexPolicies"),
            "DescribeQueries" => self.json_stub_list(&req, "queries"),
            "DescribeQueryDefinitions" => self.json_stub_list(&req, "queryDefinitions"),
            "GetDataProtectionPolicy" => self.json_stub(&req, "dataProtectionPolicy"),
            "GetLogFields" => self.json_stub_list(&req, "fields"),
            "GetLogGroupFields" => self.json_stub_list(&req, "fields"),
            "ListAggregateLogGroupSummaries" => self.json_stub_list(&req, "aggregates"),
            "ListAnomalies" => self.json_stub_list(&req, "anomalies"),
            "ListIntegrations" => self.json_stub_list(&req, "integrations"),
            "ListLogAnomalyDetectors" => self.json_stub_list(&req, "logGroupIdentifiers"),
            "ListLogGroups" => self.json_stub_list(&req, "logGroups"),
            "ListScheduledQueries" => self.json_stub_list(&req, "scheduledQueryNames"),
            "PutAccountPolicy" => self.json_stub(&req, "{}"),
            "PutDeliveryDestination" => self.json_stub(&req, "destinationArn"),
            "PutDeliverySource" => self.json_stub(&req, "sourceArn"),
            "PutDestination" => self.json_stub(&req, "{}"),
            "PutResourcePolicy" => self.json_stub(&req, "{}"),
            "StopQuery" => self.json_stub(&req, "{}"),
            "TagResource" => self.json_stub(&req, "{}"),
            "TestMetricFilter" => self.json_stub_list(&req, "metricValues"),
                "AssociateSourceToS3TableIntegration" => { AwsResponse::json(200, json!({"identifier": ""})) },
    "CancelExportTask" => { AwsResponse::json(200, json!({})) },
    "CreateDelivery" => { AwsResponse::json(200, json!({"delivery": json!({"id": "", "arn": "", "deliverySourceName": "", "deliveryDestinationArn": "", "deliveryDestinationType": "", "recordFields": json!([]), "fieldDelimiter": "", "s3DeliveryConfiguration": json!({"suffixPath": "", "enableHiveCompatiblePath": false}), "tags": json!({})})})) },
    "DeleteAccountPolicy" => { AwsResponse::json(200, json!({})) },
    "DeleteDataProtectionPolicy" => { AwsResponse::json(200, json!({})) },
    "DeleteDelivery" => { AwsResponse::json(200, json!({})) },
    "DeleteDeliveryDestination" => { AwsResponse::json(200, json!({})) },
    "DeleteDeliveryDestinationPolicy" => { AwsResponse::json(200, json!({})) },
    "DeleteDeliverySource" => { AwsResponse::json(200, json!({})) },
    "DeleteDestination" => { AwsResponse::json(200, json!({})) },
    "DeleteIndexPolicy" => { AwsResponse::json(200, json!({})) },
    "DeleteIntegration" => { AwsResponse::json(200, json!({})) },
    "DeleteLogAnomalyDetector" => { AwsResponse::json(200, json!({})) },
    "DeleteQueryDefinition" => { AwsResponse::json(200, json!({"success": false})) },
    "DeleteResourcePolicy" => { AwsResponse::json(200, json!({})) },
    "DeleteScheduledQuery" => { AwsResponse::json(200, json!({})) },
    "DeleteTransformer" => { AwsResponse::json(200, json!({})) },
    "DescribeDestinations" => { AwsResponse::json(200, json!({"destinations": json!([]), "nextToken": ""})) },
    "DescribeResourcePolicies" => { AwsResponse::json(200, json!({"resourcePolicies": json!([]), "nextToken": ""})) },
    "DisassociateKmsKey" => { AwsResponse::json(200, json!({})) },
    "DisassociateSourceFromS3TableIntegration" => { AwsResponse::json(200, json!({"identifier": ""})) },
    "GetDelivery" => { AwsResponse::json(200, json!({"delivery": json!({"id": "", "arn": "", "deliverySourceName": "", "deliveryDestinationArn": "", "deliveryDestinationType": "", "recordFields": json!([]), "fieldDelimiter": "", "s3DeliveryConfiguration": json!({"suffixPath": "", "enableHiveCompatiblePath": false}), "tags": json!({})})})) },
    "GetDeliveryDestination" => { AwsResponse::json(200, json!({"deliveryDestination": json!({"name": "", "arn": "", "deliveryDestinationType": "", "outputFormat": "", "deliveryDestinationConfiguration": json!({"destinationResourceArn": ""}), "tags": json!({})})})) },
    "GetDeliveryDestinationPolicy" => { AwsResponse::json(200, json!({"policy": json!({"deliveryDestinationPolicy": ""})})) },
    "GetDeliverySource" => { AwsResponse::json(200, json!({"deliverySource": json!({"name": "", "arn": "", "resourceArns": json!([]), "service": "", "logType": "", "tags": json!({})})})) },
    "GetIntegration" => { AwsResponse::json(200, json!({"integrationName": "", "integrationType": "", "integrationStatus": "", "integrationDetails": json!({"openSearchIntegrationDetails": json!({"dataSource": json!({"dataSourceName": "", "status": json!({"status": "", "statusMessage": ""})}), "application": json!({"applicationEndpoint": "", "applicationArn": "", "applicationId": "", "status": json!({"status": "", "statusMessage": ""})}), "collection": json!({"collectionEndpoint": "", "collectionArn": "", "status": json!({"status": "", "statusMessage": ""})}), "workspace": json!({"workspaceId": "", "status": json!({"status": "", "statusMessage": ""})}), "encryptionPolicy": json!({"policyName": "", "status": json!({"status": "", "statusMessage": ""})}), "networkPolicy": json!({"policyName": "", "status": json!({"status": "", "statusMessage": ""})}), "accessPolicy": json!({"policyName": "", "status": json!({"status": "", "statusMessage": ""})}), "lifecyclePolicy": json!({"policyName": "", "status": json!({"status": "", "statusMessage": ""})})})})})) },
    "GetLogAnomalyDetector" => { AwsResponse::json(200, json!({"detectorName": "", "logGroupArnList": json!([]), "evaluationFrequency": "", "filterPattern": "", "anomalyDetectorStatus": "", "kmsKeyId": "", "creationTimeStamp": 0, "lastModifiedTimeStamp": 0, "anomalyVisibilityTime": 0})) },
    "GetLogObject" => { AwsResponse::json(200, json!({"fieldStream": json!({"fields": json!({"data": Value::Null}), "InternalStreamingException": json!({"message": ""})})})) },
    "GetLogRecord" => { AwsResponse::json(200, json!({"logRecord": json!({})})) },
    "GetScheduledQuery" => { AwsResponse::json(200, json!({"scheduledQueryArn": "", "name": "", "description": "", "queryLanguage": "", "queryString": "", "logGroupIdentifiers": json!([]), "scheduleExpression": "", "timezone": "", "startTimeOffset": 0, "destinationConfiguration": json!({"s3Configuration": json!({"destinationIdentifier": "", "roleArn": ""})}), "state": "", "lastTriggeredTime": 0, "lastExecutionStatus": "", "scheduleStartTime": 0, "scheduleEndTime": 0, "executionRoleArn": "", "creationTime": 0, "lastUpdatedTime": 0})) },
    "GetScheduledQueryHistory" => { AwsResponse::json(200, json!({"name": "", "scheduledQueryArn": "", "triggerHistory": json!([]), "nextToken": ""})) },
    "GetTransformer" => { AwsResponse::json(200, json!({"logGroupIdentifier": "", "creationTime": 0, "lastModifiedTime": 0, "transformerConfig": json!([])})) },
    "ListLogGroupsForQuery" => { AwsResponse::json(200, json!({"logGroupIdentifiers": json!([]), "nextToken": ""})) },
    "ListSourcesForS3TableIntegration" => { AwsResponse::json(200, json!({"sources": json!([]), "nextToken": ""})) },
    "ListTagsForResource" => { AwsResponse::json(200, json!({"tags": json!({})})) },
    "PutBearerTokenAuthentication" => { AwsResponse::json(200, json!({})) },
    "PutDataProtectionPolicy" => { AwsResponse::json(200, json!({"logGroupIdentifier": "", "policyDocument": "", "lastUpdatedTime": 0})) },
    "PutDeliveryDestinationPolicy" => { AwsResponse::json(200, json!({"policy": json!({"deliveryDestinationPolicy": ""})})) },
    "PutDestinationPolicy" => { AwsResponse::json(200, json!({})) },
    "PutIndexPolicy" => { AwsResponse::json(200, json!({"indexPolicy": json!({"logGroupIdentifier": "", "lastUpdateTime": 0, "policyDocument": "", "policyName": "", "source": ""})})) },
    "PutIntegration" => { AwsResponse::json(200, json!({"integrationName": "", "integrationStatus": ""})) },
    "PutLogGroupDeletionProtection" => { AwsResponse::json(200, json!({})) },
    "PutTransformer" => { AwsResponse::json(200, json!({})) },
    "StartLiveTail" => { AwsResponse::json(200, json!({"responseStream": json!({"sessionStart": json!({"requestId": "", "sessionId": "", "logGroupIdentifiers": json!([]), "logStreamNames": json!([]), "logStreamNamePrefixes": json!([]), "logEventFilterPattern": ""}), "sessionUpdate": json!({"sessionMetadata": json!({"sampled": false}), "sessionResults": json!([])}), "SessionTimeoutException": json!({"message": ""}), "SessionStreamingException": json!({"message": ""})})})) },
    "TestTransformer" => { AwsResponse::json(200, json!({"transformedLogs": json!([])})) },
    "UntagResource" => { AwsResponse::json(200, json!({})) },
    "UpdateAnomaly" => { AwsResponse::json(200, json!({})) },
    "UpdateDeliveryConfiguration" => { AwsResponse::json(200, json!({})) },
    "UpdateLogAnomalyDetector" => { AwsResponse::json(200, json!({})) },
    "UpdateScheduledQuery" => { AwsResponse::json(200, json!({"scheduledQueryArn": "", "name": "", "description": "", "queryLanguage": "", "queryString": "", "logGroupIdentifiers": json!([]), "scheduleExpression": "", "timezone": "", "startTimeOffset": 0, "destinationConfiguration": json!({"s3Configuration": json!({"destinationIdentifier": "", "roleArn": ""})}), "state": "", "lastTriggeredTime": 0, "lastExecutionStatus": "", "scheduleStartTime": 0, "scheduleEndTime": 0, "executionRoleArn": "", "creationTime": 0, "lastUpdatedTime": 0})) },
other => AwsResponse::error(400, "ResourceNotFoundException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn group_value(g: &LogGroup) -> Value {
        json!({
            "arn": g.arn,
            "logGroupName": g.name,
            "creationTime": g.created,
            "retentionInDays": *g.retention_in_days.read(),
            "storedBytes": 0,
            "mbPerDay": 0,
            "agingDate": 0,
            "dataProtection": { "enabled": false }
        })
    }

    fn create_log_group(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if name.is_empty() {
            return AwsResponse::error(400, "InvalidParameterException", "logGroupName is required");
        }
        let state = self.get_state(req.account, &req.region);
        if state.get_log_group(&name).is_some() {
            return AwsResponse::error(400, "ResourceAlreadyExistsException",
                &format!("The resource ({}) already exists.", name));
        }
        let group = Arc::new(LogGroup::new(req.account, &req.region, name));
        state.log_groups.write().insert(group.name.clone(), group);
        AwsResponse::json(200, json!({}))
    }

    fn delete_log_group(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if state.log_groups.write().remove(name).is_none() {
            return AwsResponse::error(404, "ResourceNotFoundException",
                &format!("The log group ({}) does not exist.", name));
        }
        AwsResponse::json(200, json!({}))
    }

    fn describe_log_groups(&self, req: &AwsRequest) -> AwsResponse {
        let prefix = req.params.get("logGroupNamePrefix").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        let groups: Vec<Value> = state.log_groups.read().values()
            .filter(|g| g.name.starts_with(prefix))
            .map(|g| Self::group_value(g.as_ref()))
            .collect::<Vec<Value>>();
        AwsResponse::json(200, json!({
            "logGroups": groups,
            "nextToken": null
        }))
    }

    fn get_log_group(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        match state.get_log_group(name) {
            Some(g) => AwsResponse::json(200, Self::group_value(&g)),
            None => AwsResponse::error(404, "ResourceNotFoundException",
                &format!("The log group ({}) does not exist.", name)),
        }
    }

    fn put_retention_policy(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default();
        let days = req.params.get("retentionInDays").and_then(|v| v.as_i64()).unwrap_or(90);
        let state = self.get_state(req.account, &req.region);
        if let Some(g) = state.get_log_group(name) {
            *g.retention_in_days.write() = Some(days);
        }
        AwsResponse::json(200, json!({ "logGroupName": name }))
    }

    fn stream_value(s: &LogStream, group_name: &str) -> Value {
        json!({
            "arn": s.arn,
            "logStreamName": s.name,
            "creationTime": s.created,
            "firstEventTimestamp": s.first_event_time,
            "lastEventTimestamp": s.last_event_time,
            "lastIngestionTime": s.last_ingested_time,
            "storedBytes": 0,
            "logGroupName": group_name
        })
    }

    fn create_log_stream(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let stream_name = req.params.get("logStreamName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let state = self.get_state(req.account, &req.region);
        let group = match state.get_log_group(&group_name) {
            Some(g) => g,
            None => return AwsResponse::error(404, "ResourceNotFoundException",
                &format!("The log group ({}) does not exist.", group_name)),
        };
        // Check if stream already exists
        if group.log_streams.read().iter().any(|s| s.name == stream_name) {
            return AwsResponse::error(400, "ResourceAlreadyExistsException",
                &format!("The log stream ({}) already exists.", stream_name));
        }
        let now = chrono::Utc::now().timestamp_millis();
        let stream = LogStream {
            name: stream_name.to_string(),
            arn: format!("{}:{}" , group.arn, stream_name),
            created: now as u64,
            first_event_time: now,
            last_event_time: now,
            last_ingested_time: now,
            store_name: None,
            events: RwLock::new(Vec::new()),
        };
        group.log_streams.write().push(stream);
        AwsResponse::json(200, json!({}))
    }

    fn delete_log_stream(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default();
        let stream_name = req.params.get("logStreamName").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(group) = state.get_log_group(group_name) {
            group.log_streams.write().retain(|s| s.name != stream_name);
            return AwsResponse::json(200, json!({}));
        }
        AwsResponse::error(404, "ResourceNotFoundException", "Log group not found")
    }

    fn describe_log_streams(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let prefix = req.params.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
        let streams: Vec<Value> = state.get_log_group(group_name)
            .map(|g| {
                g.log_streams.read().iter()
                    .filter(|s| prefix.is_empty() || s.name.starts_with(prefix))
                    .map(|s| Self::stream_value(s, &g.name))
                    .collect()
            })
            .unwrap_or_default();
        AwsResponse::json(200, json!({
            "logStreams": streams,
            "nextToken": null
        }))
    }

    fn put_log_events(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default();
        let stream_name = req.params.get("logStreamName").and_then(|v| v.as_str()).unwrap_or_default();
        let events: Vec<Value> = req.params.get("logEvents")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let group = match state.get_log_group(group_name) {
            Some(g) => g,
            None => return AwsResponse::error(404, "ResourceNotFoundException",
                &format!("The log group ({}) does not exist.", group_name)),
        };
        // Auto-create stream if it doesn't exist
        if !group.log_streams.read().iter().any(|s| s.name == stream_name) {
            let now = chrono::Utc::now().timestamp_millis();
            let stream = LogStream {
                name: stream_name.to_string(),
                arn: format!("{}:{}", group.arn, stream_name),
                created: now as u64,
                first_event_time: now,
                last_event_time: now,
                last_ingested_time: now,
                store_name: None,
                events: RwLock::new(Vec::new()),
            };
            group.log_streams.write().push(stream);
        }
        let mut rejected: Vec<Value> = Vec::new();
        let mut put_time = 0u64;
        if let Some(stream) = group.log_streams.read().iter().find(|s| s.name == stream_name) {
            let mut evs = stream.events.write();
            for e in &events {
                let ts = e.get("timestamp").and_then(|v| v.as_i64()).unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
                let msg = e.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
                evs.push(LogEvent {
                    timestamp: ts,
                    message: msg,
                    id: uuid::Uuid::new_v4().simple().to_string(),
                });
            }
            put_time = chrono::Utc::now().timestamp_millis() as u64;
        }
        AwsResponse::json(200, json!({
            "rejectedLogEventsInfo": rejected,
            "logStreamName": stream_name,
            "logGroupName": group_name
        }))
    }

    fn get_log_events(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default();
        let stream_name = req.params.get("logStreamName").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let events: Vec<Value> = state.get_log_group(group_name)
            .map(|g| {
                g.log_streams.read().iter()
                    .find(|s| s.name == stream_name)
                    .map(|s| {
                        s.events.read().iter().map(|e| {
                            json!({
                                "timestamp": e.timestamp,
                                "message": e.message,
                                "logStreamName": s.name,
                                "logStreamArn": s.arn
                            })
                        }).collect::<Vec<Value>>()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        AwsResponse::json(200, json!({
            "events": events,
            "nextForwardToken": null,
            "nextBackwardToken": null
        }))
    }

    fn filter_log_events(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = req.params.get("logGroupName")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let state = self.get_state(req.account, &req.region);
        let events: Vec<Value> = state.get_log_group(&group_name)
            .map(|g| {
                let streams = g.log_streams.read();
                let mut result = Vec::new();
                for s in streams.iter() {
                    let evs = s.events.read();
                    for e in evs.iter() {
                        result.push(json!({
                            "timestamp": e.timestamp,
                            "message": e.message,
                            "logStreamName": s.name
                        }));
                    }
                }
                result
            })
            .unwrap_or_default();
        AwsResponse::json(200, json!({
            "events": events,
            "searchedBy": null,
            "nextToken": null
        }))
    }

    fn put_metric_filter(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("filterName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let state = self.get_state(req.account, &req.region);
        state.metric_filters.write().push(json!({
            "filterName": name,
            "filterPattern": req.params.get("filterPattern").and_then(|v| v.as_str()).unwrap_or(""),
            "logGroupName": req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or(""),
            "metricTransformations": req.params.get("metricTransformations").cloned().unwrap_or_default()
        }));
        AwsResponse::json(200, json!({}))
    }

    fn describe_metric_filters(&self, _req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(_req.account, &_req.region);
        let filters: Vec<Value> = state.metric_filters.read().clone();
        AwsResponse::json(200, json!({
            "metricFilters": filters,
            "nextToken": null
        }))
    }

    fn delete_metric_filter(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn put_subscription_filter(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({ "subscriptionArn": format!("arn:aws:logs:us-east-1:123456789012:subscription-filter:{}", uuid::Uuid::new_v4()) }))
    }

    fn describe_subscription_filters(&self, _req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(_req.account, &_req.region);
        let filters: Vec<Value> = state.subscriptions.read().clone();
        AwsResponse::json(200, json!({
            "subscriptionFilters": filters,
            "nextToken": null
        }))
    }

    fn delete_subscription_filter(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn put_query_definition(&self, _req: &AwsRequest) -> AwsResponse {
        let id = uuid::Uuid::new_v4().simple().to_string();
        AwsResponse::json(200, json!({ "queryId": id }))
    }

    fn get_query_results(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "status": "Complete",
            "results": []
        }))
    }

    fn start_query(&self, _req: &AwsRequest) -> AwsResponse {
        let id = uuid::Uuid::new_v4().simple().to_string();
        AwsResponse::json(200, json!({ "queryId": id }))
    }

    fn list_tags(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let tags = state.get_log_group(name)
            .map(|g| g.tags.read().clone())
            .unwrap_or_default();
        AwsResponse::json(200, json!({ "tags": tags }))
    }

    fn tag_log_group(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default();
        let tags = req.params.get("tags").and_then(|v| v.as_object()).cloned().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(g) = state.get_log_group(name) {
            for (k, v) in tags {
                g.tags.write().insert(k, v.as_str().unwrap_or("").to_string());
            }
        }
        AwsResponse::json(200, json!({}))
    }

    fn untag_log_group(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("logGroupName").and_then(|v| v.as_str()).unwrap_or_default();
        let keys: Vec<String> = req.params.get("logGroupNames")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(g) = state.get_log_group(name) {
            g.tags.write().retain(|k, _| !keys.contains(k));
        }
        AwsResponse::json(200, json!({}))
    }
}

impl Default for LogsHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "logs".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_create_and_describe_log_group() {
        let handler = LogsHandler::new();
        handler.handle(make_req("CreateLogGroup", json!({ "logGroupName": "/app" })));
        let resp = handler.handle(make_req("DescribeLogGroups", json!({})));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("/app"));
    }

    #[test]
    fn test_put_and_get_log_events() {
        let handler = LogsHandler::new();
        handler.handle(make_req("CreateLogGroup", json!({ "logGroupName": "/test" })));
        handler.handle(make_req("CreateLogStream", json!({
            "logGroupName": "/test",
            "logStreamName": "stream1"
        })));
        handler.handle(make_req("PutLogEvents", json!({
            "logGroupName": "/test",
            "logStreamName": "stream1",
            "logEvents": [
                {"timestamp": 1000, "message": "hello"},
                {"timestamp": 2000, "message": "world"}
            ]
        })));
        let resp = handler.handle(make_req("GetLogEvents", json!({
            "logGroupName": "/test",
            "logStreamName": "stream1"
        })));
        assert_eq!(resp.status, 200);
        let events: Value = serde_json::from_str(&resp.body).unwrap();
        assert_eq!(events["events"].as_array().unwrap().len(), 2);
    }
}
