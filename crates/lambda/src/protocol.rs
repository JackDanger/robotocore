//! Lambda request/response types (rest-json protocol).

use bytes::Bytes;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct AwsRequest {
    pub service: String,
    pub operation: String,
    pub account: u64,
    pub region: String,
    pub params: Value,
    pub body: Bytes,
    /// HTTP method.
    pub method: String,
    /// Request path.
    pub path: String,
    /// Query string.
    pub query_string: String,
    /// Headers (lowercase).
    pub headers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AwsResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl AwsResponse {
    pub fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("x-amz-request-id".to_string(), uuid::Uuid::new_v4().to_string()),
                ("server".to_string(), "robotocore".to_string()),
            ],
            body: serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
        }
    }

    pub fn raw(status: u16, content_type: &str, body: String) -> Self {
        Self {
            status,
            headers: vec![
                ("Content-Type".to_string(), content_type.to_string()),
                ("x-amz-request-id".to_string(), uuid::Uuid::new_v4().to_string()),
                ("server".to_string(), "robotocore".to_string()),
            ],
            body,
        }
    }

    pub fn error(status: u16, code: &str, message: &str) -> Self {
        let body = serde_json::json!({
            "Type": "User",
            "Message": message
        });
        Self {
            status,
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("x-amz-request-id".to_string(), uuid::Uuid::new_v4().to_string()),
                ("server".to_string(), "robotocore".to_string()),
            ],
            body: serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
        }
    }
}
