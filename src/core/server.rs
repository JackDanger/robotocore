//! Axum-based HTTP server for the Robotocore Rust implementation.
//!
//! Implements catch-all routing, protocol detection, and service dispatch.

use axum::{
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use http_body_util::BodyExt;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::core::account::parse_account_from_key;
use crate::core::protocol::{
    extract_operation, parse_query_protocol, ParsedRequest, ParsedResponse,
};
use crate::core::services::sts;
use crate::router::{route_to_service, AwsRequest};
use http::header::HeaderValue;
use sqs::SqsHandler;

/// Service handler trait for AWS services.
pub trait ServiceHandler: Send + Sync {
    /// Handle a parsed request and return a response.
    fn handle_sync(
        &self,
        req: &ParsedRequest,
    ) -> Result<ParsedResponse, Box<dyn std::error::Error>>;
}

/// Synchronous wrapper for async STS handler
pub struct StsFunctionHandler;

impl ServiceHandler for StsFunctionHandler {
    fn handle_sync(
        &self,
        req: &ParsedRequest,
    ) -> Result<ParsedResponse, Box<dyn std::error::Error>> {
        // `handle_sts_request` is async but has no awaits; poll it to
        // completion on a local waker without touching the current runtime.
        use std::future::Future;
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

        fn noop_raw() -> RawWaker {
            fn clone(_: *const ()) -> RawWaker {
                RawWaker::new(null_ptr(), &VTABLE)
            }
            fn wakeup(_: *const ()) {}
            static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wakeup, wakeup, wakeup);
            RawWaker::new(null_ptr(), &VTABLE)
        }
        fn null_ptr() -> *const () {
            std::ptr::null()
        }

        let waker = unsafe { Waker::from_raw(noop_raw()) };
        let mut cx = Context::from_waker(&waker);
        let mut fut = std::pin::pin!(sts::handle_sts_request(req));
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(result) => result,
            Poll::Pending => Err("STS handler unexpectedly pending".into()),
        }
    }
}

/// Adapter that bridges the core `ParsedRequest`/`ParsedResponse` protocol to
/// the native SQS service crate.
pub struct SqsServiceHandler {
    inner: sqs::DefaultSqsHandler,
}

impl SqsServiceHandler {
    fn to_sqs_request(req: &ParsedRequest) -> sqs::protocol::AwsRequest {
        let params: serde_json::Value = serde_json::to_value(&req.params)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        sqs::protocol::AwsRequest {
            service: req.service.clone(),
            operation: req.operation.clone(),
            account: req.account,
            region: req.region.clone(),
            params,
            body: req.body.clone(),
        }
    }

    fn to_parsed_response(resp: sqs::protocol::AwsResponse) -> ParsedResponse {
        let mut headers = std::collections::HashMap::new();
        for (k, v) in resp.headers {
            headers.insert(k, v);
        }
        ParsedResponse {
            status: StatusCode::from_u16(resp.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            headers,
            body: serde_json::Value::Null,
            raw: Some(resp.body),
        }
    }
}

impl ServiceHandler for SqsServiceHandler {
    fn handle_sync(
        &self,
        req: &ParsedRequest,
    ) -> Result<ParsedResponse, Box<dyn std::error::Error>> {
        let sqs_req = Self::to_sqs_request(req);
        let resp = self.inner.handle(sqs_req);
        Ok(Self::to_parsed_response(resp))
    }
}

/// Adapter that bridges the core protocol to the native S3 service crate.
pub struct S3ServiceHandler {
    inner: s3::DefaultS3Handler,
}

impl S3ServiceHandler {
    fn to_s3_request(
        req: &ParsedRequest,
        method: &str,
        query_string: &str,
    ) -> s3::protocol::AwsRequest {
        // Extract bucket and key from the path
        let path = req
            .params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let path_parts: Vec<&str> = path.trim_start_matches('/').splitn(2, '/').collect();
        let bucket = path_parts
            .first()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());
        let key = path_parts
            .get(1)
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());

