//! Robotocore Rust - High-performance AWS API mock server
//!
//! This module provides Rust implementations of robotocore components,
//! gradually replacing Python implementations for better performance.
//!
//! Currently ported components:
//! - S3 virtual-hosted routing (`s3_routing`)
//! - AWS service detection / request routing (`router`)
//! - CORS header handling (`cors`)

pub mod cors;
pub mod router;
pub mod s3_routing;

pub use cors::{
    build_cors_headers, build_s3_cors_headers, fnmatch, method_matches, origin_matches, parse_csv,
    resolve_origin, CorsConfig, S3CorsRule,
};
pub use router::{route_to_service, AwsRequest};
pub use s3_routing::{
    get_s3_routing_config, is_s3_vhost_request, parse_s3_vhost, rewrite_vhost_to_path,
    S3RoutingConfig, S3VhostInfo, Scope,
};

pub mod core;

// PyO3 FFI bindings
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::{PyBytes, PyDict, PyTuple};

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "parse_s3_vhost", signature = (host=None))]
fn parse_s3_vhost_py(py: Python<'_>, host: Option<&str>) -> Option<Py<PyDict>> {
    let result = s3_routing::parse_s3_vhost(host);
    result.map(|info| {
        let dict = PyDict::new_bound(py);
        dict.set_item("bucket", info.bucket).unwrap();
        if let Some(region) = info.region {
            dict.set_item("region", region).unwrap();
        }
        dict.unbind()
    })
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "is_s3_vhost_request")]
fn is_s3_vhost_request_py(scope: &Bound<'_, PyAny>) -> PyResult<bool> {
    let scope_type: String = if let Ok(t) = scope.getattr("type") {
        t.extract()?
    } else {
        scope.get_item("type")?.extract()?
    };
    if scope_type != "http" {
        return Ok(false);
    }

    // Python: scope.get("headers", [])
    let headers: Vec<(Vec<u8>, Vec<u8>)> = match scope.get_item("headers") {
        Ok(h) => h.extract().unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    for (key, value) in headers {
        if key.as_slice() == b"host" {
            let host_str = String::from_utf8_lossy(&value);
            return Ok(s3_routing::parse_s3_vhost(Some(&host_str)).is_some());
        }
    }
    Ok(false)
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "rewrite_vhost_to_path")]
fn rewrite_vhost_to_path_py<'a>(scope: &'a Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
    // ASGI scopes (and the unit-test scopes) are dicts; copying the full
    // scope mirrors Python's `new_scope = dict(scope)` so the live gateway
    // keeps scheme/server/client and any other keys untouched.
    let scope = scope
        .downcast::<PyDict>()
        .map_err(|e| pyo3::exceptions::PyTypeError::new_err(e.to_string()))?;
    // Python: scope.get("headers", [])
    let headers: Vec<(Vec<u8>, Vec<u8>)> = match scope.get_item("headers") {
        Ok(Some(h)) => h.extract().unwrap_or_default(),
        _ => Vec::new(),
    };

    let mut host = String::new();
    for (key, value) in &headers {
        if key.as_slice() == b"host" {
            host = String::from_utf8_lossy(value).to_string();
            break;
        }
    }

    if host.is_empty() {
        return Ok(None);
    }

    let parsed = match s3_routing::parse_s3_vhost(Some(&host)) {
        Some(p) => p,
        None => return Ok(None),
    };

    let bucket = parsed.bucket;
    // Python: scope.get("path", "/")
    let original_path: String = match scope.get_item("path")? {
        Some(v) => v.extract()?,
        None => "/".to_string(),
    };

    let new_path = if original_path == "/" {
        format!("/{}", bucket)
    } else {
        format!("/{}{}", bucket, original_path)
    };

    let new_host = if host.len() > bucket.len() + 1 {
        host[bucket.len() + 1..].to_string()
    } else {
        host.clone()
    };

    let py = scope.py();
    let new_scope: Bound<'_, PyDict> = scope.copy()?;

    // Rebuild the header list as (bytes, bytes) tuples. A bare Vec<u8>
    // converts to a list of ints in pyo3 0.22, so wrap explicitly in PyBytes.
    let new_headers: Vec<Py<PyTuple>> = headers
        .iter()
        .map(|(k, v)| {
            let key: &[u8] = if k.as_slice() == b"host" {
                b"host"
            } else {
                k.as_slice()
            };
            let val: &[u8] = if k.as_slice() == b"host" {
                new_host.as_bytes()
            } else {
                v.as_slice()
            };
            let tup = PyTuple::new_bound(
                py,
                &[PyBytes::new_bound(py, key), PyBytes::new_bound(py, val)],
            );
            Ok::<_, PyErr>(tup.unbind())
        })
        .collect::<PyResult<Vec<Py<PyTuple>>>>()?;
    new_scope.set_item("path", &new_path)?;
    new_scope.set_item("headers", &new_headers)?;

    // Python: scope.get("query_string", b"")
    let qs: Vec<u8> = match scope.get_item("query_string")? {
        Some(v) => v.extract()?,
        None => Vec::new(),
    };
    // raw_path is bytes in the ASGI spec (Python: new_path.encode("utf-8")
    // [+ b"?" + query_string]).
    if !qs.is_empty() {
        let mut raw_path = new_path.clone().into_bytes();
        raw_path.push(b'?');
        raw_path.extend_from_slice(&qs);
        new_scope.set_item("raw_path", PyBytes::new_bound(py, &raw_path))?;
    } else {
        new_scope.set_item("raw_path", PyBytes::new_bound(py, new_path.as_bytes()))?;
    }

    Ok(Some(new_scope.as_any().clone().unbind()))
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "get_s3_routing_config")]
fn get_s3_routing_config_py(py: Python<'_>) -> Py<PyDict> {
    let config = s3_routing::get_s3_routing_config();
    let dict = PyDict::new_bound(py);
    dict.set_item("s3_hostname", config.s3_hostname).unwrap();
    dict.set_item("virtual_hosted_style", config.virtual_hosted_style)
        .unwrap();
    dict.set_item("website_hostname", config.website_hostname)
        .unwrap();
    dict.set_item("supported_patterns", config.supported_patterns)
        .unwrap();
    dict.unbind()
}

