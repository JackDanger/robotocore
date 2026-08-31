//! Firehose operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::FirehoseState;
use crate::protocol::{AwsRequest, AwsResponse};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DeliveryStream {
    name: String,
    arn: String,
    status: String,
    failure_count: u64,
    failure_percentage: f64,
    record_count: u64,
    record_size: u64,
    destination: String,
    destination_type: String,
    created: u64,
    encryption: Option<serde_json::Value>,
    tags: Vec<serde_json::Value>,
}

impl FirehoseState {
    fn get_stream(&self, name: &str) -> Option<DeliveryStream> {
        self.resources.read().get(name)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    fn put_stream(&self, stream: DeliveryStream) {
        self.resources.write().insert(stream.name.clone(), serde_json::to_value(&stream).unwrap());
    }
    fn remove_stream(&self, name: &str) {
        self.resources.write().remove(name);
    }
}

pub struct FirehoseHandler {
    state: RwLock<HashMap<(u64, String), FirehoseState>>,
}

impl FirehoseHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> FirehoseState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(FirehoseState::new).clone()
    }

    fn stream_value(s: &DeliveryStream) -> Value {
        json!({
            "DeliveryStreamName": s.name,
            "DeliveryStreamStatus": s.status,
            "DeliveryStreamARN": s.arn,
            "FailureCount": s.failure_count,
            "FailurePercentage": s.failure_percentage,
            "RecordCount": s.record_count,
            "RecordSize": s.record_size,
            "HasEncryptionConfiguration": s.encryption.is_some(),
            "Destinations": [],
        })
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let op = req.operation.as_str();
        match op {
            "CreateDeliveryStream" => self.create_stream(&req),
            "DeleteDeliveryStream" => self.delete_stream(&req),
            "DescribeDeliveryStream" => self.describe_stream(&req),
            "ListDeliveryStreams" => self.list_streams(&req),
            "PutRecord" => self.put_record(&req),
            "PutRecordBatch" => self.put_record_batch(&req),
            "UpdateDestination" => self.update_destination(&req),
            "TagDeliveryStream" => self.tag_stream(&req),
            "UntagDeliveryStream" => self.untag_stream(&req),
            "ListTagsForDeliveryStream" => self.list_tags(&req),
            "StartDeliveryStreamEncryption" => self.start_encryption(&req),
            "StopDeliveryStreamEncryption" => self.stop_encryption(&req),
            other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn create_stream(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DeliveryStreamName")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if name.is_empty() {
            return AwsResponse::error(400, "ResourceAlreadyExistsException", "Name required");
        }
        let state = self.get_state(req.account, &req.region);
        if state.get_stream(&name).is_some() {
            return AwsResponse::error(400, "ResourceAlreadyExistsException",
                &format!("Firehose {name} already exists"));
        }
        let arn = format!("arn:aws:firehose:{}:{}:deliverystream/{}", req.region, req.account, name);
        let stream = DeliveryStream {
            name,
            arn: arn.clone(),
            status: "CREATING".to_string(),
            failure_count: 0,
            failure_percentage: 0.0,
            record_count: 0,
            record_size: 0,
            destination: req.params.get("ExtendedS3DestinationConfiguration")
                .and_then(|v| v.get("BucketARN"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            destination_type: "extended_s3".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            encryption: None,
            tags: req.params.get("Tags")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default(),
        };
        state.put_stream(stream);
        AwsResponse::json(200, json!({ "DeliveryStreamARN": arn }))
    }

    fn delete_stream(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DeliveryStreamName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if state.get_stream(name).is_none() {
            return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Firehose {name} not found"));
        }
        state.remove_stream(name);
        AwsResponse::json(200, json!({}))
    }

    fn describe_stream(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DeliveryStreamName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        match state.get_stream(name) {
            Some(s) => {
                let mut v = Self::stream_value(&s);
                v["DeliveryStreamType"] = json!("standard");
                v["DeliveryStreamStatus"] = json!("ACTIVE");
                if let Some(enc) = &s.encryption {
                    v["EncryptionConfiguration"] = enc.clone();
                }
                AwsResponse::json(200, json!({ "DeliveryStreamDescription": v }))
            }
            None => AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Firehose {name} not found")),
        }
    }

    fn list_streams(&self, req: &AwsRequest) -> AwsResponse {
        let limit = req.params.get("Limit")
            .and_then(|v| v.as_u64()).unwrap_or(24) as usize;
        let state = self.get_state(req.account, &req.region);
        let streams: Vec<String> = state.resources.read().keys().cloned().collect();
        let limited: Vec<String> = streams.iter().take(limit).cloned().collect();
        AwsResponse::json(200, json!({
            "DeliveryStreamNames": limited,
            "HasMoreDeliveryStreams": streams.len() > limit
        }))
    }

    fn put_record(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DeliveryStreamName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if state.get_stream(name).is_none() {
            return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Firehose {name} not found"));
        }
        let id = uuid::Uuid::new_v4().simple().to_string();
        AwsResponse::json(200, json!({ "RecordId": id }))
    }

    fn put_record_batch(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DeliveryStreamName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let records: Vec<Value> = req.params.get("Records")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if state.get_stream(name).is_none() {
            return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Firehose {name} not found"));
        }
        let ids: Vec<Value> = records.iter()
            .map(|_| json!(uuid::Uuid::new_v4().simple().to_string()))
            .collect();
        AwsResponse::json(200, json!({
            "FailedPutCount": 0,
            "RequestIds": ids
        }))
    }

    fn update_destination(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DeliveryStreamName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if state.get_stream(name).is_none() {
            return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Firehose {name} not found"));
        }
        AwsResponse::json(200, json!({}))
    }

    fn tag_stream(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DeliveryStreamName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let tags: Vec<Value> = req.params.get("Tags")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(mut s) = state.get_stream(name) {
            s.tags.extend(tags);
            state.put_stream(s);
        }
        AwsResponse::json(200, json!({}))
    }

    fn untag_stream(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DeliveryStreamName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let keys: Vec<String> = req.params.get("RemoveTagKeys")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(mut s) = state.get_stream(name) {
            s.tags.retain(|t| {
                t.get("Key").and_then(|k| k.as_str())
                    .map(|k| !keys.contains(&k.to_string()))
                    .unwrap_or(true)
            });
            state.put_stream(s);
        }
        AwsResponse::json(200, json!({}))
    }

    fn list_tags(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DeliveryStreamName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let tags = state.get_stream(name)
            .map(|s| s.tags)
            .unwrap_or_default();
        AwsResponse::json(200, json!({ "Tags": tags }))
    }

    fn start_encryption(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DeliveryStreamName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(mut s) = state.get_stream(name) {
            s.encryption = Some(json!({
                "KeyType": "KMS",
                "KeyARN": req.params.get("KeyARN")
                    .and_then(|v| v.as_str()).unwrap_or("")
            }));
            state.put_stream(s);
        }
        AwsResponse::json(200, json!({}))
    }

    fn stop_encryption(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("DeliveryStreamName")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(mut s) = state.get_stream(name) {
            s.encryption = None;
            state.put_stream(s);
        }
        AwsResponse::json(200, json!({}))
    }
}

impl Default for FirehoseHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "firehose".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_create_and_list_streams() {
        let handler = FirehoseHandler::new();
        handler.handle(make_req("CreateDeliveryStream", json!({
            "DeliveryStreamName": "test-stream"
        })));
        let resp = handler.handle(make_req("ListDeliveryStreams", json!({})));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("test-stream"));
    }

    #[test]
    fn test_put_record() {
        let handler = FirehoseHandler::new();
        handler.handle(make_req("CreateDeliveryStream", json!({
            "DeliveryStreamName": "rec-stream"
        })));
        let resp = handler.handle(make_req("PutRecord", json!({
            "DeliveryStreamName": "rec-stream",
            "Record": { "Data": "test" }
        })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("RecordId"));
    }
}
