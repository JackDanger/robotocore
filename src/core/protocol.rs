//! AWS wire protocol parsing and serialization.
//!
//! Handles parsing HTTP requests into ParsedRequest structures,
//! and serializing ParsedResponse back to HTTP responses.
//! Supports query, json, ec2, rest-json, rest-xml protocols.

use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::str;

/// Parsed AWS request extracted from HTTP.
#[derive(Debug, Clone)]
pub struct ParsedRequest {
    pub service: String,
    pub operation: String,
    pub params: HashMap<String, Value>,
    pub body: Bytes,
    pub region: String,
    pub account: u64,
}

/// Parsed AWS response to be serialized to HTTP.
#[derive(Debug, Clone)]
pub struct ParsedResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Value,
    /// Optional pre-serialized body. When set, it is sent verbatim,
    /// bypassing query-XML/JSON encoding (used by JSON-protocol services).
    pub raw: Option<String>,
}

impl ParsedResponse {
    /// Create a successful response with JSON body.
    pub fn json_success(body: Value) -> Self {
        Self {
            status: StatusCode::OK,
            headers: {
                let mut h = HashMap::new();
                h.insert(
                    "Content-Type".to_string(),
                    "application/x-amz-json-1.1".to_string(),
                );
                h
            },
            body,
            raw: None,
        }
    }

    /// Create a response with a pre-serialized raw body (e.g. JSON-protocol
    /// service responses that control their own serialization and content type).
    pub fn raw(status: StatusCode, headers: HashMap<String, String>, body: String) -> Self {
        Self {
            status,
            headers,
            body: Value::Null,
            raw: Some(body),
        }
    }

    /// Create an error response.
    pub fn error(status: StatusCode, code: String, message: String, protocol: &str) -> Self {
        let body = match protocol {
            "json" | "rest-json" => json!({
                "__type": code,
                "message": message
            }),
            "query" | "ec2" => {
                // XML format will be handled by the server
                json!({
                    "error_code": code,
                    "error_message": message
                })
            }
            _ => json!({
                "message": message
            }),
        };

        Self {
            status,
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            body,
            raw: None,
        }
    }
}

/// Parse a query-protocol request (form-encoded parameters).
pub fn parse_query_protocol(
    body: &[u8],
) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
    let body_str = str::from_utf8(body)?;
    let mut params = HashMap::new();

    for pair in body_str.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = urlencoding::decode(key)?.into_owned();
            let value = urlencoding::decode(value)?.into_owned();
            params.insert(key, Value::String(value));
        }
    }

    Ok(params)
}

/// Parse a JSON-protocol request.
pub fn parse_json_protocol(
    body: &[u8],
) -> Result<HashMap<String, Value>, Box<dyn std::error::Error>> {
    if body.is_empty() {
        return Ok(HashMap::new());
    }

    let json: Value = serde_json::from_slice(body)?;
    let mut params = HashMap::new();

    if let Some(obj) = json.as_object() {
        for (key, value) in obj {
            params.insert(key.clone(), value.clone());
        }
    }

    Ok(params)
}

/// Serialize a response body to JSON.
pub fn serialize_json_response(value: &Value, _operation: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

/// Serialize a response body to XML (query protocol).
pub fn serialize_query_response(value: &Value, operation: &str, request_id: &str) -> String {
    // Build query protocol XML response
    let result_wrapper = format!("{}Result", operation);
    let mut xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<{operation}Response xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
    <{result_wrapper}>"#,
        operation = operation,
        result_wrapper = result_wrapper
    );

    // Serialize value as XML elements
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            xml.push_str(&format!("        <{}>", key));
            xml.push_str(&value_to_xml_content(val));
            xml.push_str(&format!("</{}>\n", key));
        }
    }

    xml.push_str(&format!(
        "    </{}>
",
        result_wrapper
    ));
    xml.push_str(&format!(
        "    <ResponseMetadata><RequestId>{}</RequestId></ResponseMetadata>
",
        request_id
    ));
    xml.push_str(&format!("</{operation}Response>", operation = operation));

    xml
}

/// Serialize an error response to XML (query protocol).
pub fn serialize_query_error(code: &str, message: &str, request_id: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ErrorResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
    <Error>
        <Type>Sender</Type>
        <Code>{code}</Code>
        <Message>{message}</Message>
    </Error>
    <RequestId>{request_id}</RequestId>
</ErrorResponse>"#,
        code = code,
        message = message,
        request_id = request_id
    )
}

fn value_to_xml_content(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        Value::Array(arr) => arr
            .iter()
            .map(value_to_xml_content)
            .collect::<Vec<_>>()
            .join(""),
        Value::Object(obj) => obj
            .iter()
            .map(|(k, v)| {
                format!(
                    "<{key}>{val}</{key}>",
                    key = k,
                    val = value_to_xml_content(v)
                )
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}

/// Determine operation name from headers and body.
pub fn extract_operation(
    _method: &Method,
    headers: &HeaderMap,
    body: &[u8],
    _service: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Try X-Amz-Target header (JSON/EC2 protocols)
    if let Some(target) = headers.get("X-Amz-Target") {
        let target_str = target.to_str()?;
        if let Some(op_part) = target_str.rsplit('.').next() {
            return Ok(op_part.to_string());
        }
    }

    // Try form-encoded Action parameter (query/EC2 protocols)
    let body_str = str::from_utf8(body).unwrap_or("");
    if let Some(start) = body_str.find("Action=") {
        let after_action = &body_str[start + 7..];
        if let Some(end) = after_action.find('&') {
            return Ok(after_action[..end].to_string());
        } else {
            return Ok(after_action.to_string());
        }
    }

    // Try querystring Action
    // (would need query params parsing)

    Err("Could not determine operation".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_protocol() {
        let body = b"Action=GetCallerIdentity&Version=2011-06-15";
        let result = parse_query_protocol(body).unwrap();
        assert_eq!(
            result.get("Action").and_then(|v| v.as_str()),
            Some("GetCallerIdentity")
        );
        assert_eq!(
            result.get("Version").and_then(|v| v.as_str()),
            Some("2011-06-15")
        );
    }

    #[test]
    fn test_parse_json_protocol() {
        let body = br#"{"foo": "bar", "num": 42}"#;
        let result = parse_json_protocol(body).unwrap();
        assert_eq!(result.get("foo").and_then(|v| v.as_str()), Some("bar"));
    }

    #[test]
    fn test_serialize_query_response() {
        let value = json!({
            "UserId": "AKIAIOSFODNN7EXAMPLE",
            "Account": "123456789012",
            "Arn": "arn:aws:iam::123456789012:root"
        });
        let xml = serialize_query_response(&value, "GetCallerIdentity", "req-123");
        assert!(xml.contains("UserId"));
        assert!(xml.contains("123456789012"));
    }
}