/// Route an AWS request to a service (Python binding for the Rust router).
///
/// Accepts a dict with keys: `method`, `path`, `query_string`, `headers`
/// (headers may be a list of (bytes/str, bytes/str) pairs or a dict).
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "route_to_service")]
fn route_to_service_py(req: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
    fn get_field(any: &Bound<'_, PyAny>, name: &str) -> PyResult<String> {
        let value = if let Ok(v) = any.getattr(name) {
            v
        } else {
            any.get_item(name)?
        };
        // Accept bytes or str
        if let Ok(b) = value.extract::<Vec<u8>>() {
            Ok(String::from_utf8_lossy(&b).into_owned())
        } else {
            value.extract()
        }
    }

    let method = get_field(req, "method")?;
    let path = get_field(req, "path")?;
    let query = get_field(req, "query_string")?;

    let headers_raw = if let Ok(h) = req.getattr("headers") {
        h
    } else {
        req.get_item("headers")?
    };
    let headers: Vec<(String, String)> =
        if let Ok(list) = headers_raw.extract::<Vec<(Vec<u8>, Vec<u8>)>>() {
            list.into_iter()
                .map(|(k, v)| {
                    (
                        String::from_utf8_lossy(&k).into_owned(),
                        String::from_utf8_lossy(&v).into_owned(),
                    )
                })
                .collect()
        } else if let Ok(list) = headers_raw.extract::<Vec<(String, String)>>() {
            list
        } else {
            let dict: std::collections::HashMap<String, String> = headers_raw.extract()?;
            dict.into_iter().collect()
        };

    let request = AwsRequest {
        method,
        path,
        query_string: query,
        headers,
    };
    Ok(router::route_to_service(&request))
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "cors_from_env")]
fn cors_from_env_py(py: Python<'_>) -> Py<PyDict> {
    let config = CorsConfig::from_env();
    let dict = PyDict::new_bound(py);
    dict.set_item("disable_cors_headers", config.disable_cors_headers)
        .unwrap();
    dict.set_item("disable_cors_checks", config.disable_cors_checks)
        .unwrap();
    dict.set_item("disable_custom_cors_s3", config.disable_custom_cors_s3)
        .unwrap();
    dict.set_item(
        "disable_custom_cors_apigateway",
        config.disable_custom_cors_apigateway,
    )
    .unwrap();
    dict.set_item(
        "disable_preflight_processing",
        config.disable_preflight_processing,
    )
    .unwrap();
    dict.set_item("allowed_headers", config.allowed_headers)
        .unwrap();
    dict.set_item("expose_headers", config.expose_headers)
        .unwrap();
    dict.set_item("allowed_origins", config.allowed_origins)
        .unwrap();
    dict.set_item("allowed_methods", config.allowed_methods)
        .unwrap();
    dict.unbind()
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "cors_parse_csv")]
fn cors_parse_csv_py(value: &str) -> Vec<String> {
    parse_csv(value)
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "cors_fnmatch")]
fn cors_fnmatch_py(pattern: &str, text: &str) -> bool {
    fnmatch(pattern, text)
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "cors_origin_matches")]
fn cors_origin_matches_py(origin: &str, patterns: Vec<String>) -> bool {
    origin_matches(origin, &patterns)
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "cors_method_matches")]
fn cors_method_matches_py(method: &str, allowed_methods: Vec<String>) -> bool {
    method_matches(method, &allowed_methods)
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "cors_build_cors_headers")]
fn cors_build_cors_headers_py(
    py: Python<'_>,
    config: &Bound<'_, PyAny>,
    request_origin: Option<String>,
) -> PyResult<Py<PyDict>> {
    let bool_field = |key: &str| -> PyResult<bool> {
        match config.get_item(key) {
            Ok(v) => v.extract(),
            Err(_) => Ok(false),
        }
    };
    let list_field = |key: &str| -> PyResult<Vec<String>> {
        match config.get_item(key) {
            Ok(v) => v.extract(),
            Err(_) => Ok(Vec::new()),
        }
    };

    let cors_config = CorsConfig {
        disable_cors_headers: bool_field("disable_cors_headers")?,
        disable_cors_checks: bool_field("disable_cors_checks")?,
        disable_custom_cors_s3: bool_field("disable_custom_cors_s3")?,
        disable_custom_cors_apigateway: bool_field("disable_custom_cors_apigateway")?,
        disable_preflight_processing: bool_field("disable_preflight_processing")?,
        allowed_headers: list_field("allowed_headers")?,
        expose_headers: list_field("expose_headers")?,
        allowed_origins: list_field("allowed_origins")?,
        allowed_methods: list_field("allowed_methods")?,
    };

    let headers = build_cors_headers(&cors_config, request_origin.as_deref());
    let dict = PyDict::new_bound(py);
    for (k, v) in headers {
        dict.set_item(k, v)?;
    }
    Ok(dict.unbind())
}

