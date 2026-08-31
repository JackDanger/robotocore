//! Kinesis operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{KinesisState, KinesisStream, Shard};
use crate::protocol::{AwsRequest, AwsResponse};

pub struct KinesisHandler {
    state: RwLock<HashMap<(u64, String), KinesisState>>,
}

impl KinesisHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> KinesisState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(KinesisState::new).clone()
    }

    fn stream_value(s: &KinesisStream) -> Value {
        json!({
            "StreamName": s.stream_name,
            "StreamARN": s.stream_arn,
            "StreamStatus": *s.stream_status.read(),
            "HasShards": true,
            "HasSubscription": false
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
            "CreateStream" => self.create_stream(&req),
            "DeleteStream" => self.delete_stream(&req),
            "DescribeStream" => self.describe_stream(&req),
            "ListStreams" => self.list_streams(&req),
            "PutRecord" => self.put_record(&req),
            "PutRecords" => self.put_records(&req),
            "GetShardIterator" => self.get_shard_iterator(&req),
            "GetRecords" => self.get_records(&req),
            "UpdateShardingConfiguration" => self.update_sharding_config(&req),
            "MergeShards" => self.merge_shards(&req),
            "SplitShard" => self.split_shard(&req),
            "IncreaseRetentionPeriod" => self.increase_retention(&req),
            "DecreaseRetentionPeriod" => self.decrease_retention(&req),
            "TagStream" => self.tag_stream(&req),
            "UntagStream" => self.untag_stream(&req),
            "ListTagsForStream" => self.list_tags_for_stream(&req),
            "AddTagsToStream" => self.add_tags(&req),
            "RemoveTagsFromStream" => self.remove_tags(&req),
                        "DescribeAccountSettings" => self.json_stub(&req, "AccountSettings"),
            "DescribeLimits" => self.json_stub(&req, "Limits"),
            "DescribeStreamSummary" => self.describe_stream_summary(&req),
            "EnableEnhancedMonitoring" => self.json_stub(&req, "EnableEnhancedMonitoring"),
            "IncreaseStreamRetentionPeriod" => self.json_stub(&req, "IncreaseStreamRetentionPeriod"),
            "ListShards" => self.json_stub_list(&req, "Shards"),
            "ListStreamConsumers" => self.json_stub_list(&req, "StreamConsumers"),
            "PutResourcePolicy" => self.json_stub(&req, "ResourcePolicy"),
            "RegisterStreamConsumer" => self.json_stub(&req, "RegisterStreamConsumer"),
            "StartStreamEncryption" => self.json_stub(&req, "StartStreamEncryption"),
            "TagResource" => self.json_stub(&req, "TagResource"),
            "UpdateAccountSettings" => self.json_stub(&req, "AccountSettings"),
            "UpdateShardCount" => self.json_stub(&req, "ShardCount"),
            "UpdateStreamMode" => self.json_stub(&req, "StreamMode"),
other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn create_stream(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("StreamName").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if name.is_empty() {
            return AwsResponse::error(400, "ValidationException", "StreamName is required");
        }
        let shard_count = req.params.get("ShardCount").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
        let state = self.get_state(req.account, &req.region);
        if state.get_stream(&name).is_some() {
            return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Stream {name} already exists."));
        }
        let stream = Arc::new(KinesisStream::new(req.account, &req.region, name, shard_count));
        state.streams.write().insert(stream.stream_name.clone(), stream);
        AwsResponse::json(200, json!({}))
    }

    fn delete_stream(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("StreamName").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if state.streams.write().remove(name).is_none() {
            return AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Stream {name} not found."));
        }
        AwsResponse::json(200, json!({}))
    }

    fn describe_stream(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("StreamName").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        match state.get_stream(name) {
            Some(stream) => {
                let shards: Vec<Value> = stream.shards.read().iter().map(|s| {
                    json!({
                        "ShardId": s.shard_id,
                        "SequenceNumberRange": {
                            "StartingSequenceNumber": s.sequence_number,
                            "EndingSequenceNumber": null
                        },
                        "ParentShardId": s.parent_id
                    })
                }).collect();
                AwsResponse::json(200, json!({
                    "StreamDescription": {
                        "StreamName": stream.stream_name,
                        "StreamARN": stream.stream_arn,
                        "StreamStatus": *stream.stream_status.read(),
                        "HasShards": true,
                        "HasSubscription": false,
                        "RetentionPeriodHours": stream.retention_period_hours,
                        "ShardLevelMetrics": *stream.shard_level_metrics.read(),
                        "StreamModeDetails": {
                            "StreamMode": stream.stream_mode
                        },
                        "Shards": shards,
                        "HasMoreShards": false,
                        "EncryptionType": "KMS",
                        "KeyId": "alias/aws/kinesis"
                    }
                }))
            }
            None => AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Stream {name} not found.")),
        }
    }

    fn list_streams(&self, _req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(_req.account, &_req.region);
        let streams: Vec<Value> = state.streams.read().values()
            .map(|s| json!({
                "StreamName": s.stream_name,
                "StreamARN": s.stream_arn,
                "StreamStatus": *s.stream_status.read()
            }))
            .collect();
        AwsResponse::json(200, json!({
            "StreamNames": streams.iter().map(|s| s["StreamName"].clone()).collect::<Vec<_>>(),
            "StreamSummaries": streams,
            "HasMoreStreams": false
        }))
    }

    fn put_record(&self, req: &AwsRequest) -> AwsResponse {
        let stream_name = req.params.get("StreamName").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        match state.get_stream(stream_name) {
            Some(stream) => {
                let shard = stream.shards.read().first().cloned().unwrap_or_default();
                AwsResponse::json(200, json!({
                    "SequenceNumber": "4959033827149996696551807410074587034031434670931271810",
                    "ShardId": shard.shard_id,
                    "EventId": uuid::Uuid::new_v4().to_string()
                }))
            }
            None => AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Stream {stream_name} not found.")),
        }
    }

    fn put_records(&self, req: &AwsRequest) -> AwsResponse {
        let stream_name = req.params.get("StreamName").and_then(|v| v.as_str()).unwrap_or_default();
        let records: Vec<Value> = req.params.get("Records")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        match state.get_stream(stream_name) {
            Some(stream) => {
                let shard = stream.shards.read().first().cloned().unwrap_or_default();
                let results: Vec<Value> = records.iter().map(|_| {
                    json!({
                        "SequenceNumber": "4959033827149996696551807410074587034031434670931271811",
                        "ShardId": shard.shard_id
                    })
                }).collect();
                AwsResponse::json(200, json!({
                    "FailedRecordCount": 0,
                    "Records": results
                }))
            }
            None => AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Stream {stream_name} not found.")),
        }
    }

    fn get_shard_iterator(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "ShardIterator": format!("iterator:{}", uuid::Uuid::new_v4().simple())
        }))
    }

    fn get_records(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({
            "Records": [],
            "NextShardIterator": format!("iterator:{}", uuid::Uuid::new_v4().simple()),
            "MillisBehindLatest": 0
        }))
    }

    fn update_sharding_config(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn merge_shards(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn split_shard(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn increase_retention(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn decrease_retention(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn tag_stream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn untag_stream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn list_tags_for_stream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({ "Tags": [] }))
    }

    fn add_tags(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn remove_tags(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({}))
    }

    fn describe_stream_summary(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("StreamName").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        match state.get_stream(name) {
            Some(stream) => {
                let shard_count = stream.shards.read().len();
                AwsResponse::json(200, json!({
                    "StreamDescriptionSummary": {
                        "StreamName": stream.stream_name,
                        "StreamARN": stream.stream_arn,
                        "StreamStatus": *stream.stream_status.read(),
                        "StreamModeDetails": {
                            "StreamMode": stream.stream_mode
                        },
                        "RetentionPeriodHours": stream.retention_period_hours,
                        "OpenShardCount": shard_count,
                        "ConsumerCount": 0,
                        "EnhancedMonitoring": [{ "ShardLevelMetrics": [] }],
                    }
                }))
            }
            None => AwsResponse::error(400, "ResourceNotFoundException",
                &format!("Stream {name} not found")),
        }
    }
}

impl Default for KinesisHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "kinesis".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_create_and_list_streams() {
        let handler = KinesisHandler::new();
        handler.handle(make_req("CreateStream", json!({
            "StreamName": "test-stream",
            "ShardCount": 2
        })));
        let resp = handler.handle(make_req("ListStreams", json!({})));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("test-stream"));
    }

    #[test]
    fn test_put_record() {
        let handler = KinesisHandler::new();
        handler.handle(make_req("CreateStream", json!({
            "StreamName": "rec-stream"
        })));
        let resp = handler.handle(make_req("PutRecord", json!({
            "StreamName": "rec-stream",
            "Data": "test",
            "PartitionKey": "key1"
        })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("SequenceNumber"));
    }
}