        // Parse query params
        let mut query_params = std::collections::HashMap::new();
        for pair in query_string.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                query_params.insert(
                    urlencoding::decode(k).unwrap_or_default().into_owned(),
                    urlencoding::decode(v).unwrap_or_default().into_owned(),
                );
            }
        }

        // Parse headers
        let mut headers = std::collections::HashMap::new();
        for (k, v) in req
            .params
            .get("__headers__")
            .and_then(|h| h.as_object())
            .cloned()
            .unwrap_or_default()
        {
            headers.insert(k.to_lowercase(), v.as_str().unwrap_or("").to_string());
        }

        s3::protocol::AwsRequest {
            service: req.service.clone(),
            operation: req.operation.clone(),
            account: req.account,
            region: req.region.clone(),
            bucket,
            key,
            query_params,
            headers,
            method: method.to_string(),
            body: req.body.clone(),
            params: serde_json::to_value(&req.params).unwrap_or_default(),
        }
    }

    fn to_parsed_response(resp: s3::protocol::AwsResponse) -> ParsedResponse {
        let mut headers = std::collections::HashMap::new();
        for (k, v) in resp.headers {
            headers.insert(k, v);
        }
        ParsedResponse {
            status: StatusCode::from_u16(resp.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            headers,
            body: serde_json::Value::Null,
            raw: Some(String::from_utf8_lossy(&resp.body).to_string()),
        }
    }
}

impl ServiceHandler for S3ServiceHandler {
    fn handle_sync(
        &self,
        req: &ParsedRequest,
    ) -> Result<ParsedResponse, Box<dyn std::error::Error>> {
        let s3_req = Self::to_s3_request(req, "GET", "");
        let resp = self.inner.handle(s3_req);
        Ok(Self::to_parsed_response(resp))
    }
}

/// Adapter that bridges the core protocol to the native DynamoDB service crate.
pub struct DynamoDbServiceHandler {
    inner: dynamodb::DefaultDynamoDbHandler,
}

impl DynamoDbServiceHandler {
    fn to_dynamo_req(req: &ParsedRequest) -> dynamodb::protocol::AwsRequest {
        let params = serde_json::to_value(&req.params).unwrap_or_default();
        dynamodb::protocol::AwsRequest {
            service: req.service.clone(),
            operation: req.operation.clone(),
            account: req.account,
            region: req.region.clone(),
            params,
            body: req.body.clone(),
        }
    }

    fn to_parsed_response(resp: dynamodb::protocol::AwsResponse) -> ParsedResponse {
        let mut headers = std::collections::HashMap::new();
        for (k, v) in resp.headers {
            headers.insert(k, v);
        }
        ParsedResponse {
            status: StatusCode::from_u16(resp.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            headers,
            body: serde_json::Value::Null,
            raw: Some(resp.body),
        }
    }
}

impl ServiceHandler for DynamoDbServiceHandler {
    fn handle_sync(
        &self,
        req: &ParsedRequest,
    ) -> Result<ParsedResponse, Box<dyn std::error::Error>> {
        let ddb_req = Self::to_dynamo_req(req);
        let resp = self.inner.handle(ddb_req);
        Ok(Self::to_parsed_response(resp))
    }
}

/// Registry of service handlers.
pub struct ServiceRegistry {
    handlers: HashMap<String, Arc<dyn ServiceHandler>>,
}

impl ServiceRegistry {
    /// Create a new registry with built-in services.
    pub fn new() -> Self {
        let mut handlers: HashMap<String, Arc<dyn ServiceHandler>> = HashMap::new();

        // Register STS handler
        handlers.insert(
            "sts".to_string(),
            Arc::new(StsFunctionHandler) as Arc<dyn ServiceHandler>,
        );

        // Register native SQS handler
        handlers.insert(
            "sqs".to_string(),
            Arc::new(SqsServiceHandler {
                inner: sqs::DefaultSqsHandler::new(),
            }) as Arc<dyn ServiceHandler>,
        );

        // Register native S3 handler
        handlers.insert(
            "s3".to_string(),
            Arc::new(S3ServiceHandler {
                inner: s3::DefaultS3Handler::new(),
            }) as Arc<dyn ServiceHandler>,
        );

        // Register native DynamoDB handler
        handlers.insert(
            "dynamodb".to_string(),
            Arc::new(DynamoDbServiceHandler {
                inner: dynamodb::DefaultDynamoDbHandler::new(),
            }) as Arc<dyn ServiceHandler>,
        );

        Self { handlers }
    }

    /// Get a service handler by name.
    pub fn get(&self, service: &str) -> Option<Arc<dyn ServiceHandler>> {
        self.handlers.get(service).cloned()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state shared across requests.
pub struct AppState {
    pub registry: ServiceRegistry,
}

/// Catch-all handler for all AWS API requests.
pub async fn catch_all_handler(
    State(state): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: axum::body::Body,
) -> impl IntoResponse {
    // Read the body
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid body"),
    };

    // Determine service from URI and headers
    let service = match extract_service(&method, &uri, &headers) {
        Ok(s) => s,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Could not determine service: {}", e),
            )
        }
    };