/// Extract a string list (or None) from a rule dict value.
#[cfg(feature = "python")]
fn rule_str_list(value: Option<&Bound<'_, PyAny>>) -> Vec<String> {
    match value {
        Some(v) => v.extract::<Vec<String>>().unwrap_or_default(),
        None => Vec::new(),
    }
}

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "cors_build_s3_cors_headers")]
fn cors_build_s3_cors_headers_py(
    py: Python<'_>,
    rules: &Bound<'_, PyAny>,
    request_origin: Option<String>,
    request_method: Option<String>,
    request_headers: Option<String>,
) -> PyResult<Py<PyDict>> {
    let mut cors_rules: Vec<S3CorsRule> = Vec::new();
    for rule in rules.iter()? {
        let rule = rule?;
        let item = |key: &str| rule.get_item(key).ok();
        cors_rules.push(S3CorsRule {
            allowed_origins: rule_str_list(item("AllowedOrigins").as_ref()),
            allowed_methods: rule_str_list(item("AllowedMethods").as_ref()),
            allowed_headers: rule_str_list(item("AllowedHeaders").as_ref()),
            expose_headers: rule_str_list(item("ExposeHeaders").as_ref()),
            max_age_seconds: match item("MaxAgeSeconds") {
                Some(v) => v
                    .extract::<i64>()
                    .ok()
                    .or_else(|| v.extract::<String>().ok().and_then(|s| s.parse().ok())),
                None => None,
            },
        });
    }

    let headers = build_s3_cors_headers(
        &cors_rules,
        request_origin.as_deref(),
        request_method.as_deref(),
        request_headers.as_deref(),
    );
    let dict = PyDict::new_bound(py);
    for (k, v) in headers {
        dict.set_item(k, v)?;
    }
    Ok(dict.unbind())
}

#[cfg(feature = "python")]
#[pymodule]
fn robotocore_rust(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_s3_vhost_py, m)?)?;
    m.add_function(wrap_pyfunction!(is_s3_vhost_request_py, m)?)?;
    m.add_function(wrap_pyfunction!(rewrite_vhost_to_path_py, m)?)?;
    m.add_function(wrap_pyfunction!(get_s3_routing_config_py, m)?)?;
    m.add_function(wrap_pyfunction!(route_to_service_py, m)?)?;
    m.add_function(wrap_pyfunction!(cors_from_env_py, m)?)?;
    m.add_function(wrap_pyfunction!(cors_parse_csv_py, m)?)?;
    m.add_function(wrap_pyfunction!(cors_fnmatch_py, m)?)?;
    m.add_function(wrap_pyfunction!(cors_origin_matches_py, m)?)?;
    m.add_function(wrap_pyfunction!(cors_method_matches_py, m)?)?;
    m.add_function(wrap_pyfunction!(cors_build_cors_headers_py, m)?)?;
    m.add_function(wrap_pyfunction!(cors_build_s3_cors_headers_py, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(k: &str, v: &str) -> (String, String) {
        (k.to_string(), v.to_string())
    }

    #[test]
    fn test_route_dynamodb_target() {
        let req = AwsRequest {
            method: "POST".to_string(),
            path: "/".to_string(),
            query_string: String::new(),
            headers: vec![hdr("x-amz-target", "DynamoDB_20120810.GetItem")],
        };
        assert_eq!(route_to_service(&req).as_deref(), Some("dynamodb"));
    }

    #[test]
    fn test_route_lambda_path() {
        let req = AwsRequest {
            method: "POST".to_string(),
            path: "/2015-03-31/functions".to_string(),
            query_string: String::new(),
            headers: vec![hdr(
                "authorization",
                "AWS4-HMAC-SHA256 Credential=AKIA/20240101/us-east-1/lambda/aws4_request",
            )],
        };
        assert_eq!(route_to_service(&req).as_deref(), Some("lambda"));
    }

    #[test]
    fn test_route_s3_host() {
        let req = AwsRequest {
            method: "GET".to_string(),
            path: "/my-bucket/key.txt".to_string(),
            query_string: String::new(),
            headers: vec![hdr("host", "my-bucket.s3.us-east-1.amazonaws.com")],
        };
        assert_eq!(route_to_service(&req).as_deref(), Some("s3"));
    }
}
