//! Firehose operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::FirehoseState;
use crate::protocol::{AwsRequest, AwsResponse};

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

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let op = req.operation.as_str();
        match op {
            "CreateDeliveryStream" => self.createdeliverystream(&req),
            "DeleteDeliveryStream" => self.deletedeliverystream(&req),
            "DescribeDeliveryStream" => self.describedeliverystream(&req),
            "ListDeliveryStreams" => self.listdeliverystreams(&req),
            "ListTagsForDeliveryStream" => self.listtagsfordeliverystream(&req),
            "PutRecord" => self.putrecord(&req),
            "PutRecordBatch" => self.putrecordbatch(&req),
            "StartDeliveryStreamEncryption" => self.startdeliverystreamencryption(&req),
            "StopDeliveryStreamEncryption" => self.stopdeliverystreamencryption(&req),
            "TagDeliveryStream" => self.tagdeliverystream(&req),
            "UntagDeliveryStream" => self.untagdeliverystream(&req),
            "UpdateDestination" => self.updatedestination(&req),
            other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn createdeliverystream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletedeliverystream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describedeliverystream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listdeliverystreams(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listtagsfordeliverystream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putrecord(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putrecordbatch(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn startdeliverystreamencryption(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn stopdeliverystreamencryption(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn tagdeliverystream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn untagdeliverystream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updatedestination(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }
}

impl Default for FirehoseHandler {
    fn default() -> Self { Self::new() }
}
