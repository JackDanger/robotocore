//! S3 virtual-hosted-style routing for robotocore.
//!
//! Parses Host headers to detect S3 virtual-hosted-style requests and rewrites
//! them to path-style for downstream handling.

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use regex::Regex;
use std::env;
use std::sync::Arc;

/// Default hostname base for S3 virtual-hosted-style requests
const DEFAULT_S3_HOSTNAME: &str = "s3.localhost.robotocore.cloud";

/// Backwards-compatible alias for localstack.cloud hostnames
#[allow(dead_code)]
const S3_LOCALSTACK_HOSTNAME: &str = "s3.localhost.localstack.cloud";

/// Pre-compiled pattern for standard AWS S3 virtual-hosted requests
/// Matches: mybucket.s3[.region].amazonaws.com
static VHOST_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<bucket>[a-zA-Z0-9][a-zA-Z0-9.\-]{1,61}[a-zA-Z0-9])\.s3(?:\.(?P<region>[a-z]{2}-[a-z]+-\d+))?\.(?P<rest>.+?)(?::\d+)?$"
    ).unwrap()
});

/// Pre-compiled pattern for localstack.cloud backwards-compatible alias
static VHOST_LOCALSTACK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<bucket>[a-zA-Z0-9][a-zA-Z0-9.\-]{1,61}[a-zA-Z0-9])\.s3\.localhost\.localstack\.cloud(?::\d+)?$"
    ).unwrap()
});

/// Custom hostname pattern cache
struct CustomPatternCache {
    pattern: Regex,
    hostname: String,
}

static CUSTOM_PATTERN_CACHE: Lazy<Arc<RwLock<Option<CustomPatternCache>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Return the configured S3 hostname base from environment or default
fn get_s3_hostname() -> String {
    env::var("S3_HOSTNAME").unwrap_or_else(|_| DEFAULT_S3_HOSTNAME.to_string())
}

/// Get or build the custom hostname pattern, caching it
fn get_custom_pattern() -> (Regex, String) {
    let hostname = get_s3_hostname();
    
    {
        let read_guard = CUSTOM_PATTERN_CACHE.read();
        if let Some(cache) = read_guard.as_ref() {
            if cache.hostname == hostname {
                return (cache.pattern.clone(), cache.hostname.clone());
            }
        }
    }

    // Build new pattern
    let escaped = regex::escape(&hostname);
    let pattern = Regex::new(&format!(
        r"^(?P<bucket>[a-zA-Z0-9][a-zA-Z0-9.\-{{1,61}}][a-zA-Z0-9])\.{}(?::\d+)?$",
        escaped
    )).unwrap();

    // Update cache
    {
        let mut write_guard = CUSTOM_PATTERN_CACHE.write();
        *write_guard = Some(CustomPatternCache {
            pattern: pattern.clone(),
            hostname: hostname.clone(),
        });
    }

    (pattern, hostname)
}

/// S3 routing result
#[derive(Debug, Clone, PartialEq)]
pub struct S3VhostInfo {
    pub bucket: String,
    pub region: Option<String>,
}

