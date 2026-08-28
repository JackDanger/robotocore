//! cloudwatch request/response types (query protocol, XML).
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
    pub query: String,
}

#[derive(Debug, Clone)]
pub struct AwsResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl AwsResponse {
    pub fn xml(status: u16, root: &str, body_xml: String) -> Self {
        let full_body = format!(
            "<{root}Response xmlns=\\"https://api.aws.amazon.com/doc/2010-08-01/\\">{body_xml}</{root}Response>"
        );
        Self { status,
            headers: vec![
                ("Content-Type".to_string(), "text/xml".to_string()),
                ("x-amzn-RequestId".to_string(), uuid::Uuid::new_v4().to_string()),
                ("server".to_string(), "robotocore".to_string()),
            ],
            body: full_body,
}
}

    pub fn error(status: u16, code: &str, message: &str) -> Self {
        let body = format!(
            "<ErrorResponse><Error><Code>{}</Code><Message>{}</Message></Error></ErrorResponse>",
            code, message
        );
        Self { status,
            headers: vec![
                ("Content-Type".to_string(), "text/xml".to_string()),
                ("x-amzn-RequestId".to_string(), uuid::Uuid::new_v4().to_string()),
                ("server".to_string(), "robotocore".to_string()),
            ],
            body,
}
}
}

pub fn get_param(req: &AwsRequest, key: &str) -> Option<String> {
    if let Some(v) = req.params.get(key).and_then(|v| v.as_str()) {
        return Some(v.to_string());
}
    if !req.query.is_empty() {
        for pair in req.query.split('&') {
            if let Some((k, v)) = pair.split_once("=") {
                if k == key { return Some(v.to_string()); }
}
}
    }
    None
}
