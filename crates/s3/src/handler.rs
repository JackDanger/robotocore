//! S3 operation handler.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{Bucket, MultipartUpload, S3State};
use crate::protocol::{AwsRequest, AwsResponse};
use crate::xml;
use serde_json::{json, Value};

/// HTTP method enum for S3 operation detection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method_ {
    Get,
    Put,
    Post,
    Delete,
    Head,
}

/// The S3 service handler.
pub struct S3Handler {
    /// Per (account, region) state.
    state: RwLock<HashMap<(u64, String), S3State>>,
}

impl S3Handler {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(HashMap::new()),
        }
    }

    fn get_state(&self, account: u64, region: &str) -> S3State {
        let mut states = self.state.write();
        states
            .entry((account, region.to_string()))
            .or_default()
            .clone()
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let operation = req.operation.as_str();
        match operation {
            "ListBuckets" => self.list_buckets(&req),
            "CreateBucket" => self.create_bucket(&req),
            "DeleteBucket" => self.delete_bucket(&req),
            "HeadBucket" => self.head_bucket(&req),
            "GetBucketLocation" => self.get_bucket_location(&req),
            "PutBucketPolicy" => self.put_bucket_policy(&req),
            "GetBucketPolicy" => self.get_bucket_policy(&req),
            "DeleteBucketPolicy" => self.delete_bucket_policy(&req),
            "PutObject" => self.put_object(&req),
            "GetObject" => self.get_object(&req),
            "HeadObject" => self.head_object(&req),
            "DeleteObject" => self.delete_object(&req),
            "ListObjects" | "ListObjectsV2" => self.list_objects(&req),
            "CopyObject" => self.copy_object(&req),
            "PutBucketCors" => self.put_bucket_cors(&req),
            "GetBucketCors" => self.get_bucket_cors(&req),
            "DeleteBucketCors" => self.delete_bucket_cors(&req),
            "PutBucketVersioning" => self.put_bucket_versioning(&req),
            "GetBucketVersioning" => self.get_bucket_versioning(&req),
            "PutBucketTagging" => self.put_bucket_tagging(&req),
            "GetBucketTagging" => self.get_bucket_tagging(&req),
            "DeleteBucketTagging" => self.delete_bucket_tagging(&req),
            "PutObjectTagging" => self.put_object_tagging(&req),
            "GetObjectTagging" => self.get_object_tagging(&req),
            "DeleteObjectTagging" => self.delete_object_tagging(&req),
            "CreateMultipartUpload" => self.create_multipart_upload(&req),
            "UploadPart" => self.upload_part(&req),
            "CompleteMultipartUpload" => self.complete_multipart_upload(&req),
            "AbortMultipartUpload" => self.abort_multipart_upload(&req),
            "GetBucketAcl" => self.get_bucket_acl(&req),
            "PutBucketAcl" => self.put_bucket_acl(&req),
            "GetObjectAcl" => self.get_object_acl(&req),
            "PutObjectAcl" => self.put_object_acl(&req),
            "RestoreObject" => self.restore_object(&req),
            "DeleteObjects" => self.delete_objects(&req),
            "GetBucketLifecycle" => self.get_bucket_lifecycle(&req),
            "PutBucketLifecycle" => self.put_bucket_lifecycle(&req),
            "DeleteBucketLifecycle" => self.delete_bucket_lifecycle(&req),
            "GetBucketWebsite" => self.get_bucket_website(&req),
            "PutBucketWebsite" => self.put_bucket_website(&req),
            "GetLifecycleConfiguration" => self.get_lifecycle_config(&req),
            "PutLifecycleConfiguration" => self.put_lifecycle_config(&req),
            "DeleteBucketWebsite" => self.delete_bucket_website(&req),
            other => AwsResponse::error(
                400,
                "NotImplemented",
                &format!("The operation {} is not implemented", other),
            ),
        }
    }

    // ---- Bucket operations ----

    fn list_buckets(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let buckets = state.list_buckets();
        let body = xml::list_buckets(&buckets);
        AwsResponse::xml(200, body)
    }

    fn create_bucket(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = match &req.bucket {
            Some(name) => name.clone(),
            None => {
                return AwsResponse::error(
                    400,
                    "InvalidBucketName",
                    "The specified bucket is not valid",
                );
            }
        };

        if !Self::valid_bucket_name(&bucket_name) {
            return AwsResponse::error(
                400,
                "InvalidBucketName",
                "The specified bucket is not valid",
            );
        }

        let state = self.get_state(req.account, &req.region);
        if state.get_bucket(&bucket_name).is_some() {
            // Match moto behavior: return success for duplicate bucket creation
            let body = xml::create_bucket_result(&bucket_name);
            return AwsResponse::xml(200, body);
        }

        let bucket = Arc::new(Bucket::new(bucket_name.clone(), req.region.clone()));
        state.put_bucket(bucket);

        let body = xml::create_bucket_result(&bucket_name);
        AwsResponse::xml(200, body)
    }

    fn delete_bucket(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = match &req.bucket {
            Some(name) => name.clone(),
            None => {
                return AwsResponse::error(
                    400,
                    "InvalidBucketName",
                    "The specified bucket is not valid",
                );
            }
        };

        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        if !bucket.objects.read().is_empty() {
            return AwsResponse::error(
                409,
                "BucketNotEmpty",
                "The bucket you tried to delete is not empty",
            );
        }

        state.delete_bucket(&bucket_name);
        AwsResponse::no_content(204)
    }

    fn head_bucket(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = match &req.bucket {
            Some(name) => name.clone(),
            None => {
                return AwsResponse::error(
                    400,
                    "InvalidBucketName",
                    "The specified bucket is not valid",
                );
            }
        };

        let state = self.get_state(req.account, &req.region);
        if state.get_bucket(&bucket_name).is_none() {
            return AwsResponse::error(404, "NoSuchBucket", "The specified bucket does not exist");
        }

        AwsResponse::no_content(200)
    }

    fn get_bucket_location(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = match &req.bucket {
            Some(name) => name.clone(),
            None => {
                return AwsResponse::error(
                    400,
                    "InvalidBucketName",
                    "The specified bucket is not valid",
                );
            }
        };

        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let body = xml::get_bucket_location(&bucket.region);
        AwsResponse::xml(200, body)
    }

    fn put_bucket_policy(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = match &req.bucket {
            Some(name) => name.clone(),
            None => {
                return AwsResponse::error(
                    400,
                    "InvalidBucketName",
                    "The specified bucket is not valid",
                );
            }
        };

        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let policy_str = String::from_utf8_lossy(&req.body).to_string();
        *bucket.policy.write() = Some(policy_str);
        AwsResponse::no_content(200)
    }

    fn get_bucket_policy(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = match &req.bucket {
            Some(name) => name.clone(),
            None => {
                return AwsResponse::error(
                    400,
                    "InvalidBucketName",
                    "The specified bucket is not valid",
                );
            }
        };

        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let policy = bucket.policy.read().clone();
        match policy {
            Some(p) => AwsResponse::binary(200, p.as_bytes().to_vec(), "application/json"),
            None => AwsResponse::error(
                404,
                "NoSuchBucketPolicy",
                "The bucket policy does not exist",
            ),
        }
    }

    fn delete_bucket_policy(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = match &req.bucket {
            Some(name) => name.clone(),
            None => {
                return AwsResponse::error(
                    400,
                    "InvalidBucketName",
                    "The specified bucket is not valid",
                );
            }
        };

        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        *bucket.policy.write() = None;
        AwsResponse::no_content(204)
    }

    // ---- Object operations ----

    fn put_object(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        // Check for CopyObject (PUT with x-amz-copy-source header)
        let copy_source = req.headers.get("x-amz-copy-source").cloned().unwrap_or_default();
        if !copy_source.is_empty() {
            return self.do_copy_object(req, &bucket_name, &key, &copy_source, &state);
        }

        let content_type = req
            .headers
            .get("content-type")
            .cloned()
            .unwrap_or_else(|| "binary/octet-stream".to_string());

        let obj = crate::models::Object::new(key.clone(), req.body.to_vec(), content_type);
        let etag = obj.etag.clone();
        bucket.objects.write().insert(key, obj);

        let mut resp = AwsResponse::xml(200, String::new());
        resp.headers.push(("ETag".to_string(), format!("\"{}\"", etag)));
        resp
    }

    fn do_copy_object(&self, req: &AwsRequest, dest_bucket: &str, dest_key: &str, copy_source: &str, state: &S3State) -> AwsResponse {
        let trimmed = copy_source.trim_start_matches('/');
        let source_parts: Vec<&str> = trimmed.splitn(2, '/').collect();
        if source_parts.len() < 2 {
            return AwsResponse::error(400, "InvalidArgument",
                "The x-amz-copy-source header must be in the format /source-bucket/source-key");
        }
        let source_bucket_name = source_parts[0];
        let source_key = source_parts[1];
        let source_bucket = match state.get_bucket(source_bucket_name) {
            Some(b) => b,
            None => return AwsResponse::error(404, "NoSuchBucket",
                "The specified source bucket does not exist"),
        };
        let (source_data, source_ct) = {
            let objects = source_bucket.objects.read();
            match objects.get(source_key) {
                Some(o) => (o.data.clone(), o.content_type.clone()),
                None => return AwsResponse::error(404, "NoSuchKey",
                    "The specified source key does not exist"),
            }
        };
        let dest_bucket = match state.get_bucket(dest_bucket) {
            Some(b) => b,
            None => return AwsResponse::error(404, "NoSuchBucket",
                "The specified destination bucket does not exist"),
        };
        let obj = crate::models::Object::new(dest_key.to_string(), source_data, source_ct);
        let etag = obj.etag.clone();
        dest_bucket.objects.write().insert(dest_key.to_string(), obj);
        AwsResponse::xml(200, format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><CopyObjectResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><LastModified>2024-01-01T00:00:00.000Z</LastModified><ETag>{}</ETag></CopyObjectResult>"#,
            etag
        ))
    }

    fn get_object(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let objects = bucket.objects.read();
        let obj = match objects.get(&key) {
            Some(o) => o,
            None => {
                return AwsResponse::error(404, "NoSuchKey", "The specified key does not exist");
            }
        };

        let mut resp = AwsResponse::binary(200, obj.data.clone(), &obj.content_type);
        resp.headers.push(("ETag".to_string(), format!("\"{}\"", obj.etag)));
        resp
    }

    fn head_object(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let objects = bucket.objects.read();
        let obj = match objects.get(&key) {
            Some(o) => o,
            None => {
                return AwsResponse::error(404, "NoSuchKey", "The specified key does not exist");
            }
        };

        let headers =
            xml::head_object_headers(obj.size, &obj.etag, &obj.content_type, obj.last_modified);
        AwsResponse {
            status: 200,
            headers,
            body: vec![],
        }
    }

    fn delete_object(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        bucket.objects.write().remove(&key);
        AwsResponse::no_content(204)
    }

    fn list_objects(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let prefix = req.query_params.get("prefix").cloned().unwrap_or_default();
        let max_keys: usize = req
            .query_params
            .get("max-keys")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000);
        let marker = req.query_params.get("marker").cloned().unwrap_or_default();

        let objects = bucket.objects.read();
        let mut contents: Vec<(String, String, usize, u64, String)> = Vec::new();
        let mut common_prefixes: Vec<String> = Vec::new();
        let mut seen_prefixes: Vec<String> = Vec::new();

        for (key, obj) in objects.iter() {
            if !key.starts_with(&prefix) {
                continue;
            }
            if !marker.is_empty() && key <= &marker {
                continue;
            }

            let delimiter = req.query_params.get("delimiter");
            if let Some(delim) = delimiter {
                let rest = &key[prefix.len()..];
                if let Some(pos) = rest.find(delim.as_str()) {
                    let prefix_part = &key[..prefix.len() + pos + delim.len()];
                    if !seen_prefixes.contains(&prefix_part.to_string()) {
                        seen_prefixes.push(prefix_part.to_string());
                        common_prefixes.push(prefix_part.to_string());
                    }
                    continue;
                }
            }

            contents.push((
                key.clone(),
                obj.etag.clone(),
                obj.size,
                obj.last_modified,
                obj.storage_class.clone(),
            ));

            if contents.len() >= max_keys {
                break;
            }
        }

        let is_truncated = false;

        let body = xml::list_objects_v2(
            &bucket_name,
            &prefix,
            &marker,
            max_keys,
            is_truncated,
            &contents,
            &common_prefixes,
        );
        // Add KeyCount and EncodingType
        let body = body
            .replace("</ListBucketResult>", &format!(
                "<KeyCount>{}</KeyCount><EncodingType>url</EncodingType></ListBucketResult>",
                contents.len()
            ));
        AwsResponse::xml(200, body)
    }

    fn copy_object(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let copy_source = req
            .headers
            .get("x-amz-copy-source")
            .cloned()
            .unwrap_or_default();

        let source_parts: Vec<&str> = copy_source.splitn(2, '/').collect();
        if source_parts.len() < 2 {
            return AwsResponse::error(
                400,
                "InvalidArgument",
                "The x-amz-copy-source header must be in the format /source-bucket/source-key",
            );
        }

        let source_bucket_name = source_parts[0].trim_start_matches('/');
        let source_key = source_parts[1];

        let source_bucket = match state.get_bucket(source_bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let source_obj = {
            let objects = source_bucket.objects.read();
            match objects.get(source_key) {
                Some(o) => (o.data.clone(), o.content_type.clone()),
                None => {
                    return AwsResponse::error(
                        404,
                        "NoSuchKey",
                        "The specified key does not exist",
                    );
                }
            }
        };
        let (source_data, source_content_type) = source_obj;
        let obj = crate::models::Object::new(key.clone(), source_data, source_content_type);
        let etag = obj.etag.clone();
        bucket.objects.write().insert(key, obj);

        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><CopyObjectResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <ETag>{}</ETag>
  <LastModified>2024-01-01T00:00:00.000Z</LastModified>
</CopyObjectResult>"#,
            etag
        );
        AwsResponse::xml(200, body)
    }

    // ---- CORS ----

    fn put_bucket_cors(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let _cors_xml = String::from_utf8_lossy(&req.body).to_string();
        *bucket.cors_rules.write() = Vec::new();
        AwsResponse::no_content(200)
    }

    fn get_bucket_cors(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        if bucket.cors_rules.read().is_empty() {
            return AwsResponse::error(
                404,
                "NoSuchCORSConfiguration",
                "The CORS configuration does not exist",
            );
        }

        let mut rules_xml = String::new();
        for rule in bucket.cors_rules.read().iter() {
            rules_xml.push_str("<CORSRule>");
            if let Some(m) = rule.get("AllowedMethods").and_then(|v| v.as_array()) {
                rules_xml.push_str("<AllowedMethod>");
                for m in m.iter().filter_map(|v| v.as_str()) {
                    rules_xml.push_str(m);
                    rules_xml.push_str("</AllowedMethod><AllowedMethod>");
                }
                rules_xml.pop(); // remove trailing <AllowedMethod>
                rules_xml.pop(); rules_xml.pop(); rules_xml.pop(); // remove >
                rules_xml.push_str("</AllowedMethod>");
            }
            if let Some(o) = rule.get("AllowedOrigins").and_then(|v| v.as_array()) {
                for o in o.iter().filter_map(|v| v.as_str()) {
                    rules_xml.push_str(&format!("<AllowedOrigin>{}</AllowedOrigin>", o));
                }
            }
            if let Some(h) = rule.get("AllowedHeaders").and_then(|v| v.as_array()) {
                for h in h.iter().filter_map(|v| v.as_str()) {
                    rules_xml.push_str(&format!("<AllowedHeader>{}</AllowedHeader>", h));
                }
            }
            if let Some(e) = rule.get("ExposeHeaders").and_then(|v| v.as_array()) {
                for e in e.iter().filter_map(|v| v.as_str()) {
                    rules_xml.push_str(&format!("<ExposeHeader>{}</ExposeHeader>", e));
                }
            }
            rules_xml.push_str("</CORSRule>");
        }
        let body = format!(r#"<?xml version="1.0" encoding="UTF-8"?><CORSConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">{}</CORSConfiguration>"#, rules_xml);
        AwsResponse::xml(200, body)
    }

    fn delete_bucket_cors(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        bucket.cors_rules.write().clear();
        AwsResponse::no_content(204)
    }

    // ---- Versioning ----

    fn put_bucket_versioning(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let body_str = String::from_utf8_lossy(&req.body);
        let enabled = body_str.contains("<Status>Enabled</Status>");
        let suspended = body_str.contains("<Status>Suspended</Status>");
        *bucket.versioning.write() = if enabled { Some(true) } else if suspended { Some(false) } else { None };
        AwsResponse::no_content(200)
    }

    fn get_bucket_versioning(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let status = match *bucket.versioning.read() {
            Some(true) => "Enabled",
            Some(false) => "Suspended",
            None => "",
        };
        let status_elem = if status.is_empty() {
            String::new()
        } else {
            format!("<Status>{}</Status>", status)
        };
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><VersioningConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  {}
</VersioningConfiguration>"#,
            status_elem
        );
        AwsResponse::xml(200, body)
    }

    // ---- Tagging ----

    fn put_bucket_tagging(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let body_str = String::from_utf8_lossy(&req.body);
        let mut tags = HashMap::new();
        let mut in_tag = false;
        let mut key = String::new();
        let mut value = String::new();

        for line in body_str.lines() {
            let trimmed = line.trim();
            if trimmed == "<Tag>" {
                in_tag = true;
                key.clear();
                value.clear();
            } else if trimmed == "</Tag>" {
                in_tag = false;
                if !key.is_empty() {
                    tags.insert(key.clone(), value.clone());
                }
            } else if in_tag && trimmed.starts_with("<Key>") {
                key = trimmed
                    .trim_start_matches("<Key>")
                    .trim_end_matches("</Key>")
                    .to_string();
            } else if in_tag && trimmed.starts_with("<Value>") {
                value = trimmed
                    .trim_start_matches("<Value>")
                    .trim_end_matches("</Value>")
                    .to_string();
            }
        }
        *bucket.tags.write() = tags;
        AwsResponse::no_content(200)
    }

    fn get_bucket_tagging(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let tags = bucket.tags.read();
        let mut xml_str = r#"<?xml version="1.0" encoding="UTF-8"?><Tagging xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><TagSet>"#.to_string();
        for (k, v) in tags.iter() {
            xml_str.push_str(&format!("<Tag><Key>{}</Key><Value>{}</Value></Tag>", k, v));
        }
        xml_str.push_str("</TagSet></Tagging>");
        AwsResponse::xml(200, xml_str)
    }

    fn delete_bucket_tagging(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        bucket.tags.write().clear();
        AwsResponse::no_content(204)
    }

    fn put_object_tagging(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let objects = bucket.objects.read();
        if !objects.contains_key(&key) {
            return AwsResponse::error(404, "NoSuchKey", "The specified key does not exist");
        }
        AwsResponse::no_content(200)
    }

    fn get_object_tagging(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, _key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let _bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let body = r#"<?xml version="1.0" encoding="UTF-8"?><Tagging xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><TagSet></TagSet></Tagging>"#;
        AwsResponse::xml(200, body.to_string())
    }

    fn delete_object_tagging(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, _key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let _bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        AwsResponse::no_content(204)
    }

    // ---- Multipart upload ----

    fn create_multipart_upload(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let content_type = req
            .headers
            .get("content-type")
            .cloned()
            .unwrap_or_else(|| "binary/octet-stream".to_string());

        let upload = MultipartUpload::new(key.clone(), content_type);
        let upload_id = upload.upload_id.clone();
        bucket
            .multipart_uploads
            .write()
            .insert(upload_id.clone(), upload);

        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><InitiateMultipartUploadResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Bucket>{}</Bucket>
  <Key>{}</Key>
  <UploadId>{}</UploadId>
</InitiateMultipartUploadResult>"#,
            bucket_name, key, upload_id
        );
        AwsResponse::xml(200, body)
    }

    fn upload_part(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, _key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let upload_id = req
            .query_params
            .get("uploadId")
            .cloned()
            .unwrap_or_default();
        let part_number: u32 = req
            .query_params
            .get("partNumber")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let uploads = bucket.multipart_uploads.read();
        let upload_exists = uploads.get(&upload_id).is_some();
        drop(uploads);

        if !upload_exists {
            return AwsResponse::error(
                404,
                "NoSuchUpload",
                "The specified multipart upload does not exist",
            );
        }

        let part = crate::models::MultipartPart {
            part_number,
            etag: {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(&req.body);
                hex::encode(hasher.finalize())
            },
            size: req.body.len(),
            data: req.body.to_vec(),
            last_modified: chrono::Utc::now().timestamp() as u64,
        };
        let etag = part.etag.clone();

        bucket
            .multipart_uploads
            .write()
            .get_mut(&upload_id)
            .unwrap()
            .parts
            .write()
            .insert(part_number, part);

        let mut resp = AwsResponse::xml(200, String::new());
        resp.headers.push(("ETag".to_string(), format!("\"{}\"", etag)));
        resp
    }

    fn complete_multipart_upload(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let upload_id = req
            .query_params
            .get("uploadId")
            .cloned()
            .unwrap_or_default();

        let uploads = bucket.multipart_uploads.read();
        let upload_exists = uploads.get(&upload_id).is_some();
        let content_type = uploads
            .get(&upload_id)
            .map(|u| u.content_type.clone())
            .unwrap_or_default();
        drop(uploads);

        if !upload_exists {
            return AwsResponse::error(
                404,
                "NoSuchUpload",
                "The specified multipart upload does not exist",
            );
        }

        // Combine parts in order
        let uploads = bucket.multipart_uploads.read();
        let upload = uploads.get(&upload_id).unwrap();
        let parts = upload.parts.read();
        let mut part_vec: Vec<_> = parts.iter().collect();
        part_vec.sort_by_key(|(k, _)| *k);
        let mut combined = Vec::new();
        for (_part_num, part) in &part_vec {
            combined.extend_from_slice(&part.data);
        }
        drop(parts);
        drop(uploads);

        let obj = crate::models::Object::new(key.clone(), combined, content_type);
        let etag = obj.etag.clone();
        bucket.objects.write().insert(key.clone(), obj);
        bucket.multipart_uploads.write().remove(&upload_id);

        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><CompleteMultipartUploadResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Location>http://s3.amazonaws.com/{}/{}
</Location>
  <Bucket>{}</Bucket>
  <Key>{}</Key>
  <ETag>{}</ETag>
</CompleteMultipartUploadResult>"#,
            bucket_name, key, bucket_name, key, etag
        );
        AwsResponse::xml(200, body)
    }

    fn abort_multipart_upload(&self, req: &AwsRequest) -> AwsResponse {
        let (bucket_name, _key) = Self::bucket_and_key(req);
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let upload_id = req
            .query_params
            .get("uploadId")
            .cloned()
            .unwrap_or_default();
        bucket.multipart_uploads.write().remove(&upload_id);
        AwsResponse::no_content(204)
    }

    // ---- ACL ----

    fn get_bucket_acl(&self, _req: &AwsRequest) -> AwsResponse {
        let body = r#"<?xml version="1.0" encoding="UTF-8"?><AccessControlPolicy xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Owner><ID>robotocore</ID><DisplayName>robotocore</DisplayName></Owner>
  <AccessControlList>
    <Grant><Grantee><ID>robotocore</ID><Type>CanonicalUser</Type></Grantee>
    <Permission>FULL_CONTROL</Permission>
    </Grant>
  </AccessControlList>
</AccessControlPolicy>"#.to_string();
        AwsResponse::xml(200, body)
    }

    fn put_bucket_acl(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::no_content(200)
    }

    fn get_object_acl(&self, _req: &AwsRequest) -> AwsResponse {
        let body = r#"<?xml version="1.0" encoding="UTF-8"?><AccessControlPolicy xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Owner><ID>robotocore</ID><DisplayName>robotocore</DisplayName></Owner><AccessControlList><Grant><Grantee><ID>robotocore</ID><Type>CanonicalUser</Type></Grantee><Permission>FULL_CONTROL</Permission></Grant></AccessControlList></AccessControlPolicy>"#;
        AwsResponse::xml(200, body.to_string())
    }

    fn put_object_acl(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::no_content(200)
    }

    // ---- Restore ----

    fn restore_object(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::no_content(202)
    }

    // ---- Batch delete ----

    fn delete_objects(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => {
                return AwsResponse::error(
                    404,
                    "NoSuchBucket",
                    "The specified bucket does not exist",
                );
            }
        };

        let body_str = String::from_utf8_lossy(&req.body);
        let mut deleted: Vec<String> = Vec::new();

        // Parse <Key>...</Key> elements (works for both multi-line and single-line XML)
        let mut rest: &str = &body_str;
        while let Some(start) = rest.find("<Key>") {
            let after = &rest[start + 5..];
            if let Some(end) = after.find("</Key>") {
                let key = &after[..end];
                if !key.is_empty() {
                    bucket.objects.write().remove(key);
                    deleted.push(key.to_string());
                }
            }
            rest = after;
        }

        let mut xml_str = r#"<?xml version="1.0" encoding="UTF-8"?><DeleteResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">"#.to_string();
        for k in &deleted {
            xml_str.push_str(&format!("<Deleted><Key>{}</Key></Deleted>", k));
        }
        xml_str.push_str("</DeleteResult>");
        AwsResponse::xml(200, xml_str)
    }

    // ---- Lifecycle ----

    fn get_bucket_lifecycle(&self, _req: &AwsRequest) -> AwsResponse {
        let body = r#"<?xml version="1.0" encoding="UTF-8"?><LifecycleConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/"></LifecycleConfiguration>"#;
        AwsResponse::xml(200, body.to_string())
    }

    fn put_bucket_lifecycle(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::no_content(200)
    }

    fn delete_bucket_lifecycle(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::no_content(204)
    }

    // ---- Website ----

    fn get_lifecycle_config(&self, _req: &AwsRequest) -> AwsResponse {
        let body = r#"<?xml version="1.0" encoding="UTF-8"?><LifecycleConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Rules/></LifecycleConfiguration>"#;
        AwsResponse::xml(200, body.to_string())
    }

    fn put_lifecycle_config(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::no_content(200)
    }

    fn get_bucket_website(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => return AwsResponse::error(404, "NoSuchBucket", "The specified bucket does not exist"),
        };
        let website = bucket.website.read();
        if website.is_none() {
            return AwsResponse::error(404, "NoSuchWebsiteConfiguration", "The specified bucket does not have a website configuration");
        }
        // Convert the stored JSON to XML
        let w = website.as_ref().unwrap();
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?><WebsiteConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">"#);
        if let Some(idx) = w.get("IndexDocument").and_then(|v| v.as_object()) {
            xml.push_str("<IndexDocument>");
            if let Some(suffix) = idx.get("Suffix").and_then(|v| v.as_str()) {
                xml.push_str(&format!("<Suffix>{}</Suffix>", suffix));
            }
            xml.push_str("</IndexDocument>");
        }
        if let Some(err) = w.get("ErrorDocument").and_then(|v| v.as_object()) {
            xml.push_str("<ErrorDocument>");
            if let Some(key) = err.get("Key").and_then(|v| v.as_str()) {
                xml.push_str(&format!("<Key>{}</Key>", key));
            }
            xml.push_str("</ErrorDocument>");
        }
        xml.push_str("</WebsiteConfiguration>");
        AwsResponse::xml(200, xml)
    }

    fn put_bucket_website(&self, req: &AwsRequest) -> AwsResponse {
        let bucket_name = req.bucket.clone().unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let bucket = match state.get_bucket(&bucket_name) {
            Some(b) => b,
            None => return AwsResponse::error(404, "NoSuchBucket", "The specified bucket does not exist"),
        };
        // Parse the XML body to get the website config
        let body = String::from_utf8_lossy(&req.body).to_string();
        let mut config = serde_json::Map::new();
        if let Some(start) = body.find("<IndexDocument>") {
            if let Some(end) = body.rfind("</IndexDocument>") {
                let inner = &body[start..end];
                if let Some(suffix_start) = inner.find("<Suffix>") {
                    if let Some(suffix_end) = inner.rfind("</Suffix>") {
                        let suffix = &inner[suffix_start + 8..suffix_end];
                        config.insert("IndexDocument".to_string(), json!({"Suffix": suffix}));
                    }
                }
            }
        }
        if let Some(start) = body.find("<ErrorDocument>") {
            if let Some(end) = body.rfind("</ErrorDocument>") {
                let inner = &body[start..end];
                if let Some(key_start) = inner.find("<Key>") {
                    if let Some(key_end) = inner.rfind("</Key>") {
                        let key = &inner[key_start + 5..key_end];
                        config.insert("ErrorDocument".to_string(), json!({"Key": key}));
                    }
                }
            }
        }
        *bucket.website.write() = Some(Value::Object(config));
        AwsResponse::no_content(200)
    }

    fn delete_bucket_website(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::no_content(204)
    }

    // ---- Operation detection ----

    /// Detect the S3 operation from HTTP method, path, and query params.
    /// Mirrors the Python `_detect_rest_operation` logic.
    pub fn detect_s3_operation(method: &str, path: &str, query_params: &std::collections::HashMap<String, String>) -> Option<String> {
        let parts: Vec<&str> = path.trim_start_matches('/').splitn(2, '/').collect();
        let has_bucket = parts.first().copied().filter(|s| !s.is_empty()).is_some();
        let has_key = parts.len() > 1;

        // Sub-resource query params
        let query_ops: &[(&str, &[(Method_, &str)])] = &[
            ("acl", &[(Method_::Get, "GetBucketAcl"), (Method_::Put, "PutBucketAcl"), (Method_::Delete, "DeleteBucketAcl")]),
            ("versioning", &[(Method_::Get, "GetBucketVersioning"), (Method_::Put, "PutBucketVersioning")]),
            ("tagging", &[(Method_::Get, "GetBucketTagging"), (Method_::Put, "PutBucketTagging"), (Method_::Delete, "DeleteBucketTagging")]),
            ("lifecycle", &[(Method_::Get, "GetBucketLifecycle"), (Method_::Put, "PutBucketLifecycle"), (Method_::Delete, "DeleteBucketLifecycle")]),
            ("cors", &[(Method_::Get, "GetBucketCors"), (Method_::Put, "PutBucketCors"), (Method_::Delete, "DeleteBucketCors")]),
            ("policy", &[(Method_::Get, "GetBucketPolicy"), (Method_::Put, "PutBucketPolicy"), (Method_::Delete, "DeleteBucketPolicy")]),
            ("location", &[(Method_::Get, "GetBucketLocation")]),
            ("website", &[(Method_::Get, "GetBucketWebsite"), (Method_::Put, "PutBucketWebsite"), (Method_::Delete, "DeleteBucketWebsite")]),
            ("delete", &[(Method_::Post, "DeleteObjects")]),
            ("uploads", &[(Method_::Post, "CreateMultipartUpload")]),
            ("uploadId", &[(Method_::Put, "UploadPart"), (Method_::Post, "CompleteMultipartUpload"), (Method_::Delete, "AbortMultipartUpload")]),
            ("restore", &[(Method_::Post, "RestoreObject")]),
        ];

        let method_ = match method {
            "GET" => Method_::Get,
            "PUT" => Method_::Put,
            "POST" => Method_::Post,
            "DELETE" => Method_::Delete,
            "HEAD" => Method_::Head,
            _ => Method_::Get,
        };

        for (param, ops) in query_ops {
            if query_params.contains_key(*param) {
                for (m, op) in *ops {
                    if *m == method_ {
                        return Some(op.to_string());
                    }
                }
            }
        }

        // Root path
        if !has_bucket {
            return match method {
                "GET" => Some("ListBuckets".to_string()),
                _ => None,
            };
        }

        // Object-level
        if has_key {
            return match method {
                "GET" => Some("GetObject".to_string()),
                "PUT" => Some("PutObject".to_string()),
                "DELETE" => Some("DeleteObject".to_string()),
                "HEAD" => Some("HeadObject".to_string()),
                "POST" => Some("PutObject".to_string()),
                _ => None,
            };
        }

        // Bucket-level
        match method {
            "GET" => {
                if query_params.get("list-type").map(|s| s.as_str()) == Some("2") {
                    Some("ListObjectsV2".to_string())
                } else {
                    Some("ListObjects".to_string())
                }
            }
            "PUT" => Some("CreateBucket".to_string()),
            "DELETE" => Some("DeleteBucket".to_string()),
            "HEAD" => Some("HeadBucket".to_string()),
            _ => None,
        }
    }

    // ---- Helpers ----

    fn valid_bucket_name(name: &str) -> bool {
        if name.len() < 3 || name.len() > 63 {
            return false;
        }
        if name.starts_with('.') || name.ends_with('.') {
            return false;
        }
        if name.starts_with('-') || name.ends_with('-') {
            return false;
        }
        if name.contains("--") {
            return false;
        }
        name.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
    }

    fn bucket_and_key(req: &AwsRequest) -> (String, String) {
        let bucket = req.bucket.clone().unwrap_or_default();
        let key = req.key.clone().unwrap_or_default();
        (bucket, key)
    }
}

