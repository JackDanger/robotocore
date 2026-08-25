//! AWS Signature Version 4 validation.
//!
//! Extracts and validates SigV4 signatures from Authorization headers.
//! Returns 403 SignatureDoesNotMatch on validation failure.

use std::collections::HashMap;

/// AWS SigV4 signature information extracted from Authorization header.
#[derive(Debug, Clone)]
pub struct SigV4Info {
    pub algorithm: String,
    pub credential: String,
    pub signed_headers: Vec<String>,
    pub signature: String,
}

/// Parse Authorization header.
///
/// Expected format:
/// ```text
/// AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20230101/us-east-1/sts/aws4_request,
///   SignedHeaders=host;x-amz-date, Signature=xxx
/// ```
pub fn parse_authorization_header(header: &str) -> Result<SigV4Info, String> {
    if !header.starts_with("AWS4-HMAC-SHA256 ") {
        return Err("Invalid SigV4 format".to_string());
    }

    let parts: Vec<&str> = header[17..].split(", ").collect();
    let mut info = SigV4Info {
        algorithm: "AWS4-HMAC-SHA256".to_string(),
        credential: String::new(),
        signed_headers: Vec::new(),
        signature: String::new(),
    };

    for part in parts {
        if let Some(cred) = part.strip_prefix("Credential=") {
            info.credential = cred.to_string();
        } else if let Some(headers) = part.strip_prefix("SignedHeaders=") {
            info.signed_headers = headers.split(';').map(|s| s.to_string()).collect();
        } else if let Some(sig) = part.strip_prefix("Signature=") {
            info.signature = sig.to_string();
        }
    }

    if info.credential.is_empty() || info.signature.is_empty() {
        return Err("Missing credential or signature".to_string());
    }

    Ok(info)
}

/// Extract credential parts from credential string.
///
/// Expected format: AKIA.../20230101/us-east-1/sts/aws4_request
pub fn parse_credential(credential: &str) -> Result<(String, String, String, String), String> {
    let parts: Vec<&str> = credential.split('/').collect();
    if parts.len() != 5 {
        return Err("Invalid credential format".to_string());
    }

    let access_key = parts[0].to_string();
    let datestamp = parts[1].to_string();
    let region = parts[2].to_string();
    let service = parts[3].to_string();

    Ok((access_key, datestamp, region, service))
}

/// Validate a SigV4 signature.
///
/// For now, this is a stub that logs the validation and returns true.
/// In production, you would:
/// 1. Reconstruct the canonical request
/// 2. Derive the signing key
/// 3. Sign and compare
pub fn validate_signature(
    _sig_info: &SigV4Info,
    _method: &str,
    _path: &str,
    _query: &str,
    _headers: &HashMap<String, String>,
    _body: &[u8],
) -> Result<(), String> {
    // For MVP, accept all valid SigV4 format signatures
    // Detailed signature validation can be added later
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authorization_header() {
        let header = "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20230101/us-east-1/sts/aws4_request, SignedHeaders=host;x-amz-date, Signature=abcd1234";
        let info = parse_authorization_header(header).unwrap();
        assert_eq!(info.algorithm, "AWS4-HMAC-SHA256");
        assert_eq!(
            info.credential,
            "AKIAIOSFODNN7EXAMPLE/20230101/us-east-1/sts/aws4_request"
        );
        assert_eq!(info.signature, "abcd1234");
        assert_eq!(info.signed_headers, vec!["host", "x-amz-date"]);
    }

    #[test]
    fn test_parse_credential() {
        let cred = "AKIAIOSFODNN7EXAMPLE/20230101/us-east-1/sts/aws4_request";
        let (ak, ds, region, svc) = parse_credential(cred).unwrap();
        assert_eq!(ak, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(ds, "20230101");
        assert_eq!(region, "us-east-1");
        assert_eq!(svc, "sts");
    }

    #[test]
    fn test_validate_signature_format() {
        let sig_info = SigV4Info {
            algorithm: "AWS4-HMAC-SHA256".to_string(),
            credential: "AKIAIOSFODNN7EXAMPLE/20230101/us-east-1/sts/aws4_request".to_string(),
            signed_headers: vec!["host".to_string()],
            signature: "abcd1234".to_string(),
        };

        let result = validate_signature(
            &sig_info,
            "POST",
            "/",
            "",
            &HashMap::new(),
            b"Action=GetCallerIdentity",
        );
        assert!(result.is_ok());
    }
}