/// Parse an S3 virtual-hosted-style Host header.
///
/// Returns `Some(S3VhostInfo)` if the host matches an S3 pattern,
/// or `None` if it does not.
pub fn parse_s3_vhost(host: Option<&str>) -> Option<S3VhostInfo> {
    let host = host?;
    if host.is_empty() {
        return None;
    }

    // Strip port if present for matching
    let host_no_port = if let Some(idx) = host.rfind(':') {
        // Check if it's actually a port (all digits after last colon)
        let after_colon = &host[idx + 1..];
        if after_colon.chars().all(|c| c.is_ascii_digit()) {
            &host[..idx]
        } else {
            host
        }
    } else {
        host
    };

    // Check custom hostname pattern first (most specific)
    let (custom_re, _) = get_custom_pattern();
    if let Some(m) = custom_re.captures(host) {
        return Some(S3VhostInfo {
            bucket: m.name("bucket")?.as_str().to_string(),
            region: None,
        });
    }

    // Check localstack.cloud backwards-compatible alias
    if let Some(m) = VHOST_LOCALSTACK_RE.captures(host) {
        return Some(S3VhostInfo {
            bucket: m.name("bucket")?.as_str().to_string(),
            region: None,
        });
    }

    // Check standard AWS patterns
    if let Some(m) = VHOST_RE.captures(host) {
        let mut result = S3VhostInfo {
            bucket: m.name("bucket")?.as_str().to_string(),
            region: None,
        };

        if let Some(region) = m.name("region") {
            result.region = Some(region.as_str().to_string());
        } else {
            // Try to extract region from the rest part
            let rest = m.name("rest")?.as_str();
            if let Some(region_match) = Regex::new(r"(?:^|\.)((?:us|eu|ap|sa|ca|me|af|il)-[a-z]+-\d+)")
                .unwrap()
                .captures(rest)
            {
                result.region = Some(region_match.get(1)?.as_str().to_string());
            }
        }
        return Some(result);
    }

    // Check bare s3 pattern: <bucket>.s3.<anything>
    if host_no_port.contains(".s3.") {
        let parts: Vec<&str> = host_no_port.splitn(2, ".s3.").collect();
        if !parts[0].is_empty() && !parts[0].starts_with('.') {
            let bucket = parts[0].to_string();
            let remainder = parts[1];
            
            let mut result = S3VhostInfo {
                bucket,
                region: None,
            };
            
            if let Some(region_match) = Regex::new(r"(?:^|\.)(us|eu|ap|sa|ca|me|af|il)(-[a-z]+-\d+)")
                .unwrap()
                .captures(remainder)
            {
                result.region = Some(
                    format!("{}{}", region_match.get(1)?.as_str(), region_match.get(2)?.as_str())
                );
            }
            return Some(result);
        }
    }

    // S3 Express directory buckets: <bucket>.localhost[:port]
    // S3 Object Lambda: <route-token>.localhost[:port]
    if host_no_port.ends_with(".localhost") || host_no_port.contains(".localhost:") {
        let label = if let Some(idx) = host_no_port.find(".localhost") {
            &host_no_port[..idx]
        } else {
            return None;
        };
        
        if !label.is_empty() && !label.contains('.') {
            return Some(S3VhostInfo {
                bucket: label.to_string(),
                region: None,
            });
        }
    }

    None
}

/// Check if an ASGI scope represents an S3 virtual-hosted-style request.
///
/// This is a simplified version that works with a mock scope structure.
pub fn is_s3_vhost_request(scope: &Scope) -> bool {
    if scope.r#type != "http" {
        return false;
    }
    
    for header in &scope.headers {
        if header.0 == "host" {
            return parse_s3_vhost(Some(&header.1)).is_some();
        }
    }
    false
}

/// ASGI scope mock structure for testing
#[derive(Debug, Clone)]
pub struct Scope {
    pub r#type: String,
    pub method: Option<String>,
    pub path: String,
    pub query_string: String,
    pub headers: Vec<(String, String)>,
}

/// Rewrite a virtual-hosted-style S3 request scope to path-style.
///
/// Returns a new Scope with the path rewritten to include the bucket,
/// or `None` if the Host header does not match.
pub fn rewrite_vhost_to_path(scope: &Scope) -> Option<Scope> {
    let mut new_scope = scope.clone();
    
    let mut host = String::new();
    for header in &scope.headers {
        if header.0 == "host" {
            host = header.1.clone();
            break;
        }
    }
    
    if host.is_empty() {
        return None;
    }

    let parsed = parse_s3_vhost(Some(&host))?;
    let bucket = parsed.bucket;
    let original_path = &scope.path;

    // Rewrite path: / -> /bucket, /key -> /bucket/key
    let new_path = if original_path == "/" {
        format!("/{}", bucket)
    } else {
        format!("/{}{}", bucket, original_path)
    };

    // Strip the bucket prefix from the Host header
    let new_host = if host.len() > bucket.len() + 1 {
        host[bucket.len() + 1..].to_string()
    } else {
        host.clone()
    };

    // Update headers
    new_scope.headers = scope
        .headers
        .iter()
        .map(|h| {
            if h.0 == "host" {
                ("host".to_string(), new_host.clone())
            } else {
                h.clone()
            }
        })
        .collect();

    new_scope.path = new_path;

    // Set raw_path
    if !scope.query_string.is_empty() {
        new_scope.query_string = scope.query_string.clone();
    }

    Some(new_scope)
}

/// S3 routing configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct S3RoutingConfig {
    pub s3_hostname: String,
    pub virtual_hosted_style: bool,
    pub website_hostname: String,
    pub supported_patterns: Vec<String>,
}

