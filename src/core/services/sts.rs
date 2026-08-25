//! AWS STS (Security Token Service) handler.
//!
//! Implements GetCallerIdentity, GetAccessKeyInfo, and other STS operations.

use crate::core::account::parse_account_from_key;
use crate::core::protocol::{ParsedRequest, ParsedResponse};
use http::StatusCode;
use serde_json::json;

/// Handle STS requests.
pub async fn handle_sts_request(
    req: &ParsedRequest,
) -> Result<ParsedResponse, Box<dyn std::error::Error>> {
    match req.operation.as_str() {
        "GetCallerIdentity" => handle_get_caller_identity(req),
        "GetAccessKeyInfo" => handle_get_access_key_info(req),
        _ => Err(format!("Unknown STS operation: {}", req.operation).into()),
    }
}

/// GetCallerIdentity operation.
///
/// Returns UserId, Account, and Arn based on the access key ID.
fn handle_get_caller_identity(
    req: &ParsedRequest,
) -> Result<ParsedResponse, Box<dyn std::error::Error>> {
    // Account is already parsed and available
    let account = req.account;

    // UserId from access key - format as AKIA... or similar
    let user_id = "AKIAIOSFODNN7EXAMPLE";

    // Build ARN
    let arn = format!("arn:aws:sts::{}:user/moto", account);

    let body = json!({
        "UserId": user_id,
        "Account": account.to_string(),
        "Arn": arn
    });

    Ok(ParsedResponse {
        status: StatusCode::OK,
        headers: {
            let mut h = std::collections::HashMap::new();
            h.insert("Content-Type".to_string(), "text/xml".to_string());
            h
        },
        body,
        raw: None,
    })
}

/// GetAccessKeyInfo operation.
///
/// Returns the account ID for the given access key ID.
fn handle_get_access_key_info(
    req: &ParsedRequest,
) -> Result<ParsedResponse, Box<dyn std::error::Error>> {
    // Extract AccessKeyId from params
    let access_key_id = req
        .params
        .get("AccessKeyId")
        .and_then(|v| v.as_str())
        .ok_or("Missing AccessKeyId parameter")?;

    // Determine account from access key
    let account = parse_account_from_key(access_key_id);

    let body = json!({
        "Account": account.to_string()
    });

    Ok(ParsedResponse {
        status: StatusCode::OK,
        headers: {
            let mut h = std::collections::HashMap::new();
            h.insert("Content-Type".to_string(), "text/xml".to_string());
            h
        },
        body,
        raw: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::HashMap;

    #[test]
    fn test_get_caller_identity() {
        let mut params = HashMap::new();
        let req = ParsedRequest {
            service: "sts".to_string(),
            operation: "GetCallerIdentity".to_string(),
            params,
            body: bytes::Bytes::new(),
            region: "us-east-1".to_string(),
            account: 123456789012,
        };

        let resp = handle_get_caller_identity(&req).unwrap();
        assert_eq!(resp.status, StatusCode::OK);

        if let Value::Object(obj) = &resp.body {
            assert!(obj.contains_key("UserId"));
            assert!(obj.contains_key("Account"));
            assert!(obj.contains_key("Arn"));
        } else {
            panic!("Expected JSON object response");
        }
    }

    #[test]
    fn test_get_access_key_info() {
        let mut params = HashMap::new();
        params.insert(
            "AccessKeyId".to_string(),
            Value::String("123456789012".to_string()),
        );

        let req = ParsedRequest {
            service: "sts".to_string(),
            operation: "GetAccessKeyInfo".to_string(),
            params,
            body: bytes::Bytes::new(),
            region: "us-east-1".to_string(),
            account: 123456789012,
        };

        let resp = handle_get_access_key_info(&req).unwrap();
        assert_eq!(resp.status, StatusCode::OK);
    }
}