impl Default for S3Handler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn make_req(
        operation: &str,
        bucket: Option<&str>,
        key: Option<&str>,
        body: &[u8],
    ) -> AwsRequest {
        AwsRequest {
            service: "s3".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            bucket: bucket.map(String::from),
            key: key.map(String::from),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            method: "GET".to_string(),
            body: Bytes::copy_from_slice(body),
            params: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_create_and_list_buckets() {
        let handler = S3Handler::new();

        let req = make_req("CreateBucket", Some("my-bucket"), None, b"");
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert!(String::from_utf8_lossy(&resp.body).contains("my-bucket"));

        let req = make_req("ListBuckets", None, None, b"");
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert!(String::from_utf8_lossy(&resp.body).contains("my-bucket"));
    }

    #[test]
    fn test_put_and_get_object() {
        let handler = S3Handler::new();

        handler.handle(make_req("CreateBucket", Some("test-bucket"), None, b""));

        let req = make_req(
            "PutObject",
            Some("test-bucket"),
            Some("hello.txt"),
            b"Hello, World!",
        );
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);

        let req = make_req("GetObject", Some("test-bucket"), Some("hello.txt"), b"");
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, b"Hello, World!");
    }

    #[test]
    fn test_delete_object() {
        let handler = S3Handler::new();

        handler.handle(make_req("CreateBucket", Some("test-bucket"), None, b""));
        handler.handle(make_req(
            "PutObject",
            Some("test-bucket"),
            Some("file.txt"),
            b"data",
        ));

        let req = make_req("DeleteObject", Some("test-bucket"), Some("file.txt"), b"");
        let resp = handler.handle(req);
        assert_eq!(resp.status, 204);

        let req = make_req("GetObject", Some("test-bucket"), Some("file.txt"), b"");
        let resp = handler.handle(req);
        assert_eq!(resp.status, 404);
    }

    #[test]
    fn test_nonexistent_bucket() {
        let handler = S3Handler::new();
        let req = make_req("HeadBucket", Some("no-such-bucket"), None, b"");
        let resp = handler.handle(req);
        assert_eq!(resp.status, 404);
        assert!(String::from_utf8_lossy(&resp.body).contains("NoSuchBucket"));
    }

    #[test]
    fn test_invalid_bucket_name() {
        let handler = S3Handler::new();
        let req = make_req("CreateBucket", Some("a"), None, b"");
        let resp = handler.handle(req);
        assert_eq!(resp.status, 400);
    }

    #[test]
    fn test_bucket_not_empty_delete() {
        let handler = S3Handler::new();
        handler.handle(make_req("CreateBucket", Some("test"), None, b""));
        handler.handle(make_req("PutObject", Some("test"), Some("file"), b"data"));

        let req = make_req("DeleteBucket", Some("test"), None, b"");
        let resp = handler.handle(req);
        assert_eq!(resp.status, 409);
    }
}
