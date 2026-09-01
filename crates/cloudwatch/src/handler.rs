//! CloudWatch operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::CloudwatchState;
use crate::protocol::{AwsRequest, AwsResponse};

pub struct CloudwatchHandler {
    state: RwLock<HashMap<(u64, String), CloudwatchState>>,
}

impl CloudwatchHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> CloudwatchState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(CloudwatchState::new).clone()
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
            "PutMetricData" => self.put_metric_data(&req),
            "GetMetricStatistics" => self.get_metric_statistics(&req),
            "GetMetricData" => self.get_metric_data(&req),
            "ListMetrics" => self.list_metrics(&req),
            "DescribeAlarms" => self.describe_alarms(&req),
            "PutMetricAlarm" => self.put_metric_alarm(&req),
            "DeleteAlarms" => self.delete_alarms(&req),
            "DeleteDashboards" => self.delete_dashboards(&req),
            "GetMetricAlarmHistory" => self.get_alarm_history(&req),
            "ListDashboards" => self.list_dashboards(&req),
            "CreateDashboard" => self.create_dashboard(&req),
            "DeleteDashboard" => self.delete_dashboard(&req),
            "GetDashboard" => self.get_dashboard(&req),
            "PutDashboard" => self.put_dashboard(&req),
            "ListTagsForResource" => self.list_tags(&req),
            "TagResource" => self.tag_resource(&req),
            "UntagResource" => self.untag_resource(&req),
            "GetMetricAlarmHistory" => self.get_alarm_history(&req),
            "DescribeAlarmsForMetric" => self.describe_alarms_for_metric(&req),
            "SetAlarmState" => self.set_alarm_state(&req),
            "ListMetrics" => self.list_metrics(&req),
            "GetMetricData" => self.get_metric_data(&req),
            "GetMetricStatistics" => self.get_metric_statistics(&req),
            "GetMetricData" => self.get_metric_data(&req),
                        "DeleteAlarmMuteRule" => self.json_stub(&req, "AlarmMuteRule"),
            "DeleteAnomalyDetector" => self.json_stub(&req, "AnomalyDetector"),
            "DeleteMetricStream" => self.json_stub(&req, "MetricStream"),
            "DescribeAlarmContributors" => self.json_stub(&req, "AlarmContributors"),
            "DescribeAlarmHistory" => self.json_stub(&req, "AlarmHistory"),
            "DescribeAnomalyDetectors" => self.json_stub(&req, "AnomalyDetectors"),
            "DisableAlarmActions" => self.set_alarm_actions(&req, false),
            "EnableAlarmActions" => self.set_alarm_actions(&req, true),
            "GetMetricWidgetImage" => self.json_stub(&req, "MetricWidgetImage"),
            "ListAlarmMuteRules" => self.json_stub_list(&req, "AlarmMuteRules"),
            "ListManagedInsightRules" => self.json_stub_list(&req, "ManagedInsightRules"),
            "ListMetricStreams" => self.json_stub_list(&req, "MetricStreams"),
            "PutAlarmMuteRule" => self.json_stub(&req, "AlarmMuteRule"),
            "PutAnomalyDetector" => self.json_stub(&req, "AnomalyDetector"),
            "PutCompositeAlarm" => self.json_stub(&req, "CompositeAlarm"),
            "PutInsightRule" => self.json_stub(&req, "InsightRule"),
            "PutManagedInsightRules" => self.json_stub(&req, "ManagedInsightRules"),
            "PutMetricStream" => self.json_stub(&req, "MetricStream"),