    // Determine operation
    let operation = match extract_operation(&method, &headers, &body_bytes, &service) {
        Ok(o) => o,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Could not determine operation"),
    };

    // Extract account from Authorization header or default
    let account = extract_account_from_request(&headers);
    let region = extract_region_from_request(&headers).unwrap_or_else(|| "us-east-1".to_string());

    // Parse request parameters based on protocol
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let params = if content_type.contains("json") {
        crate::core::protocol::parse_json_protocol(&body_bytes).unwrap_or_default()
    } else {
        parse_query_protocol(&body_bytes).unwrap_or_default()
    };

    // Create ParsedRequest
    let parsed_req = ParsedRequest {
        service: service.clone(),
        operation: operation.clone(),
        params,
        body: body_bytes,
        region,
        account,
    };

    // Get service handler
    let handler = match state.registry.get(&service) {
        Some(h) => h,
        None => {
            return error_response(
                StatusCode::NOT_IMPLEMENTED,
                &format!("Service {} not implemented", service),
            )
        }
    };

    // Handle request
    match handler.handle_sync(&parsed_req) {
        Ok(resp) => response_from_parsed(resp, &service, &parsed_req.operation),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

/// Health check endpoint
pub async fn health_handler() -> impl IntoResponse {
    let health = json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": 0.0,
        "services": {
            "sts": {
                "status": "running",
                "type": "native",
                "requests": 0
            }
        }
    });

    (StatusCode::OK, axum::Json(health))
}

/// Config endpoint
pub async fn config_handler() -> impl IntoResponse {
    let config = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "debug": false,
        "specifications_dir": std::env::var("ROBOTOCORE_SPECS_DIR").unwrap_or_else(|_| "/opt/homebrew/lib/python3.14/site-packages/botocore/data".to_string())
    });

    (StatusCode::OK, axum::Json(config))
}

/// Audit endpoint
pub async fn audit_handler() -> impl IntoResponse {
    let audit = json!({
        "entries": [],
        "count": 0
    });

    (StatusCode::OK, axum::Json(audit))
}

/// Build the Axum router.
pub fn build_router(registry: ServiceRegistry) -> Router {
    let state = Arc::new(AppState { registry });

    Router::new()
        .route("/_robotocore/health", get(health_handler))
        .route("/_robotocore/config", get(config_handler))
        .route("/_robotocore/audit", get(audit_handler))
        .fallback(catch_all_handler)
        .with_state(state)
}

/// Extract service name from URI, headers, or body.
///
/// Uses the full AWS service router first, falling back to the legacy
/// X-Amz-Target / path heuristics only when the router cannot decide.
fn extract_service(method: &Method, uri: &Uri, headers: &HeaderMap) -> Result<String, String> {
    // Build a router request from method, URI, and headers
    let mut header_pairs: Vec<(String, String)> = headers
        .iter()
        .filter_map(|(k, v)| {
            let v = v.to_str().ok()?;
            Some((k.to_string(), v.to_string()))
        })
        .collect();
    // The full router checks the Host header for virtual-hosted services
    // (sqs.{region}.localhost.robotocore.cloud, S3 virtual hosts, etc.).
    // Axum strips Host from HeaderMap, so recover it from the authority.
    if !header_pairs
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("host"))
    {
        if let Some(authority) = uri.authority() {
            header_pairs.push(("host".to_string(), authority.as_str().to_string()));
        }
    }
    let router_req = AwsRequest {
        method: method.as_str().to_string(),
        path: uri.path().to_string(),
        query_string: uri.query().unwrap_or("").to_string(),
        headers: header_pairs,
    };
    if let Some(service) = route_to_service(&router_req) {
        return Ok(service);
    }

    // Fallback: X-Amz-Target header (contains "service.Operation")
    if let Some(target) = headers.get("X-Amz-Target") {
        if let Ok(target_str) = target.to_str() {
            if let Some(service) = target_str.split('.').next() {
                return Ok(service.to_lowercase());
            }
        }
    }

    // Fallback: path-based detection (e.g., /bucket/key for S3)
    let path = uri.path();
    if path.starts_with("/") {
        // Default to sts for root path
        if path == "/" {
            return Ok("sts".to_string());
        }
    }

    Err("Could not determine service".to_string())
}

