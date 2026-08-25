//! Native Secrets Manager service for robotocore.
//!
//! JSON protocol (x-amz-json-1.1) with target prefix "secretsmanager".

pub mod handler;
pub mod models;
pub mod protocol;

pub use protocol::{AwsRequest, AwsResponse};

pub struct DefaultSecretsManagerHandler {
    pub(crate) inner: handler::SecretsManagerHandler,
}

impl DefaultSecretsManagerHandler {
    pub fn new() -> Self {
        Self {
            inner: handler::SecretsManagerHandler::new(),
        }
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        self.inner.handle(req)
    }
}

impl Default for DefaultSecretsManagerHandler {
    fn default() -> Self {
        Self::new()
    }
}