other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn put_metric_data(&self, req: &AwsRequest) -> AwsResponse {
        let namespace = req.params.get("Namespace")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let datapoints: Vec<Value> = req.params.get("MetricData")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut metrics = state.metrics.write();
        for dp in &datapoints {
            let metric_name = dp.get("MetricName")
                .and_then(|v| v.as_str()).unwrap_or("unknown");
            let key = format!("{}/{}", namespace, metric_name);
            metrics.entry(key).or_insert_with(Vec::new).push(dp.clone());
        }
        AwsResponse::json(200, json!({}))
    }

    fn get_metric_statistics(&self, req: &AwsRequest) -> AwsResponse {
        let namespace = req.params.get("Namespace")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let metric_name = req.params.get("MetricName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let key = format!("{}/{}", namespace, metric_name);
        let state = self.get_state(req.account, &req.region);
        let metrics = state.metrics.read();
        let datapoints: Vec<Value> = metrics.get(&key)
            .cloned()
            .unwrap_or_default();
        let stats = if datapoints.is_empty() {
            json!([])
        } else {
            let values: Vec<f64> = datapoints.iter()
                .filter_map(|dp| dp.get("Value").and_then(|v| v.as_f64()))
                .collect();
            let sum: f64 = values.iter().sum();
            let max = values.iter().cloned().fold(0.0f64, f64::max);
            let min = values.iter().cloned().fold(0.0f64, f64::min);
            json!([{
                "Timestamp": chrono::Utc::now().to_rfc3339(),
                "Average": if values.is_empty() { 0.0 } else { sum / values.len() as f64 },
                "Sum": sum,
                "Minimum": min,
                "Maximum": max,
                "SampleCount": values.len() as u64
            }])
        };
        AwsResponse::json(200, json!({
            "Label": metric_name,
            "Datapoints": stats
        }))
    }

    fn get_metric_data(&self, req: &AwsRequest) -> AwsResponse {
        let requests: Vec<Value> = req.params.get("MetricDataQueries")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let metrics = state.metrics.read();
        let results: Vec<Value> = requests.iter().map(|req| {
            let id = req.get("Id").and_then(|v| v.as_str()).unwrap_or("default");
            let ns = req.get("Namespace").and_then(|v| v.as_str()).unwrap_or("");
            let mn = req.get("MetricName").and_then(|v| v.as_str()).unwrap_or("");
            let key = format!("{}/{}", ns, mn);
            let dps: Vec<Value> = metrics.get(&key).cloned().unwrap_or_default();
            json!({
                "Id": id,
                "Label": mn,
                "Timestamps": dps.iter().filter_map(|dp| dp.get("Timestamp").cloned()).collect::<Vec<_>>(),
                "Values": dps.iter().filter_map(|dp| dp.get("Value").cloned()).collect::<Vec<_>>()
            })
        }).collect();
        AwsResponse::json(200, json!({
            "MetricDataResults": results
        }))
    }

    fn list_metrics(&self, req: &AwsRequest) -> AwsResponse {
        let namespace = req.params.get("Namespace")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let metric_name = req.params.get("MetricName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let metrics = state.metrics.read();
        let metrics: Vec<Value> = metrics.iter()
            .filter(|(k, _)| k.starts_with(&format!("{}/", namespace)))
            .filter(|(k, _)| metric_name.is_empty() || k.split('/').last() == Some(&metric_name))
            .map(|(key, dps)| {
                let parts: Vec<&str> = key.splitn(2, '/').collect();
                json!({
                    "Namespace": parts.get(0).copied().unwrap_or(""),
                    "MetricName": parts.get(1).copied().unwrap_or(""),
                    "Dimensions": []
                })
            })
            .collect();
        AwsResponse::json(200, json!({
            "Metrics": metrics,
            "NextToken": Value::Null
        }))
    }

    fn describe_alarms(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let alarms = state.alarms.read();
        let alarm_names: Vec<String> = req.params.get("AlarmNames")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let items: Vec<Value> = if alarm_names.is_empty() {
            alarms.values().cloned().collect()
        } else {
            alarm_names.iter()
                .filter_map(|n| alarms.get(n).cloned())
                .collect()
        };
        AwsResponse::json(200, json!({
            "MetricAlarms": items,
            "CompositeAlarms": [],
            "NextToken": Value::Null
        }))
    }

    fn put_metric_alarm(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("AlarmName")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let arn = format!("arn:aws:cloudwatch:{}:{}:alarm:{}", req.region, req.account, name);
        let state = self.get_state(req.account, &req.region);
        let alarm = json!({
            "AlarmName": name,
            "AlarmArn": arn,
            "AlarmDescription": req.params.get("AlarmDescription")
                .and_then(|v| v.as_str()).unwrap_or(""),
            "Namespace": req.params.get("Namespace")
                .and_then(|v| v.as_str()).unwrap_or(""),
            "MetricName": req.params.get("MetricName")
                .and_then(|v| v.as_str()).unwrap_or(""),
            "Threshold": req.params.get("Threshold")
                .and_then(|v| v.as_f64()).unwrap_or(0.0),
            "ComparisonOperator": req.params.get("ComparisonOperator")
                .and_then(|v| v.as_str()).unwrap_or("GreaterThanThreshold"),
            "EvaluationPeriods": req.params.get("EvaluationPeriods")
                .and_then(|v| v.as_u64()).unwrap_or(1),
            "Period": req.params.get("Period")
                .and_then(|v| v.as_u64()).unwrap_or(300),
            "ActionsEnabled": req.params.get("ActionsEnabled")
                .and_then(|v| v.as_bool()).unwrap_or(true),
            "AlarmActions": req.params.get("AlarmActions")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default(),
            "OKActions": req.params.get("OKActions")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default(),
            "InsufficientDataActions": req.params.get("InsufficientDataActions")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default(),
            "StateValue": "OK",
            "StateReason": "Unchecked: Initial alarm creation",
            "StateReasonData": "{}",
            "StateUpdatedTimestamp": chrono::Utc::now().to_rfc3339(),
            "TreatMissingData": req.params.get("TreatMissingData")
                .and_then(|v| v.as_str()).unwrap_or("missing"),
            "Dimensions": req.params.get("Dimensions")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default(),
        });
        state.alarms.write().insert(name.clone(), alarm.clone());
        // Store tags from the request
        if let Some(tags) = req.params.get("Tags").and_then(|v| v.as_array()) {
            let mut all_tags = state.tags.write();
            let entry = all_tags.entry(arn.clone()).or_insert_with(Vec::new);
            entry.extend(tags.iter().cloned());
        }
        AwsResponse::json(200, json!({}))
    }

    fn delete_alarms(&self, req: &AwsRequest) -> AwsResponse {
        let names: Vec<String> = req.params.get("AlarmNames")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut alarms = state.alarms.write();
        for name in &names {
            alarms.remove(name);
        }
        AwsResponse::json(200, json!({}))
    }

    fn get_alarm_history(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "AlarmHistoryItems": []
        }))
    }

    fn describe_alarms_for_metric(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "MetricAlarms": [],
            "NextToken": Value::Null
        }))
    }

    fn set_alarm_state(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("AlarmName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut alarms = state.alarms.write();
        if let Some(alarm) = alarms.get_mut(name) {
            let value = req.params.get("StateValue")
                .and_then(|v| v.as_str()).unwrap_or("OK");
            alarm["StateValue"] = json!(value);
        }
        AwsResponse::json(200, json!({}))
    }

    fn list_dashboards(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let dashboards = state.dashboards.read();
        let entries: Vec<Value> = dashboards.iter().map(|(name, _)| {
            json!({
                "DashboardName": name,
                "DashboardArn": format!("arn:aws:cloudwatch:{}:{}:dashboard:{}:{}", req.region, req.account, req.account, name),
                "LastModified": "2024-01-01T00:00:00Z"
            })
        }).collect();
        AwsResponse::json(200, json!({
            "DashboardEntries": entries
        }))
    }

    fn create_dashboard(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DashboardName")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let body = req.params.get("DashboardBody")
            .and_then(|v| v.as_str()).unwrap_or("{}").to_string();
        let state = self.get_state(req.account, &req.region);
        state.dashboards.write().insert(name.clone(), body);
        AwsResponse::json(200, json!({
            "DashboardName": name,
            "DashboardArn": format!("arn:aws:cloudwatch:{}:{}:dashboard:{}:{}", req.region, req.account, req.account, name)
        }))
    }

    fn delete_dashboard(&self, req: &AwsRequest) -> AwsResponse {
        let names: Vec<String> = req.params.get("DashboardNames")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut dashboards = state.dashboards.write();
        for name in &names {
            dashboards.remove(name);
        }
        AwsResponse::json(200, json!({}))
    }

    fn get_dashboard(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DashboardName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let dashboards = state.dashboards.read();
        match dashboards.get(name) {
            Some(body) => AwsResponse::json(200, json!({
                "DashboardName": name,
                "DashboardBody": body,
                "DashboardArn": format!("arn:aws:cloudwatch:{}:{}:dashboard:{}:{}", req.region, req.account, req.account, name)
            })),
            None => AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Dashboard {name} not found")),
        }
    }

    fn put_dashboard(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DashboardName")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let body = req.params.get("DashboardBody")
            .and_then(|v| v.as_str()).unwrap_or("{}").to_string();
        let state = self.get_state(req.account, &req.region);
        state.dashboards.write().insert(name, body);
        AwsResponse::json(200, json!({}))
    }

    fn list_tags(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("ResourceARN")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let tags = state.tags.read().get(arn).cloned().unwrap_or_default();
        AwsResponse::json(200, json!({ "Tags": tags }))
    }

    fn tag_resource(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("ResourceARN")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let tags: Vec<Value> = req.params.get("Tags")
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
        let arn = req.params.get("ResourceARN")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let keys: Vec<String> = req.params.get("TagKeys")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut all_tags = state.tags.write();
        if let Some(tags) = all_tags.get_mut(arn) {
            tags.retain(|t| {
                t.get("Key").and_then(|k| k.as_str())
                    .map(|k| !keys.contains(&k.to_string()))
                    .unwrap_or(true)
            });
        }
        AwsResponse::json(200, json!({}))
    }

    fn delete_dashboards(&self, req: &AwsRequest) -> AwsResponse {
        let names: Vec<String> = req.params.get("DashboardNames")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut dashboards = state.dashboards.write();
        for name in &names {
            dashboards.remove(name);
        }
        AwsResponse::json(200, json!({}))
    }

    fn set_alarm_actions(&self, req: &AwsRequest, enabled: bool) -> AwsResponse {
        let names: Vec<String> = req.params.get("AlarmNames")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut alarms = state.alarms.write();
        for name in &names {
            if let Some(alarm) = alarms.get_mut(name) {
                alarm["AlarmActionsEnabled"] = json!(enabled);
            }
        }
        AwsResponse::json(200, json!({}))
    }
}

impl Default for CloudwatchHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "cloudwatch".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_put_and_get_metrics() {
        let handler = CloudwatchHandler::new();
        handler.handle(make_req("PutMetricData", json!({
            "Namespace": "AWS/EC2",
            "MetricData": [
                { "MetricName": "CPUUtilization", "Value": 75.0, "Unit": "Percent" }
            ]
        })));
        let resp = handler.handle(make_req("GetMetricStatistics", json!({
            "Namespace": "AWS/EC2",
            "MetricName": "CPUUtilization",
            "StartTime": "2024-01-01T00:00:00Z",
            "EndTime": "2024-01-01T01:00:00Z",
            "Period": 300
        })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("Datapoints"));
    }

    #[test]
    fn test_dashboard_crud() {
        let handler = CloudwatchHandler::new();
        handler.handle(make_req("CreateDashboard", json!({
            "DashboardName": "my-dash",
            "DashboardBody": "{}"
        })));
        let resp = handler.handle(make_req("GetDashboard", json!({
            "DashboardName": "my-dash"
        })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("my-dash"));
    }
}
