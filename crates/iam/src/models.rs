//! IAM in-memory state models.

use parking_lot::RwLock;
use serde_json::json;
use std::collections::{HashMap, HashSet};
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
    pub permissions_boundary: RwLock<Option<String>>,
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
            permissions_boundary: RwLock::new(None),
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
    pub permissions_boundary: RwLock<Option<String>>,
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
            permissions_boundary: RwLock::new(None),
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
    pub permissions_boundary: RwLock<Option<String>>,
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
            permissions_boundary: RwLock::new(None),
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
    pub version_count: RwLock<u32>,
    pub versions: RwLock<Vec<serde_json::Value>>,
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
            policy_document: RwLock::new(document.clone()),
            tags: RwLock::new(Vec::new()),
            version_count: RwLock::new(1),
            versions: RwLock::new(vec![json!({
                "VersionId": "v1",
                "IsDefaultVersion": true,
                "CreateDate": chrono::Utc::now().to_rfc3339(),
                "Document": document
            })]),
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
    pub saml_providers: Arc<RwLock<HashMap<String, String>>>,
    pub oidc_providers: Arc<RwLock<HashMap<String, String>>>,
    pub mfa_devices: Arc<RwLock<Vec<serde_json::Value>>>,
    pub instance_profiles: Arc<RwLock<HashMap<String, Vec<String>>>>,
    pub policy_version_counter: Arc<std::sync::atomic::AtomicU64>,
    pub ssh_keys: Arc<RwLock<HashMap<String, Vec<String>>>>,
    pub server_certs: Arc<RwLock<Vec<String>>>,
    pub login_profiles: Arc<RwLock<HashSet<String>>>,
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
            saml_providers: Arc::new(RwLock::new(HashMap::new())),
            oidc_providers: Arc::new(RwLock::new(HashMap::new())),
            mfa_devices: Arc::new(RwLock::new(Vec::new())),
            instance_profiles: Arc::new(RwLock::new(HashMap::new())),
            policy_version_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            ssh_keys: Arc::new(RwLock::new(HashMap::new())),
            server_certs: Arc::new(RwLock::new(Vec::new())),
            login_profiles: Arc::new(RwLock::new(HashSet::new())),
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

    // SAML/OIDC
    pub fn create_saml_provider(&self, name: &str, doc: &str) {
        self.saml_providers.write().insert(name.to_string(), doc.to_string());
    }
    pub fn delete_saml_provider(&self, name: &str) {
        self.saml_providers.write().remove(name);
    }
    pub fn list_saml_providers(&self) -> Vec<serde_json::Value> {
        self.saml_providers.read().keys().map(|n| serde_json::json!({
            "Arn": format!("arn:aws:iam::123456789012:saml-provider/{}", n),
            "Name": n,
        })).collect()
    }
    pub fn update_saml_provider(&self, name: &str, doc: &str) {
        self.saml_providers.write().insert(name.to_string(), doc.to_string());
    }
    pub fn create_oidc_provider(&self, url: &str, client: &str) {
        self.oidc_providers.write().insert(url.to_string(), client.to_string());
    }
    pub fn delete_oidc_provider(&self, url: &str) {
        self.oidc_providers.write().remove(url);
    }
    pub fn list_oidc_providers(&self) -> Vec<serde_json::Value> {
        self.oidc_providers.read().keys().map(|u| serde_json::json!({
            "Arn": format!("arn:aws:iam::123456789012:oidc-provider/{}", u),
            "Url": u,
        })).collect()
    }

    // Instance Profiles
    pub fn create_instance_profile(&self, name: &str) {
        self.instance_profiles.write().entry(name.to_string()).or_insert_with(Vec::new);
    }
    pub fn delete_instance_profile(&self, name: &str) {
        self.instance_profiles.write().remove(name);
    }
    pub fn list_instance_profiles(&self) -> Vec<serde_json::Value> {
        self.instance_profiles.read().iter().map(|(name, roles)| serde_json::json!({
            "InstanceProfileName": name,
            "InstanceProfileArn": format!("arn:aws:iam::123456789012:instance-profile/{}", name),
            "Path": "/",
            "CreateDate": "2024-01-01T00:00:00Z",
            "Roles": roles.iter().map(|r| serde_json::json!({
                "RoleName": r,
                "RoleArn": format!("arn:aws:iam::123456789012:role/{}", r),
            })).collect::<Vec<_>>(),
        })).collect()
    }
    pub fn add_role_to_instance_profile(&self, profile: &str, role: &str) {
        self.instance_profiles.write().entry(profile.to_string()).or_insert_with(Vec::new).push(role.to_string());
    }
    pub fn remove_role_from_instance_profile(&self, profile: &str, role: &str) {
        if let Some(roles) = self.instance_profiles.write().get_mut(profile) {
            roles.retain(|r| r != role);
        }
    }

    // SSH Keys
    pub fn upload_ssh_public_key(&self, user: &str, name: &str) {
        self.ssh_keys.write().entry(user.to_string()).or_insert_with(Vec::new).push(name.to_string());
    }
    pub fn list_ssh_public_keys(&self, user: &str) -> Vec<serde_json::Value> {
        self.ssh_keys.read().get(user).map(|keys| keys.iter().map(|k| serde_json::json!({
            "SSHPublicKeyId": format!("{}-{}", user, k),
            "SSHPublicKeyName": k,
            "UserName": user,
            "Status": "Active",
            "CreatedDate": "2024-01-01T00:00:00Z",
        })).collect()).unwrap_or_default()
    }

    // Server Certs
    pub fn upload_server_certificate(&self, name: &str) {
        self.server_certs.write().push(name.to_string());
    }
    pub fn delete_server_certificate(&self, name: &str) {
        self.server_certs.write().retain(|c| c != name);
    }

    // Login Profiles
    pub fn create_login_profile(&self, user: &str) {
        self.login_profiles.write().insert(user.to_string());
    }
    pub fn delete_login_profile(&self, user: &str) {
        self.login_profiles.write().remove(user);
    }
}

impl Default for IamState {
    fn default() -> Self { Self::new() }
}