/// Extract account ID from Authorization header.
fn extract_account_from_request(headers: &HeaderMap) -> u64 {
    // Try to get account from Authorization header
    if let Some(auth) = headers.get("Authorization") {
        if let Ok(auth_str) = auth.to_str() {
            // Look for Credential=AKIAIOSFODNN7EXAMPLE/...
            if let Some(cred_part) = auth_str.split("Credential=").nth(1) {
                if let Some(access_key) = cred_part.split('/').next() {
                    return parse_account_from_key(access_key);
                }
            }
        }
    }

    // Try custom headers
    if let Some(account_header) = headers.get("X-Robotocore-Account") {
        if let Ok(account_str) = account_header.to_str() {
            if let Ok(account) = account_str.parse::<u64>() {
                return account;
            }
        }
    }

    123456789012
}

/// Extract region from headers.
fn extract_region_from_request(headers: &HeaderMap) -> Option<String> {
    if let Some(region) = headers.get("X-Robotocore-Region") {
        if let Ok(region_str) = region.to_str() {
            return Some(region_str.to_string());
        }
    }

    // Try Authorization header
    if let Some(auth) = headers.get("Authorization") {
        if let Ok(auth_str) = auth.to_str() {
            // Look for credential format: AKIA.../20230101/us-east-1/...
            if let Some(cred_part) = auth_str.split("Credential=").nth(1) {
                let parts: Vec<&str> = cred_part.split('/').collect();
                if parts.len() >= 3 {
                    return Some(parts[2].to_string());
                }
            }
        }
    }

    None
}

/// Convert ParsedResponse to Axum response with appropriate protocol encoding.
fn response_from_parsed(resp: ParsedResponse, service: &str, operation: &str) -> Response {
    use crate::core::protocol::serialize_query_response;

    let request_id = Uuid::new_v4().to_string();

    let body_str = if let Some(raw) = resp.raw {
        // Pre-serialized body: send verbatim (e.g. native JSON-protocol services).
        raw
    } else if service == "sts" {
        serialize_query_response(&resp.body, operation, &request_id)
    } else {
        serde_json::to_string(&resp.body).unwrap_or_else(|_| "{}".to_string())
    };

    let mut headers_map = axum::http::HeaderMap::new();
    for (key, value) in resp.headers {
        if let Ok(header_value) = HeaderValue::from_str(&value) {
            if let Ok(header_name) = axum::http::HeaderName::from_bytes(key.as_bytes()) {
                headers_map.insert(header_name, header_value);
            }
        }
    }

    (resp.status, headers_map, body_str).into_response()
}

/// Build an error response.
fn error_response(status: StatusCode, message: &str) -> Response {
    let error = json!({
        "error": message
    });

    (status, axum::Json(error)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_service_from_target_header() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Amz-Target", "STS.GetCallerIdentity".parse().unwrap());

        let uri = "/".parse().unwrap();
        let method = Method::POST;
        let service = extract_service(&method, &uri, &headers).unwrap();
        assert_eq!(service, "sts");
    }

    #[test]
    fn test_extract_account_from_auth_header() {
        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "AWS4-HMAC-SHA256 Credential=123456789012/20230101/us-east-1/sts/aws4_request, SignedHeaders=host;x-amz-date, Signature=xyz".parse().unwrap());

        let account = extract_account_from_request(&headers);
        assert_eq!(account, 123456789012);
    }

    #[test]
    fn test_registry_creation() {
        let registry = ServiceRegistry::new();
        assert!(registry.get("sts").is_some());
        assert!(registry.get("sqs").is_some());
    }

    #[test]
    fn test_extract_service_sqs_via_target_header() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Amz-Target", "AmazonSQS.SendMessage".parse().unwrap());
        let uri = "/".parse().unwrap();
        let method = Method::POST;
        let service = extract_service(&method, &uri, &headers).unwrap();
        assert_eq!(service, "sqs");
    }

    #[test]
    fn test_extract_service_sqs_via_auth_scope() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            "AWS4-HMAC-SHA256 Credential=123456789012/20260101/us-east-1/sqs/aws4_request, SignedHeaders=host, Signature=abc".parse().unwrap(),
        );
        let uri = "/".parse().unwrap();
        let method = Method::POST;
        let service = extract_service(&method, &uri, &headers).unwrap();
        assert_eq!(service, "sqs");
    }

    #[test]
    fn test_extract_service_sqs_via_host_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Host",
            "sqs.us-east-1.localhost.robotocore.cloud:4566"
                .parse()
                .unwrap(),
        );
        let uri = "/123456789012/my-queue".parse().unwrap();
        let method = Method::POST;
        let service = extract_service(&method, &uri, &headers).unwrap();
        assert_eq!(service, "sqs");
    }
}
