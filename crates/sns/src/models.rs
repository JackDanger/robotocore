//! SNS in-memory state models.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// An SNS topic.
#[derive(Debug)]
pub struct Topic {
    pub name: String,
    pub arn: String,
    pub owner: u64,
    pub region: String,
    pub created: u64,
    pub display_name: Option<String>,
    pub policy: Option<String>,
    pub subscriptions: RwLock<HashMap<String, Subscription>>,
    pub fifo: bool,
}

impl Topic {
    pub fn new(name: String, account: u64, region: String, fifo: bool) -> Self {
        let arn = if fifo {
            format!("arn:aws:sns:{}:{}:{}:FIFO", region, account, name)
        } else {
            format!("arn:aws:sns:{}:{}:{}", region, account, name)
        };
        Self {
            name,
            arn,
            owner: account,
            region,
            created: chrono::Utc::now().timestamp() as u64,
            display_name: None,
            policy: None,
            subscriptions: RwLock::new(HashMap::new()),
            fifo,
        }
    }
}

/// An SNS subscription.
#[derive(Debug, Clone)]
pub struct Subscription {
    pub subscription_arn: String,
    pub topic_arn: String,
    pub protocol: String,
    pub endpoint: String,
    pub owner: u64,
    pub confirmed: bool,
    pub created: u64,
}

/// The SNS state store (per account+region).
#[derive(Clone)]
pub struct SnsState {
    pub topics: Arc<RwLock<HashMap<String, Arc<Topic>>>>,
    pub subscriptions: Arc<RwLock<Vec<Subscription>>>,
}

impl SnsState {
    pub fn new() -> Self {
        Self {
            topics: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn get_topic(&self, arn: &str) -> Option<Arc<Topic>> {
        self.topics.read().get(arn).cloned()
    }

    pub fn put_topic(&self, topic: Arc<Topic>) {
        self.topics.write().insert(topic.arn.clone(), topic);
    }

    pub fn delete_topic(&self, arn: &str) -> Option<Arc<Topic>> {
        self.topics.write().remove(arn)
    }

    pub fn list_topics(&self) -> Vec<Arc<Topic>> {
        self.topics.read().values().cloned().collect()
    }

    pub fn add_subscription(&self, sub: Subscription) {
        self.subscriptions.write().push(sub);
    }

    pub fn list_subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions.read().clone()
    }

    pub fn remove_subscription(&self, arn: &str) {
        self.subscriptions.write().retain(|s| s.subscription_arn != arn);
    }
}

impl Default for SnsState {
    fn default() -> Self {
        Self::new()
    }
}
