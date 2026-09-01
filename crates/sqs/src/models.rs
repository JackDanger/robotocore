//! SQS data models: messages, queues, and storage

use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// A single SQS message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqsMessage {
    pub message_id: String,
    pub body: String,
    pub md5_of_body: String,
    pub receipt_handle: String,
    pub sent_timestamp: u64,
    pub visibility_until: Option<u64>, // Unix timestamp in milliseconds when message becomes visible again
    pub receive_count: u32,
    pub first_receive_timestamp: Option<u64>,
    pub attributes: HashMap<String, String>,
    pub message_attributes: HashMap<String, serde_json::Value>,
}

impl SqsMessage {
    /// Check if message is currently visible
    pub fn is_visible(&self) -> bool {
        if let Some(until) = self.visibility_until {
            let now_ms = (Utc::now().timestamp_millis()) as u64;
            now_ms >= until
        } else {
            true
        }
    }

    /// Set visibility timeout in seconds from now
    pub fn set_visibility_timeout(&mut self, timeout_seconds: u32) {
        let now_ms = (Utc::now().timestamp_millis()) as u64;
        self.visibility_until = Some(now_ms + (timeout_seconds as u64 * 1000));
    }
}

/// SQS Queue attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueAttributes {
    pub visibility_timeout: u32,        // default 30
    pub message_retention_period: u32,  // default 345600 (4 days)
    pub maximum_message_size: u32,      // default 262144 (256 KB)
    pub delay_seconds: u32,             // default 0
    pub receive_message_wait_time: u32, // default 0
    pub policy: Option<String>,
}

impl Default for QueueAttributes {
    fn default() -> Self {
        Self {
            visibility_timeout: 30,
            message_retention_period: 345600,
            maximum_message_size: 262144,
            delay_seconds: 0,
            receive_message_wait_time: 0,
            policy: None,
        }
    }
}

/// A queue in SQS (behind RwLock for thread-safe mutation)
#[derive(Debug)]
pub struct Queue {
    pub name: String,
    pub arn: String,
    pub url: String,
    pub region: String,
    pub account_id: String,
    pub created: u64,
    pub last_modified: u64,
    pub attributes: QueueAttributes,
    pub messages: VecDeque<SqsMessage>,
    pub tags: parking_lot::RwLock<std::collections::HashMap<String, String>>,
}

impl Queue {
    pub fn new(name: String, region: String, account_id: String, url: String, arn: String) -> Self {
        let now_ms = (Utc::now().timestamp_millis()) as u64;
        Self {
            name,
            arn,
            url,
            region,
            account_id,
            created: now_ms,
            last_modified: now_ms,
            attributes: QueueAttributes::default(),
            messages: VecDeque::new(),
            tags: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Add a message to the queue
    pub fn send_message(&mut self, message: SqsMessage) {
        self.messages.push_back(message);
    }

    /// Get visible messages (respecting visibility timeout)
    pub fn receive_messages(
        &mut self,
        max_count: usize,
        visibility_timeout: u32,
    ) -> Vec<SqsMessage> {
        let mut result = Vec::new();
        let now_ms = (Utc::now().timestamp_millis()) as u64;
        let first_receive_ts = now_ms;

        for _ in 0..max_count {
            // Find next visible message
            let mut found_idx = None;
            for (idx, msg) in self.messages.iter().enumerate() {
                if msg.is_visible() {
                    found_idx = Some(idx);
                    break;
                }
            }

            if let Some(idx) = found_idx {
                if let Some(mut msg) = self.messages.remove(idx) {
                    // Update message state
                    msg.receive_count += 1;
                    if msg.first_receive_timestamp.is_none() {
                        msg.first_receive_timestamp = Some(first_receive_ts);
                    }
                    msg.set_visibility_timeout(visibility_timeout);

                    // Update attributes
                    msg.attributes
                        .insert("SenderId".to_string(), "123456789012".to_string());
                    msg.attributes
                        .insert("SentTimestamp".to_string(), msg.sent_timestamp.to_string());
                    msg.attributes.insert(
                        "ApproximateReceiveCount".to_string(),
                        msg.receive_count.to_string(),
                    );
                    msg.attributes.insert(
                        "ApproximateFirstReceiveTimestamp".to_string(),
                        msg.first_receive_timestamp
                            .unwrap_or(first_receive_ts)
                            .to_string(),
                    );

                    result.push(msg);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Put messages back in queue at the end
        for msg in result.iter() {
            self.messages.push_back(msg.clone());
        }

        result
    }

    /// Remove message by receipt handle
    pub fn delete_message(&mut self, receipt_handle: &str) -> Result<(), String> {
        for (idx, msg) in self.messages.iter().enumerate() {
            if msg.receipt_handle == receipt_handle {
                self.messages.remove(idx);
                return Ok(());
            }
        }
        Err(format!(
            "Message not found with receipt handle: {}",
            receipt_handle
        ))
    }

    /// Change the visibility timeout of a message (identified by receipt handle).
    /// Returns true if the message was found and updated.
    pub fn change_message_visibility(
        &mut self,
        receipt_handle: &str,
        timeout_seconds: u32,
    ) -> bool {
        if let Some(msg) = self
            .messages
            .iter_mut()
            .find(|m| m.receipt_handle == receipt_handle)
        {
            msg.set_visibility_timeout(timeout_seconds);
            self.last_modified = (Utc::now().timestamp_millis()) as u64;
            true
        } else {
            false
        }
    }

    /// Remove all messages from the queue.
    pub fn purge(&mut self) {
        self.messages.clear();
        self.last_modified = (Utc::now().timestamp_millis()) as u64;
    }
}

/// Thread-safe SQS storage for a single account+region
pub struct SqsStore {
    queues: Arc<RwLock<HashMap<String, Arc<RwLock<Queue>>>>>,
}

impl SqsStore {
    pub fn new() -> Self {
        Self {
            queues: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn create_queue(&self, queue: Queue) {
        let mut q = self.queues.write();
        q.insert(queue.name.clone(), Arc::new(RwLock::new(queue)));
    }

    pub fn get_queue(&self, name: &str) -> Option<Arc<RwLock<Queue>>> {
        let q = self.queues.read();
        q.get(name).cloned()
    }

    pub fn delete_queue(&self, name: &str) -> bool {
        let mut q = self.queues.write();
        q.remove(name).is_some()
    }

    pub fn list_queues(&self) -> Vec<String> {
        let q = self.queues.read();
        q.keys().cloned().collect()
    }
}

impl Default for SqsStore {
    fn default() -> Self {
        Self::new()
    }
}
