//! IAM in-memory state models.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// An IAM user.
#[derive(Debug)]
pub struct User {
    pub user_id: String,
    pub username: String,
    pub arn: String,
    pub path: String,
    pub created: u64,
    pub password: RwLock<Option<String>>,
    pub access_keys: RwLock<Vec<AccessKey>>,
    pub groups: RwLock<Vec<String>>,
    pub tags: RwLock<Vec<serde_json::Value>>,
    pub policies: RwLock<Vec<String>>,
    pub attached_policies: RwLock<Vec<String>>,
}

impl User {
    pub fn new(account: u64, username: String) -> Self {
        let user_id = format!("AIDA{}", uuid::Uuid::new_v4().simple().to_string().to_uppercase().chars().take(20).collect::<String>());
        let arn = format!("arn:aws:iam::{}:user/{}", account, username);
        Self {
            user_id: user_id.clone(),
            username,
            arn,
            path: "/".to_string(),
            created: chrono::Utc::now().timestamp_millis() as u64,
            password: RwLock::new(None),
            access_keys: RwLock::new(Vec::new()),
            groups: RwLock::new(Vec::new()),
            tags: RwLock::new(Vec::new()),
            policies: RwLock::new(Vec::new()),
            attached_policies: RwLock::new(Vec::new()),
        }
    }
}

/// An IAM access key.
#[derive(Debug, Clone)]
pub struct AccessKey {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub status: String,
    pub created: u64,
}

impl AccessKey {
    pub fn new(account: u64) -> Self {
        let mut chars: Vec<char> = vec!['A'];
        chars.push('K');
        chars.push('I');
        chars.push('A');
        for _ in 0..16 {
            let c = (chrono::Utc::now().timestamp_subsec_nanos() % 26) as u8;
            chars.push((b'A' + c) as char);
        }
        Self {
            access_key_id: chars.into_iter().collect(),
            secret_access_key: base64::encode(format!("{}", uuid::Uuid::new_v4()).into_bytes()),
            status: "Active".to_string(),
            created: chrono::Utc::now().timestamp_millis() as u64,
        }
    }
}

/// An IAM role.
#[derive(Debug)]
pub struct Role {
    pub role_id: String,
    pub role_name: String,
    pub arn: String,
    pub path: String,
    pub assume_role_policy: RwLock<String>,
    pub description: RwLock<String>,
    pub max_session_duration: u32,
    pub created: u64,
    pub tags: RwLock<Vec<serde_json::Value>>,
    pub attached_policies: RwLock<Vec<String>>,
    pub inline_policies: RwLock<Vec<String>>,
}

impl Role {
    pub fn new(account: u64, role_name: String, assume_role_policy: String) -> Self {
        let role_id = format!("AROA{}", uuid::Uuid::new_v4().simple().to_string().to_uppercase().chars().take(20).collect::<String>());
        let arn = format!("arn:aws:iam::{}:role/{}", account, role_name);
        Self {
            role_id: role_id.clone(),
            role_name,
            arn,
            path: "/".to_string(),
            assume_role_policy: RwLock::new(assume_role_policy),
            description: RwLock::new(String::new()),
            max_session_duration: 3600,
            created: chrono::Utc::now().timestamp_millis() as u64,
            tags: RwLock::new(Vec::new()),
            attached_policies: RwLock::new(Vec::new()),
            inline_policies: RwLock::new(Vec::new()),
        }
    }
}

/// An IAM group.
#[derive(Debug)]
pub struct Group {
    pub group_id: String,
    pub group_name: String,
    pub arn: String,
    pub path: String,
    pub created: u64,
    pub users: RwLock<Vec<String>>,
    pub tags: RwLock<Vec<serde_json::Value>>,
    pub attached_policies: RwLock<Vec<String>>,
    pub inline_policies: RwLock<Vec<String>>,
}

impl Group {
    pub fn new(account: u64, group_name: String) -> Self {
        let group_id = format!("AGPA{}", uuid::Uuid::new_v4().simple().to_string().to_uppercase().chars().take(20).collect::<String>());
        let arn = format!("arn:aws:iam::{}:group/{}", account, group_name);
        Self {
            group_id: group_id.clone(),
            group_name,
            arn,
            path: "/".to_string(),
            created: chrono::Utc::now().timestamp_millis() as u64,
            users: RwLock::new(Vec::new()),
            tags: RwLock::new(Vec::new()),
            attached_policies: RwLock::new(Vec::new()),
            inline_policies: RwLock::new(Vec::new()),
        }
    }
}

/// An IAM policy.
#[derive(Debug)]
pub struct Policy {
    pub policy_id: String,
    pub policy_name: String,
    pub arn: String,
    pub path: String,
    pub default_version_id: RwLock<String>,
    pub description: String,
    pub created: u64,
    pub modified: u64,
    pub attachments: u32,
    pub policy_document: RwLock<String>,
    pub tags: RwLock<Vec<serde_json::Value>>,
}

impl Policy {
    pub fn new(account: u64, policy_name: String, document: String) -> Self {
        let policy_id = format!("ANPA{}", uuid::Uuid::new_v4().simple().to_string().to_uppercase().chars().take(17).collect::<String>());
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let arn = format!("arn:aws:iam::{}:policy/{}", account, policy_name);
        Self {
            policy_id: policy_id.clone(),
            policy_name,
            arn,
            path: "/".to_string(),
            default_version_id: RwLock::new("v1".to_string()),
            description: String::new(),
            created: now,
            modified: now,
            attachments: 0,
            policy_document: RwLock::new(document),
            tags: RwLock::new(Vec::new()),
        }
    }
}

/// The IAM state store (per account, global region).
#[derive(Clone)]
pub struct IamState {
    pub users: Arc<RwLock<HashMap<String, Arc<User>>>>,
    pub roles: Arc<RwLock<HashMap<String, Arc<Role>>>>,
    pub groups: Arc<RwLock<HashMap<String, Arc<Group>>>>,
    pub policies: Arc<RwLock<HashMap<String, Arc<Policy>>>>,
    pub account_alias: Arc<RwLock<Option<String>>>,
    pub password_policy: Arc<RwLock<Option<serde_json::Value>>>,
}

impl IamState {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            roles: Arc::new(RwLock::new(HashMap::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            account_alias: Arc::new(RwLock::new(None)),
            password_policy: Arc::new(RwLock::new(None)),
        }
    }

    pub fn get_user(&self, name: &str) -> Option<Arc<User>> {
        self.users.read().get(name).cloned()
    }

    pub fn get_role(&self, name: &str) -> Option<Arc<Role>> {
        self.roles.read().get(name).cloned()
    }

    pub fn get_group(&self, name: &str) -> Option<Arc<Group>> {
        self.groups.read().get(name).cloned()
    }

    pub fn get_policy(&self, arn_or_name: &str) -> Option<Arc<Policy>> {
        let policies = self.policies.read();
        if let Some(p) = policies.get(arn_or_name) { return Some(p.clone()); }
        policies.values().find(|p| p.arn == arn_or_name).cloned()
    }
}

impl Default for IamState {
    fn default() -> Self { Self::new() }
}
