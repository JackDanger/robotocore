//! Native SNS service implementation for robotocore.
//!
//! Implements the core SNS operations using the query protocol (XML).

pub mod error;
pub mod handler;
pub mod models;
pub mod protocol;

pub use protocol::{AwsRequest, AwsResponse};

/// Default SNS handler backed by in-memory state.
pub struct DefaultSnsHandler {
    pub(crate) inner: handler::SnsHandler,
}

impl DefaultSnsHandler {
    pub fn new() -> Self {
        Self {
            inner: handler::SnsHandler::new(),
        }
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        self.inner.handle(req)
    }
}

impl Default for DefaultSnsHandler {
    fn default() -> Self {
        Self::new()
    }
}