/// Return the current S3 routing configuration
pub fn get_s3_routing_config() -> S3RoutingConfig {
    let hostname = get_s3_hostname();
    S3RoutingConfig {
        s3_hostname: hostname.clone(),
        virtual_hosted_style: true,
        website_hostname: format!("s3-website.{}", hostname),
        supported_patterns: vec![
            "<bucket>.s3.<hostname>".to_string(),
            "<bucket>.s3.<region>.amazonaws.com".to_string(),
            "<bucket>.s3.amazonaws.com".to_string(),
        ],
    }
}

#[cfg(test)]

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_cache() {
        let mut cache = CUSTOM_PATTERN_CACHE.write();
        *cache = None;
    }

    #[test]
    fn test_default_hostname() {
        reset_cache();
        let result = parse_s3_vhost(Some("mybucket.s3.localhost.robotocore.cloud"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().bucket, "mybucket");
    }

    #[test]
    fn test_localstack_hostname_alias() {
        reset_cache();
        let result = parse_s3_vhost(Some("mybucket.s3.localhost.localstack.cloud"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().bucket, "mybucket");
    }

    #[test]
    fn test_aws_region_hostname() {
        reset_cache();
        let result = parse_s3_vhost(Some("mybucket.s3.us-east-1.amazonaws.com"));
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.bucket, "mybucket");
        assert_eq!(info.region, Some("us-east-1".to_string()));
    }

    #[test]
    fn test_aws_global_hostname() {
        reset_cache();
        let result = parse_s3_vhost(Some("mybucket.s3.amazonaws.com"));
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.bucket, "mybucket");
        assert_eq!(info.region, None);
    }

    #[test]
    fn test_non_s3_hostname_returns_none() {
        reset_cache();
        assert!(parse_s3_vhost(Some("example.com")).is_none());
        assert!(parse_s3_vhost(Some("api.example.com")).is_none());
        assert!(parse_s3_vhost(Some("localhost")).is_none());
    }

    #[test]
    fn test_empty_host_returns_none() {
        reset_cache();
        assert!(parse_s3_vhost(Some("")).is_none());
    }

    #[test]
    fn test_none_host_returns_none() {
        reset_cache();
        assert!(parse_s3_vhost(None).is_none());
    }

    #[test]
    fn test_host_with_port() {
        reset_cache();
        let result = parse_s3_vhost(Some("mybucket.s3.localhost.robotocore.cloud:4566"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().bucket, "mybucket");
    }

    #[test]
    fn test_s3_express_localhost_host() {
        reset_cache();
        let result = parse_s3_vhost(Some("mybucket--use1-az1--x-s3.localhost:4566"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().bucket, "mybucket--use1-az1--x-s3");
    }

    #[test]
    fn test_s3_object_lambda_route_token() {
        reset_cache();
        let result = parse_s3_vhost(Some("my-route-token.localhost:4566"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().bucket, "my-route-token");
    }

    #[test]
    fn test_eu_west_region() {
        reset_cache();
        let result = parse_s3_vhost(Some("mybucket.s3.eu-west-1.amazonaws.com"));
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.bucket, "mybucket");
        assert_eq!(info.region, Some("eu-west-1".to_string()));
    }

    #[test]
    fn test_dualstack_hostname() {
        reset_cache();
        let result = parse_s3_vhost(Some("mybucket.s3.dualstack.us-east-1.amazonaws.com"));
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.bucket, "mybucket");
        assert_eq!(info.region, Some("us-east-1".to_string()));
    }

    #[test]
    fn test_bucket_with_dots() {
        reset_cache();
        let result = parse_s3_vhost(Some("my.bucket.name.s3.us-east-1.amazonaws.com"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().bucket, "my.bucket.name");
    }

    #[test]
    fn test_bucket_with_hyphens() {
        reset_cache();
        let result = parse_s3_vhost(Some("my-test-bucket.s3.localhost.robotocore.cloud"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().bucket, "my-test-bucket");
    }

    #[test]
    fn test_custom_s3_hostname_env() {
        reset_cache();
        env::set_var("S3_HOSTNAME", "s3.custom.local");
        let result = parse_s3_vhost(Some("testbucket.s3.custom.local"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().bucket, "testbucket");
        env::remove_var("S3_HOSTNAME");
        reset_cache();
    }

    #[test]
    fn test_aws_regional_hostname() {
        reset_cache();
        let result = parse_s3_vhost(Some("mybucket.s3.us-west-2.amazonaws.com"));
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.bucket, "mybucket");
        assert_eq!(info.region, Some("us-west-2".to_string()));
    }

    #[test]
    fn test_sa_region() {
        reset_cache();
        let result = parse_s3_vhost(Some("mybucket.s3.sa-east-1.amazonaws.com"));
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.bucket, "mybucket");
        assert_eq!(info.region, Some("sa-east-1".to_string()));
    }

    #[test]
    fn test_ap_region() {
        reset_cache();
        let result = parse_s3_vhost(Some("data-bucket.s3.ap-southeast-2.amazonaws.com"));
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.bucket, "data-bucket");
        assert_eq!(info.region, Some("ap-southeast-2".to_string()));
    }

    #[test]
    fn test_all_numeric_bucket() {
        reset_cache();
        let result = parse_s3_vhost(Some("123456789.s3.localhost.robotocore.cloud"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().bucket, "123456789");
    }

    #[test]
    fn test_mixed_case_bucket() {
        reset_cache();
        let result = parse_s3_vhost(Some("MyBucket.s3.localhost.robotocore.cloud"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().bucket, "MyBucket");
    }

    #[test]
    fn test_rewrite_root_path() {
        reset_cache();
        let scope = Scope {
            r#type: "http".to_string(),
            method: Some("GET".to_string()),
            path: "/".to_string(),
            query_string: "".to_string(),
            headers: vec![
                ("host".to_string(), "mybucket.s3.localhost.robotocore.cloud".to_string()),
                ("content-type".to_string(), "text/plain".to_string()),
            ],
        };
        let result = rewrite_vhost_to_path(&scope);
        assert!(result.is_some());
        assert_eq!(result.unwrap().path, "/mybucket");
    }

    #[test]
    fn test_rewrite_key_path() {
        reset_cache();
        let scope = Scope {
            r#type: "http".to_string(),
            method: Some("GET".to_string()),
            path: "/key.txt".to_string(),
            query_string: "".to_string(),
            headers: vec![
                ("host".to_string(), "mybucket.s3.localhost.robotocore.cloud".to_string()),
                ("content-type".to_string(), "text/plain".to_string()),
            ],
        };
        let result = rewrite_vhost_to_path(&scope);
        assert!(result.is_some());
        assert_eq!(result.unwrap().path, "/mybucket/key.txt");
    }

    #[test]
    fn test_preserves_query_string() {
        reset_cache();
        let scope = Scope {
            r#type: "http".to_string(),
            method: Some("GET".to_string()),
            path: "/key.txt".to_string(),
            query_string: "versionId=123".to_string(),
            headers: vec![
                ("host".to_string(), "mybucket.s3.localhost.robotocore.cloud".to_string()),
                ("content-type".to_string(), "text/plain".to_string()),
            ],
        };
        let result = rewrite_vhost_to_path(&scope);
        assert!(result.is_some());
        let res = result.unwrap();
        assert_eq!(res.path, "/mybucket/key.txt");
        assert_eq!(res.query_string, "versionId=123");
    }

    #[test]
    fn test_non_s3_host_returns_none() {
        reset_cache();
        let scope = Scope {
            r#type: "http".to_string(),
            method: Some("GET".to_string()),
            path: "/key.txt".to_string(),
            query_string: "".to_string(),
            headers: vec![
                ("host".to_string(), "example.com".to_string()),
                ("content-type".to_string(), "text/plain".to_string()),
            ],
        };
        assert!(rewrite_vhost_to_path(&scope).is_none());
    }

    #[test]
    fn test_is_s3_vhost_request() {
        reset_cache();
        let scope = Scope {
            r#type: "http".to_string(),
            method: Some("GET".to_string()),
            path: "/".to_string(),
            query_string: "".to_string(),
            headers: vec![
                ("host".to_string(), "mybucket.s3.localhost.robotocore.cloud".to_string()),
            ],
        };
        assert!(is_s3_vhost_request(&scope));
    }

    #[test]
    fn test_is_not_s3_vhost_request() {
        reset_cache();
        let scope = Scope {
            r#type: "http".to_string(),
            method: Some("GET".to_string()),
            path: "/".to_string(),
            query_string: "".to_string(),
            headers: vec![("host".to_string(), "localhost:4566".to_string())],
        };
        assert!(!is_s3_vhost_request(&scope));
    }

    #[test]
    fn test_rewrite_preserves_headers() {
        reset_cache();
        let scope = Scope {
            r#type: "http".to_string(),
            method: Some("GET".to_string()),
            path: "/key.txt".to_string(),
            query_string: "".to_string(),
            headers: vec![
                ("host".to_string(), "mybucket.s3.localhost.robotocore.cloud".to_string()),
                ("content-type".to_string(), "text/plain".to_string()),
            ],
        };
        let result = rewrite_vhost_to_path(&scope);
        assert!(result.is_some());
        let res = result.unwrap();
        assert_eq!(res.path, "/mybucket/key.txt");
        let host_header = res.headers.iter().find(|h| h.0 == "host").unwrap();
        assert_eq!(host_header.1, "s3.localhost.robotocore.cloud");
        assert!(res.headers.iter().any(|h| h.0 == "content-type" && h.1 == "text/plain"));
    }

    #[test]
    fn test_rewrite_aws_regional_host() {
        reset_cache();
        let scope = Scope {
            r#type: "http".to_string(),
            method: Some("GET".to_string()),
            path: "/data.csv".to_string(),
            query_string: "".to_string(),
            headers: vec![("host".to_string(), "mybucket.s3.eu-west-1.amazonaws.com".to_string())],
        };
        let result = rewrite_vhost_to_path(&scope);
        assert!(result.is_some());
        assert_eq!(result.unwrap().path, "/mybucket/data.csv");
    }

    #[test]
    fn test_config_custom_hostname() {
        reset_cache();
        env::set_var("S3_HOSTNAME", "s3.mycompany.dev");
        let config = get_s3_routing_config();
        assert_eq!(config.s3_hostname, "s3.mycompany.dev");
        assert_eq!(config.website_hostname, "s3-website.s3.mycompany.dev");
        env::remove_var("S3_HOSTNAME");
        reset_cache();
    }

    #[test]
    fn test_config_default() {
        reset_cache();
        // Ensure no S3_HOSTNAME env var is set
        env::remove_var("S3_HOSTNAME");
        let config = get_s3_routing_config();
        assert_eq!(config.s3_hostname, "s3.localhost.robotocore.cloud");
        assert!(config.virtual_hosted_style);
        assert_eq!(config.website_hostname, "s3-website.s3.localhost.robotocore.cloud");
    }

    #[test]
    fn test_deeply_nested_key() {
        reset_cache();
        let scope = Scope {
            r#type: "http".to_string(),
            method: Some("GET".to_string()),
            path: "/a/b/c/d/e/file.json".to_string(),
            query_string: "".to_string(),
            headers: vec![("host".to_string(), "mybucket.s3.localhost.robotocore.cloud".to_string())],
        };
        let result = rewrite_vhost_to_path(&scope);
        assert!(result.is_some());
        assert_eq!(result.unwrap().path, "/mybucket/a/b/c/d/e/file.json");
    }

    #[test]
    fn test_key_with_special_chars() {
        reset_cache();
        let scope = Scope {
            r#type: "http".to_string(),
            method: Some("GET".to_string()),
            path: "/path%20with%20spaces/file.txt".to_string(),
            query_string: "".to_string(),
            headers: vec![("host".to_string(), "mybucket.s3.localhost.robotocore.cloud".to_string())],
        };
        let result = rewrite_vhost_to_path(&scope);
        assert!(result.is_some());
        assert_eq!(result.unwrap().path, "/mybucket/path%20with%20spaces/file.txt");
    }
}

// PyO3 FFI bindings for Python integration
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Parse an S3 virtual-hosted-style Host header.
///
/// Returns a Python dict with keys `bucket` and optionally `region`,
/// or `None` if the host does not match any S3 pattern.
#[pyfunction]
#[pyo3(name = "parse_s3_vhost")]
fn parse_s3_vhost_py(py: Python<'_>, host: Option<&str>) -> Option<Py<PyDict>> {
    let result = parse_s3_vhost(host);
    result.map(|info| {
        let dict = PyDict::new_bound(py);
        dict.set_item("bucket", info.bucket).unwrap();
        if let Some(region) = info.region {
            dict.set_item("region", region).unwrap();
        }
        dict.unbind()
    })
}

/// Check if an ASGI scope represents an S3 virtual-hosted-style request.
#[pyfunction]
#[pyo3(name = "is_s3_vhost_request")]
fn is_s3_vhost_request_py(scope: &Bound<'_, PyAny>) -> PyResult<bool> {
    // Extract scope type - try getattr first (for object), then get_item (for dict)
    let scope_type: String = if let Ok(t) = scope.getattr("type") {
        t.extract()?
    } else {
        scope.get_item("type")?.extract()?
    };
    if scope_type != "http" {
        return Ok(false);
    }
    
    // Extract headers
    let headers: Vec<(Vec<u8>, Vec<u8>)> = if let Ok(h) = scope.getattr("headers") {
        h.extract()?
    } else {
        scope.get_item("headers")?.extract()?
    };
    
    // Look for host header (bytes in ASGI)
    for (key, value) in headers {
        if key.as_slice() == b"host" {
            let host_str = String::from_utf8_lossy(&value);
            return Ok(parse_s3_vhost(Some(&host_str)).is_some());
        }
    }
    Ok(false)
}

/// Rewrite a virtual-hosted-style S3 request scope to path-style.
#[pyfunction]
#[pyo3(name = "rewrite_vhost_to_path")]
fn rewrite_vhost_to_path_py<'a>(scope: &'a Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
    // Extract host header - handle both dict and object access
    let headers_raw = if let Ok(h) = scope.getattr("headers") {
        h
    } else {
        scope.get_item("headers")?
    };
    let headers: Vec<(Vec<u8>, Vec<u8>)> = headers_raw.extract()?;
    
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
    
    let parsed = match parse_s3_vhost(Some(&host)) {
        Some(p) => p,
        None => return Ok(None),
    };
    
    let bucket = parsed.bucket;
    let path_value = if let Ok(p) = scope.getattr("path") {
        p
    } else {
        scope.get_item("path")?
    };
    let original_path: String = path_value.extract()?;
    
    // Rewrite path
    let new_path = if original_path == "/" {
        format!("/{}", bucket)
    } else {
        format!("/{}{}", bucket, original_path)
    };
    
    // Strip bucket from host
    let new_host = if host.len() > bucket.len() + 1 {
        host[bucket.len() + 1..].to_string()
    } else {
        host.clone()
    };
    
    // Build new headers (keep original byte format)
    let new_headers: Vec<(Vec<u8>, Vec<u8>)> = headers
        .iter()
        .map(|(k, v)| {
            if k.as_slice() == b"host" {
                (b"host".to_vec(), new_host.as_bytes().to_vec())
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect();
    
    // Create new scope dict
    let py = scope.py();
    let new_scope = PyDict::new_bound(py);
    
    // Copy type
    let type_value = if let Ok(t) = scope.getattr("type") {
        t
    } else {
        scope.get_item("type")?
    };
    new_scope.set_item("type", type_value)?;
    
    // Copy method if present
    if let Ok(method) = scope.getattr("method") {
        new_scope.set_item("method", method)?;
    } else if let Ok(method) = scope.get_item("method") {
        new_scope.set_item("method", method)?;
    }
    
    new_scope.set_item("path", &new_path)?;
    
    // Copy query_string
    let qs_value = if let Ok(qs) = scope.getattr("query_string") {
        qs
    } else {
        scope.get_item("query_string")?
    };
    new_scope.set_item("query_string", &qs_value)?;
    
    new_scope.set_item("headers", new_headers)?;
    
    // Set raw_path if query_string exists
    let qs: Vec<u8> = qs_value.extract()?;
    if !qs.is_empty() {
        let raw_path = format!("{}?{}", new_path, String::from_utf8_lossy(&qs));
        new_scope.set_item("raw_path", raw_path)?;
    } else {
        new_scope.set_item("raw_path", &new_path)?;
    }
    
    Ok(Some(new_scope.as_any().clone().unbind()))
}

/// Return the current S3 routing configuration.
#[pyfunction]
#[pyo3(name = "get_s3_routing_config")]
fn get_s3_routing_config_py(py: Python<'_>) -> Py<PyDict> {
    let config = get_s3_routing_config();
    let dict = PyDict::new_bound(py);
    dict.set_item("s3_hostname", config.s3_hostname).unwrap();
    dict.set_item("virtual_hosted_style", config.virtual_hosted_style).unwrap();
    dict.set_item("website_hostname", config.website_hostname).unwrap();
    dict.set_item("supported_patterns", config.supported_patterns).unwrap();
    dict.unbind()
}

/// Python module definition
#[pymodule]
fn robotocore_rust(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_s3_vhost_py, m)?)?;
    m.add_function(wrap_pyfunction!(is_s3_vhost_request_py, m)?)?;
    m.add_function(wrap_pyfunction!(rewrite_vhost_to_path_py, m)?)?;
    m.add_function(wrap_pyfunction!(get_s3_routing_config_py, m)?)?;
    Ok(())
}
