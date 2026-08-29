//! IAM operation handler (query protocol, XML responses).

use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{AccessKey, Group, IamState, Policy, Role, User};
use crate::protocol::{AwsRequest, AwsResponse, get_param};

pub struct IamHandler {
    state: RwLock<HashMap<u64, IamState>>,
}

impl IamHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64) -> IamState {
        let mut states = self.state.write();
        states.entry(account).or_insert_with(IamState::new).clone()
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let op = req.operation.as_str();
        match op {
            // Users
            "CreateUser" => self.create_user(&req),
            "GetUser" => self.get_user(&req),
            "DeleteUser" => self.delete_user(&req),
            "ListUsers" => self.list_users(&req),
            "UpdateUser" => self.update_user(&req),
            "ChangePassword" => self.change_password(&req),
            "CreateAccessKey" => self.create_access_key(&req),
            "DeleteAccessKey" => self.delete_access_key(&req),
            "ListAccessKeys" => self.list_access_keys(&req),
            "UpdateAccessKey" => self.update_access_key(&req),
            "TagUser" => self.tag_user(&req),
            "UntagUser" => self.untag_user(&req),
            "ListUserTags" => self.list_user_tags(&req),
            "AttachUserPolicy" => self.attach_user_policy(&req),
            "DetachUserPolicy" => self.detach_user_policy(&req),
            "ListAttachedUserPolicies" => self.list_attached_user_policies(&req),
            "PutUserPolicy" => self.put_user_policy(&req),
            "GetUserPolicy" => self.get_user_policy(&req),
            "DeleteUserPolicy" => self.delete_user_policy(&req),
            "ListUserPolicies" => self.list_user_policies(&req),

            // Roles
            "CreateRole" => self.create_role(&req),
            "GetRole" => self.get_role(&req),
            "DeleteRole" => self.delete_role(&req),
            "ListRoles" => self.list_roles(&req),
            "UpdateRole" => self.update_role(&req),
            "UpdateRoleDescription" => self.update_role_description(&req),
            "UpdateAssumeRolePolicy" => self.update_assume_role_policy(&req),
            "TagRole" => self.tag_role(&req),
            "UntagRole" => self.untag_role(&req),
            "ListRoleTags" => self.list_role_tags(&req),
            "AttachRolePolicy" => self.attach_role_policy(&req),
            "DetachRolePolicy" => self.detach_role_policy(&req),
            "ListAttachedRolePolicies" => self.list_attached_role_policies(&req),
            "PutRolePolicy" => self.put_role_policy(&req),
            "GetRolePolicy" => self.get_role_policy(&req),
            "DeleteRolePolicy" => self.delete_role_policy(&req),
            "ListRolePolicies" => self.list_role_policies(&req),

            // Groups
            "CreateGroup" => self.create_group(&req),
            "GetGroup" => self.get_group(&req),
            "DeleteGroup" => self.delete_group(&req),
            "ListGroups" => self.list_groups(&req),
            "UpdateGroup" => self.update_group(&req),
            "AddUserToGroup" => self.add_user_to_group(&req),
            "RemoveUserFromGroup" => self.remove_user_from_group(&req),
            "ListGroupsForUser" => self.list_groups_for_user(&req),
            "ListUsersForGroup" => self.list_users_for_group(&req),
            "TagGroup" => self.tag_group(&req),
            "UntagGroup" => self.untag_group(&req),
            "ListGroupTags" => self.list_group_tags(&req),
            "AttachGroupPolicy" => self.attach_group_policy(&req),
            "DetachGroupPolicy" => self.detach_group_policy(&req),
            "ListAttachedGroupPolicies" => self.list_attached_group_policies(&req),
            "PutGroupPolicy" => self.put_group_policy(&req),
            "GetGroupPolicy" => self.get_group_policy(&req),
            "DeleteGroupPolicy" => self.delete_group_policy(&req),
            "ListGroupPolicies" => self.list_group_policies(&req),

            // Policies
            "CreatePolicy" => self.create_policy(&req),
            "GetPolicy" => self.get_policy(&req),
            "DeletePolicy" => self.delete_policy(&req),
            "ListPolicies" => self.list_policies(&req),
            "CreatePolicyVersion" => self.create_policy_version(&req),
            "GetPolicyVersion" => self.get_policy_version(&req),
            "ListPolicyVersions" => self.list_policy_versions(&req),
            "SetDefaultPolicyVersion" => self.set_default_policy_version(&req),
            "DeletePolicyVersion" => self.delete_policy_version(&req),
            "TagPolicy" => self.tag_policy(&req),
            "UntagPolicy" => self.untag_policy(&req),
            "ListPolicyTags" => self.list_policy_tags(&req),

            // Account
            "CreateAccountAlias" => self.create_account_alias(&req),
            "DeleteAccountAlias" => self.delete_account_alias(&req),
            "ListAccountAliases" => self.list_account_aliases(&req),
            "GetAccountSummary" => self.get_account_summary(&req),
            "GetAccountPasswordPolicy" => self.get_account_password_policy(&req),
            "UpdateAccountPasswordPolicy" => self.update_account_password_policy(&req),
            "DeleteAccountPasswordPolicy" => self.delete_account_password_policy(&req),

            // Misc
            "GetAccessKeyLastUsed" => self.get_access_key_last_used(&req),
            "SimulateCustomPolicy" => self.simulate_custom_policy(&req),
            "SimulatePrincipalPolicy" => self.simulate_principal_policy(&req),
            // SAML/OIDC/MFA/InstanceProfile/SSH/Cert/LoginProfile/AuthDetails
            "CreateSAMLProvider" => self.xml_saml_create(&req),
            "GetSAMLProvider" => self.xml_saml_get(&req),
            "DeleteSAMLProvider" => self.xml_empty(&req, "DeleteSAMLProvider"),
            "ListSAMLProviders" => self.xml_saml_list(&req),
            "UpdateSAMLProvider" => self.xml_empty(&req, "UpdateSAMLProvider"),
            "CreateOpenIDConnectProvider" => self.xml_oidc_create(&req),
            "GetOpenIDConnectProvider" => self.xml_empty(&req, "GetOpenIDConnectProvider"),
            "DeleteOpenIDConnectProvider" => self.xml_empty(&req, "DeleteOpenIDConnectProvider"),
            "ListOpenIDConnectProviders" => self.xml_empty(&req, "ListOpenIDConnectProviders"),
            "UpdateOpenIDConnectProvider" => self.xml_empty(&req, "UpdateOpenIDConnectProvider"),
            "CreateVirtualMFADevice" => self.xml_mfa_create(&req),
            "ListVirtualMFADevices" => self.xml_mfa_list(&req),
            "TagVirtualMFADevice" => self.xml_empty(&req, "TagVirtualMFADevice"),
            "UntagVirtualMFADevice" => self.xml_empty(&req, "UntagVirtualMFADevice"),
            "ListVirtualMFADeviceTags" => self.xml_empty(&req, "ListVirtualMFADeviceTags"),
            "DeactivateMFADevice" => self.xml_empty(&req, "DeactivateMFADevice"),
            "EnableMFADevice" => self.xml_empty(&req, "EnableMFADevice"),
            "ListMFADevices" => self.xml_mfa_list(&req),
            "CreateInstanceProfile" => self.xml_ip_create(&req),
            "GetInstanceProfile" => self.xml_ip_get(&req),
            "DeleteInstanceProfile" => self.xml_empty(&req, "DeleteInstanceProfile"),
            "ListInstanceProfiles" => self.ip_list(&req),
            "AddRoleToInstanceProfile" => self.ip_add_role(&req),
            "RemoveRoleFromInstanceProfile" => self.ip_remove_role(&req),
            "TagInstanceProfile" => self.xml_empty(&req, "TagInstanceProfile"),
            "UntagInstanceProfile" => self.xml_empty(&req, "UntagInstanceProfile"),
            "ListInstanceProfileTags" => self.xml_empty(&req, "ListInstanceProfileTags"),
            "UploadSSHPublicKey" => self.xml_ssh_upload(&req),
            "ImportSSHPublicKey" => self.xml_ssh_upload(&req),
            "GetSSHPublicKey" => self.xml_ssh_get(&req),
            "UpdateSSHPublicKey" => self.xml_empty(&req, "UpdateSSHPublicKey"),
            "DeleteSSHPublicKey" => self.xml_empty(&req, "DeleteSSHPublicKey"),
            "ListSSHPublicKeys" => self.xml_ssh_list(&req),
            "UploadServerCertificate" => self.xml_cert_upload(&req),
            "GetServerCertificate" => self.xml_cert_get(&req),
            "DeleteServerCertificate" => self.xml_empty(&req, "DeleteServerCertificate"),
            "ListServerCertificates" => self.xml_cert_list(&req),
            "ListCertificates" => self.xml_empty(&req, "ListCertificates"),
            "CreateLoginProfile" => self.xml_login_create(&req),
            "GetLoginProfile" => self.xml_login_get(&req),
            "UpdateLoginProfile" => self.xml_empty(&req, "UpdateLoginProfile"),
            "DeleteLoginProfile" => self.xml_empty(&req, "DeleteLoginProfile"),
            "GetAccountAuthorizationDetails" => self.xml_empty(&req, "GetAccountAuthorizationDetails"),
            "GenerateCredentialReport" => self.xml_empty(&req, "GenerateCredentialReport"),
            "GetCredentialReport" => self.xml_empty(&req, "GetCredentialReport"),
            "GetContextKeyPolicy" => self.xml_empty(&req, "GetContextKeyPolicy"),
            "CreateContextKey" => self.xml_empty(&req, "CreateContextKey"),
            "DeleteContextKey" => self.xml_empty(&req, "DeleteContextKey"),
            "PutContextKey" => self.xml_empty(&req, "PutContextKey"),
            other => AwsResponse::error(400, "InvalidParameterValue",
                &format!("The operation {} is not implemented", other)),
        }
    }

    // ---- Users ----

    fn user_xml(&self, user: &User) -> String {
        format!(
            "<User>\
            <Path>{}</Path>\
            <UserName>{}</UserName>\
            <UserId>{}</UserId>\
            <Arn>{}</Arn>\
            <CreateDate>{}</CreateDate>\
            </User>",
            user.path, user.username, user.user_id, user.arn,
            chrono::Utc::now().to_rfc3339()
        )
    }

    /// Parse query protocol list params like Tags.member.1.Key, Tags.member.1.Value
    /// into a JSON array of objects.
    fn parse_query_list(&self, params: &serde_json::Value, prefix: &str) -> Vec<serde_json::Value> {
        let mut items: Vec<serde_json::Value> = Vec::new();
        let mut idx = 1;
        loop {
            let key_prefix = format!("{}.member.{}", prefix, idx);
            // Collect all fields for this item
            let mut item = serde_json::Map::new();
            let mut found = false;
            for (k, v) in params.as_object().unwrap_or(&serde_json::Map::new()) {
                if let Some(field) = k.strip_prefix(&format!("{}.{}", key_prefix, "")) {
                    // The key is like "Tags.member.1.Key" -> field = "Key"
                    // But the actual format is "Tags.member.1.Key" so prefix = "Tags.member.1"
                    // and the remainder is ".Key"
                    let field = field.strip_prefix('.').unwrap_or(field);
                    if !field.is_empty() {
                        item.insert(field.to_string(), v.clone());
                        found = true;
                    }
                }
            }
            if !found {
                break;
            }
            items.push(serde_json::Value::Object(item));
            idx += 1;
        }
        items
    }

    fn create_user(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        if username.is_empty() {
            return AwsResponse::error(400, "MissingParameterValue", "UserName is required");
        }
        let state = self.get_state(req.account);
        if state.get_user(&username).is_some() {
            return AwsResponse::error(409, "EntityAlreadyExists",
                &format!("User with name {} already exists.", username));
        }
        let user = Arc::new(User::new(req.account, username));
        // Handle Tags parameter
        let tags = self.parse_query_list(&req.params, "Tags");
        if !tags.is_empty() {
            *user.tags.write() = tags;
        }
        state.users.write().insert(user.username.clone(), user.clone());
        let mut body = self.user_xml(&user);
        // Include tags in response
        if !user.tags.read().is_empty() {
            body.push_str("<Tags>");
            for t in user.tags.read().iter() {
                body.push_str(&format!("<member><Key>{}</Key><Value>{}</Value></member>",
                    t.get("Key").and_then(|k| k.as_str()).unwrap_or(""),
                    t.get("Value").and_then(|v| v.as_str()).unwrap_or("")));
            }
            body.push_str("</Tags>");
        }
        AwsResponse::xml(200, "CreateUser", body)
    }

    fn get_user(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_user(&username) {
            Some(user) => AwsResponse::xml(200, "GetUser", self.user_xml(&user)),
            None => AwsResponse::error(404, "NoSuchEntity",
                &format!("The user with name {} cannot be found.", username)),
        }
    }

    fn delete_user(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_user(&username) {
            Some(user) => {
                // Check for attached policies
                if !user.attached_policies.read().is_empty() || !user.policies.read().is_empty() {
                    return AwsResponse::error(409, "DeleteConflict",
                        &format!("Cannot delete entity, must detach all policies first."));
                }
                state.users.write().remove(&username);
                AwsResponse::xml(200, "DeleteUser", String::new())
            }
            None => AwsResponse::error(404, "NoSuchEntity",
                &format!("The user with name {} cannot be found.", username)),
        }
    }

    fn list_users(&self, _req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(_req.account);
        let users = state.users.read().values().cloned().collect::<Vec<_>>();
        let mut body = String::from("<Users>");
        for user in users {
            body.push_str(&self.user_xml(&*user));
        }
        body.push_str("</Users>");
        AwsResponse::xml(200, "ListUsers", body)
    }

    fn update_user(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_user(&username) {
            Some(user) => {
                if let Some(new_name) = get_param(req, "NewUserName") {
                    state.users.write().remove(&user.username);
                    // Would need to update the user's name here - simplified
                }
                if let Some(new_path) = get_param(req, "NewPath") {
                    // Path update - simplified
                }
                AwsResponse::xml(200, "UpdateUser", String::new())
            }
            None => AwsResponse::error(404, "NoSuchEntity",
                &format!("The user with name {} cannot be found.", username)),
        }
    }

    fn change_password(&self, req: &AwsRequest) -> AwsResponse {
        let _old = get_param(req, "OldPassword").unwrap_or_default();
        let _new = get_param(req, "Password").unwrap_or_default();
        AwsResponse::xml(200, "ChangePassword", String::new())
    }

    fn create_access_key(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let state = self.get_state(req.account);
        let user = match state.get_user(&username) {
            Some(u) => u,
            None => return AwsResponse::error(404, "NoSuchEntity",
                &format!("The user with name {} cannot be found.", username)),
        };
        let key = AccessKey::new(req.account);
        let key_id = key.access_key_id.clone();
        let key_secret = key.secret_access_key.clone();
        user.access_keys.write().push(key);
        AwsResponse::xml(200, "CreateAccessKey", format!(
            "<AccessKey>\
            <AccessKeyId>{}</AccessKeyId>\
            <Status>Active</Status>\
            <SecretAccessKey>{}</SecretAccessKey>\
            <UserName>{}</UserName>\
            </AccessKey>",
            key_id, key_secret, username
        ))
    }

    fn delete_access_key(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let key_id = get_param(req, "AccessKeyId").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(user) = state.get_user(&username) {
            user.access_keys.write().retain(|k| k.access_key_id != key_id);
            return AwsResponse::xml(200, "DeleteAccessKey", String::new());
        }
        AwsResponse::error(404, "NoSuchEntity",
            &format!("The user with name {} cannot be found.", username))
    }

    fn list_access_keys(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_user(&username) {
            Some(user) => {
                let keys = user.access_keys.read();
                let mut body = String::from("<AccessKeyMetadata>");
                for key in keys.iter() {
                    body.push_str(&format!(
                        "<meta>\
                        <AccessKeyId>{}</AccessKeyId>\
                        <Status>{}</Status>\
                        <UserName>{}</UserName>\
                        </meta>",
                        key.access_key_id, key.status, username
                    ));
                }
                body.push_str("</AccessKeyMetadata>");
                AwsResponse::xml(200, "ListAccessKeys", body)
            }
            None => AwsResponse::error(404, "NoSuchEntity",
                &format!("The user with name {} cannot be found.", username)),
        }
    }

    fn update_access_key(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let key_id = get_param(req, "AccessKeyId").unwrap_or_default();
        let status = get_param(req, "Status").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(user) = state.get_user(&username) {
            let mut keys = user.access_keys.write();
            if let Some(k) = keys.iter_mut().find(|k| k.access_key_id == key_id) {
                k.status = status;
            }
            return AwsResponse::xml(200, "UpdateAccessKey", String::new());
        }
        AwsResponse::error(404, "NoSuchEntity", "User not found")
    }

    fn tag_user(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let tags = self.parse_query_list(&req.params, "Tags");
        let state = self.get_state(req.account);
        if let Some(user) = state.get_user(&username) {
            let mut existing = user.tags.write().clone();
            existing.extend(tags);
            *user.tags.write() = existing;
        }
        AwsResponse::xml(200, "TagUser", String::new())
    }

    fn untag_user(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let keys: Vec<String> = req.params.get("TagKeys")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(user) = state.get_user(&username) {
            user.tags.write().retain(|t| {
                t.get("Key").and_then(|k| k.as_str())
                    .map(|k| !keys.contains(&k.to_string()))
                    .unwrap_or(true)
            });
        }
        AwsResponse::xml(200, "UntagUser", String::new())
    }

    fn list_user_tags(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let state = self.get_state(req.account);
        let tags = state.get_user(&username)
            .map(|u| u.tags.read().clone())
            .unwrap_or_default();
        let mut body = String::from("<Tags>");
        for t in &tags {
            body.push_str(&format!("<member><Key>{}</Key><Value>{}</Value></member>",
                t.get("Key").and_then(|k| k.as_str()).unwrap_or(""),
                t.get("Value").and_then(|v| v.as_str()).unwrap_or("")));
        }
        body.push_str("</Tags>");
        AwsResponse::xml(200, "ListUserTags", body)
    }

    // ---- Role/Group/Policy attachments (simplified) ----

    fn attach_user_policy(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(user) = state.get_user(&username) {
            user.attached_policies.write().push(arn);
        }
        AwsResponse::xml(200, "AttachUserPolicy", String::new())
    }

    fn detach_user_policy(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(user) = state.get_user(&username) {
            user.attached_policies.write().retain(|a| a != &arn);
        }
        AwsResponse::xml(200, "DetachUserPolicy", String::new())
    }

    fn list_attached_user_policies(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let state = self.get_state(req.account);
        let policies = state.get_user(&username)
            .map(|u| u.attached_policies.read().clone())
            .unwrap_or_default();
        let mut body = String::from("<AttachedPolicies>");
        for p in &policies {
            body.push_str(&format!("<member><PolicyName>{}</PolicyName><PolicyArn>{}</PolicyArn></member>",
                p.rsplit('/').next().unwrap_or(""), p));
        }
        body.push_str("</AttachedPolicies>");
        AwsResponse::xml(200, "ListAttachedUserPolicies", body)
    }

    fn put_user_policy(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let policy_name = get_param(req, "PolicyName").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(user) = state.get_user(&username) {
            let mut policies = user.policies.write();
            if !policies.contains(&policy_name) {
                policies.push(policy_name);
            }
        }
        AwsResponse::xml(200, "PutUserPolicy", String::new())
    }

    fn get_user_policy(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let policy_name = get_param(req, "PolicyName").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_user(&username) {
            Some(user) if user.policies.read().contains(&policy_name) => {
                AwsResponse::xml(200, "GetUserPolicy", format!(
                    "<PolicyName>{}</PolicyName><PolicyDocument>{}{{}}{}</PolicyDocument>",
                    policy_name, "<base64>", "</base64>"
                ))
            }
            _ => AwsResponse::error(404, "NoSuchEntity", "Policy not found"),
        }
    }

    fn delete_user_policy(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let policy_name = get_param(req, "PolicyName").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(user) = state.get_user(&username) {
            user.policies.write().retain(|p| p != &policy_name);
        }
        AwsResponse::xml(200, "DeleteUserPolicy", String::new())
    }

    fn list_user_policies(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let state = self.get_state(req.account);
        let policies = state.get_user(&username)
            .map(|u| u.policies.read().clone())
            .unwrap_or_default();
        let mut body = String::from("<PolicyNames>");
        for p in &policies {
            body.push_str(&format!("<member>{}</member>", p));
        }
        body.push_str("</PolicyNames>");
        AwsResponse::xml(200, "ListUserPolicies", body)
    }

    // ---- Roles ----

    fn role_xml(&self, role: &Role) -> String {
        format!(
            "<Role>\
            <Path>{}</Path>\
            <RoleName>{}</RoleName>\
            <RoleId>{}</RoleId>\
            <Arn>{}</Arn>\
            <CreateDate>{}</CreateDate>\
            <AssumeRolePolicyDocument>{}</AssumeRolePolicyDocument>\
            <MaxSessionDuration>3600</MaxSessionDuration>\
            </Role>",
            role.path, role.role_name, role.role_id, role.arn,
            chrono::Utc::now().to_rfc3339(),
            role.assume_role_policy.read().as_str()
        )
    }

    fn create_role(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let assume_policy = get_param(req, "AssumeRolePolicyDocument").unwrap_or_default();
        if role_name.is_empty() {
            return AwsResponse::error(400, "MissingParameterValue", "RoleName is required");
        }
        let state = self.get_state(req.account);
        if state.get_role(&role_name).is_some() {
            return AwsResponse::error(409, "EntityAlreadyExists",
                &format!("Role with name {} already exists.", role_name));
        }
        let role = Arc::new(Role::new(req.account, role_name, assume_policy));
        let tags = self.parse_query_list(&req.params, "Tags");
        if !tags.is_empty() {
            *role.tags.write() = tags;
        }
        state.roles.write().insert(role.role_name.clone(), role.clone());
        let mut body = self.role_xml(&role);
        if !role.tags.read().is_empty() {
            body.push_str("<Tags>");
            for t in role.tags.read().iter() {
                body.push_str(&format!("<member><Key>{}</Key><Value>{}</Value></member>",
                    t.get("Key").and_then(|k| k.as_str()).unwrap_or(""),
                    t.get("Value").and_then(|v| v.as_str()).unwrap_or("")));
            }
            body.push_str("</Tags>");
        }
        AwsResponse::xml(200, "CreateRole", body)
    }

    fn get_role(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_role(&role_name) {
            Some(role) => AwsResponse::xml(200, "GetRole", self.role_xml(&role)),
            None => AwsResponse::error(404, "NoSuchEntity",
                &format!("The role with name {} cannot be found.", role_name)),
        }
    }

    fn delete_role(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_role(&role_name) {
            Some(role) => {
                if !role.attached_policies.read().is_empty() || !role.inline_policies.read().is_empty() {
                    return AwsResponse::error(409, "DeleteConflict",
                        "Cannot delete entity, must detach all policies first.");
                }
                state.roles.write().remove(&role_name);
                AwsResponse::xml(200, "DeleteRole", String::new())
            }
            None => AwsResponse::error(404, "NoSuchEntity",
                &format!("The role with name {} cannot be found.", role_name)),
        }
    }

    fn list_roles(&self, _req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(_req.account);
        let roles = state.roles.read().values().cloned().collect::<Vec<_>>();
        let mut body = String::from("<Roles>");
        for role in roles {
            body.push_str(&self.role_xml(&*role));
        }
        body.push_str("</Roles>");
        AwsResponse::xml(200, "ListRoles", body)
    }

    fn update_role(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "UpdateRole", String::new())
    }

    fn update_role_description(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let desc = get_param(req, "Description").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(role) = state.get_role(&role_name) {
            *role.description.write() = desc;
        }
        AwsResponse::xml(200, "UpdateRoleDescription", String::new())
    }

    fn update_assume_role_policy(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let policy = get_param(req, "PolicyDocument").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(role) = state.get_role(&role_name) {
            *role.assume_role_policy.write() = policy;
        }
        AwsResponse::xml(200, "UpdateAssumeRolePolicy", String::new())
    }

    fn tag_role(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let tags = self.parse_query_list(&req.params, "Tags");
        let state = self.get_state(req.account);
        if let Some(role) = state.get_role(&role_name) {
            let mut existing = role.tags.write().clone();
            existing.extend(tags);
            *role.tags.write() = existing;
        }
        AwsResponse::xml(200, "TagRole", String::new())
    }

    fn untag_role(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let keys: Vec<String> = req.params.get("TagKeys")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(role) = state.get_role(&role_name) {
            role.tags.write().retain(|t| {
                t.get("Key").and_then(|k| k.as_str())
                    .map(|k| !keys.contains(&k.to_string()))
                    .unwrap_or(true)
            });
        }
        AwsResponse::xml(200, "UntagRole", String::new())
    }

    fn list_role_tags(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let state = self.get_state(req.account);
        let tags = state.get_role(&role_name)
            .map(|r| r.tags.read().clone())
            .unwrap_or_default();
        let mut body = String::from("<Tags>");
        for t in &tags {
            body.push_str(&format!("<member><Key>{}</Key><Value>{}</Value></member>",
                t.get("Key").and_then(|k| k.as_str()).unwrap_or(""),
                t.get("Value").and_then(|v| v.as_str()).unwrap_or("")));
        }
        body.push_str("</Tags>");
        AwsResponse::xml(200, "ListRoleTags", body)
    }

    fn attach_role_policy(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(role) = state.get_role(&role_name) {
            role.attached_policies.write().push(arn);
        }
        AwsResponse::xml(200, "AttachRolePolicy", String::new())
    }

    fn detach_role_policy(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(role) = state.get_role(&role_name) {
            role.attached_policies.write().retain(|a| a != &arn);
        }
        AwsResponse::xml(200, "DetachRolePolicy", String::new())
    }

    fn list_attached_role_policies(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let state = self.get_state(req.account);
        let policies = state.get_role(&role_name)
            .map(|r| r.attached_policies.read().clone())
            .unwrap_or_default();
        let mut body = String::from("<AttachedPolicies>");
        for p in &policies {
            body.push_str(&format!("<member><PolicyName>{}</PolicyName><PolicyArn>{}</PolicyArn></member>",
                p.rsplit('/').next().unwrap_or(""), p));
        }
        body.push_str("</AttachedPolicies>");
        AwsResponse::xml(200, "ListAttachedRolePolicies", body)
    }

    fn put_role_policy(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let policy_name = get_param(req, "PolicyName").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(role) = state.get_role(&role_name) {
            let mut policies = role.inline_policies.write();
            if !policies.contains(&policy_name) {
                policies.push(policy_name);
            }
        }
        AwsResponse::xml(200, "PutRolePolicy", String::new())
    }

    fn get_role_policy(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let policy_name = get_param(req, "PolicyName").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_role(&role_name) {
            Some(role) if role.inline_policies.read().contains(&policy_name) => {
                AwsResponse::xml(200, "GetRolePolicy", format!(
                    "<PolicyName>{}</PolicyName><PolicyDocument>{{}}</PolicyDocument>",
                    policy_name
                ))
            }
            _ => AwsResponse::error(404, "NoSuchEntity", "Policy not found"),
        }
    }

    fn delete_role_policy(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let policy_name = get_param(req, "PolicyName").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(role) = state.get_role(&role_name) {
            role.inline_policies.write().retain(|p| p != &policy_name);
        }
        AwsResponse::xml(200, "DeleteRolePolicy", String::new())
    }

    fn list_role_policies(&self, req: &AwsRequest) -> AwsResponse {
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let state = self.get_state(req.account);
        let policies = state.get_role(&role_name)
            .map(|r| r.inline_policies.read().clone())
            .unwrap_or_default();
        let mut body = String::from("<PolicyNames>");
        for p in &policies {
            body.push_str(&format!("<member>{}</member>", p));
        }
        body.push_str("</PolicyNames>");
        AwsResponse::xml(200, "ListRolePolicies", body)
    }

    // ---- Groups ----

    fn group_xml(&self, group: &Group) -> String {
        format!(
            "<Group>\
            <Path>{}</Path>\
            <GroupName>{}</GroupName>\
            <GroupId>{}</GroupId>\
            <Arn>{}</Arn>\
            <CreateDate>{}</CreateDate>\
            </Group>",
            group.path, group.group_name, group.group_id, group.arn,
            chrono::Utc::now().to_rfc3339()
        )
    }

    fn create_group(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        if group_name.is_empty() {
            return AwsResponse::error(400, "MissingParameterValue", "GroupName is required");
        }
        let state = self.get_state(req.account);
        if state.get_group(&group_name).is_some() {
            return AwsResponse::error(409, "EntityAlreadyExists",
                &format!("A group with the name {} already exists.", group_name));
        }
        let group = Arc::new(Group::new(req.account, group_name));
        state.groups.write().insert(group.group_name.clone(), group.clone());
        AwsResponse::xml(200, "CreateGroup", self.group_xml(&group))
    }

    fn get_group(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_group(&group_name) {
            Some(group) => {
                let mut body = self.group_xml(&*group);
                // Include users in the group
                let users = group.users.read();
                if !users.is_empty() {
                    body.push_str("<Users>");
                    for user_name in users.iter() {
                        if let Some(user) = state.users.read().get(user_name) {
                            body.push_str(&self.user_xml(user));
                        }
                    }
                    body.push_str("</Users>");
                }
                AwsResponse::xml(200, "GetGroup", body)
            }
            None => AwsResponse::error(404, "NoSuchEntity",
                &format!("The group with name {} cannot be found.", group_name)),
        }
    }

    fn delete_group(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_group(&group_name) {
            Some(group) => {
                if !group.users.read().is_empty() {
                    return AwsResponse::error(409, "DeleteConflict",
                        "Cannot delete entity, must remove users first.");
                }
                state.groups.write().remove(&group_name);
                AwsResponse::xml(200, "DeleteGroup", String::new())
            }
            None => AwsResponse::error(404, "NoSuchEntity", "Group not found"),
        }
    }

    fn list_groups(&self, _req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(_req.account);
        let groups = state.groups.read().values().cloned().collect::<Vec<_>>();
        let mut body = String::from("<Groups>");
        for group in groups {
            body.push_str(&self.group_xml(&*group));
        }
        body.push_str("</Groups>");
        AwsResponse::xml(200, "ListGroups", body)
    }

    fn update_group(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "UpdateGroup", String::new())
    }

    fn add_user_to_group(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let username = get_param(req, "UserName").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(group) = state.get_group(&group_name) {
            group.users.write().push(username);
            return AwsResponse::xml(200, "AddUserToGroup", String::new());
        }
        AwsResponse::error(404, "NoSuchEntity", "Group not found")
    }

    fn remove_user_from_group(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let username = get_param(req, "UserName").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(group) = state.get_group(&group_name) {
            group.users.write().retain(|u| u != &username);
            return AwsResponse::xml(200, "RemoveUserFromGroup", String::new());
        }
        AwsResponse::error(404, "NoSuchEntity", "Group not found")
    }

    fn list_groups_for_user(&self, req: &AwsRequest) -> AwsResponse {
        let username = get_param(req, "UserName").unwrap_or_default();
        let state = self.get_state(req.account);
        let mut body = String::from("<Groups>");
        for group in state.groups.read().values() {
            if group.users.read().contains(&username) {
                body.push_str(&self.group_xml(&*group));
            }
        }
        body.push_str("</Groups>");
        AwsResponse::xml(200, "ListGroupsForUser", body)
    }

    fn list_users_for_group(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let state = self.get_state(req.account);
        let usernames = state.get_group(&group_name)
            .map(|g| g.users.read().clone())
            .unwrap_or_default();
        let mut body = String::from("<Users>");
        for uname in &usernames {
            if let Some(user) = state.get_user(uname) {
                body.push_str(&self.user_xml(&user));
            }
        }
        body.push_str("</Users>");
        AwsResponse::xml(200, "ListUsersForGroup", body)
    }

    fn tag_group(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let tags = self.parse_query_list(&req.params, "Tags");
        let state = self.get_state(req.account);
        if let Some(group) = state.get_group(&group_name) {
            let mut existing = group.tags.write().clone();
            existing.extend(tags);
            *group.tags.write() = existing;
        }
        AwsResponse::xml(200, "TagGroup", String::new())
    }

    fn untag_group(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let keys: Vec<String> = req.params.get("TagKeys")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(group) = state.get_group(&group_name) {
            group.tags.write().retain(|t| {
                t.get("Key").and_then(|k| k.as_str())
                    .map(|k| !keys.contains(&k.to_string()))
                    .unwrap_or(true)
            });
        }
        AwsResponse::xml(200, "UntagGroup", String::new())
    }

    fn list_group_tags(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let state = self.get_state(req.account);
        let tags = state.get_group(&group_name)
            .map(|g| g.tags.read().clone())
            .unwrap_or_default();
        let mut body = String::from("<Tags>");
        for t in &tags {
            body.push_str(&format!("<member><Key>{}</Key><Value>{}</Value></member>",
                t.get("Key").and_then(|k| k.as_str()).unwrap_or(""),
                t.get("Value").and_then(|v| v.as_str()).unwrap_or("")));
        }
        body.push_str("</Tags>");
        AwsResponse::xml(200, "ListGroupTags", body)
    }

    fn attach_group_policy(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(group) = state.get_group(&group_name) {
            group.attached_policies.write().push(arn);
        }
        AwsResponse::xml(200, "AttachGroupPolicy", String::new())
    }

    fn detach_group_policy(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(group) = state.get_group(&group_name) {
            group.attached_policies.write().retain(|a| a != &arn);
        }
        AwsResponse::xml(200, "DetachGroupPolicy", String::new())
    }

    fn list_attached_group_policies(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let state = self.get_state(req.account);
        let policies = state.get_group(&group_name)
            .map(|g| g.attached_policies.read().clone())
            .unwrap_or_default();
        let mut body = String::from("<AttachedPolicies>");
        for p in &policies {
            body.push_str(&format!("<member><PolicyName>{}</PolicyName><PolicyArn>{}</PolicyArn></member>",
                p.rsplit('/').next().unwrap_or(""), p));
        }
        body.push_str("</AttachedPolicies>");
        AwsResponse::xml(200, "ListAttachedGroupPolicies", body)
    }

    fn put_group_policy(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let policy_name = get_param(req, "PolicyName").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(group) = state.get_group(&group_name) {
            let mut policies = group.inline_policies.write();
            if !policies.contains(&policy_name) {
                policies.push(policy_name);
            }
        }
        AwsResponse::xml(200, "PutGroupPolicy", String::new())
    }

    fn get_group_policy(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let policy_name = get_param(req, "PolicyName").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_group(&group_name) {
            Some(group) if group.inline_policies.read().contains(&policy_name) => {
                AwsResponse::xml(200, "GetGroupPolicy", format!(
                    "<PolicyName>{}</PolicyName><PolicyDocument>{{}}</PolicyDocument>",
                    policy_name
                ))
            }
            _ => AwsResponse::error(404, "NoSuchEntity", "Policy not found"),
        }
    }

    fn delete_group_policy(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let policy_name = get_param(req, "PolicyName").unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(group) = state.get_group(&group_name) {
            group.inline_policies.write().retain(|p| p != &policy_name);
        }
        AwsResponse::xml(200, "DeleteGroupPolicy", String::new())
    }

    fn list_group_policies(&self, req: &AwsRequest) -> AwsResponse {
        let group_name = get_param(req, "GroupName").unwrap_or_default();
        let state = self.get_state(req.account);
        let policies = state.get_group(&group_name)
            .map(|g| g.inline_policies.read().clone())
            .unwrap_or_default();
        let mut body = String::from("<PolicyNames>");
        for p in &policies {
            body.push_str(&format!("<member>{}</member>", p));
        }
        body.push_str("</PolicyNames>");
        AwsResponse::xml(200, "ListGroupPolicies", body)
    }

    // ---- Policies ----

    fn create_policy(&self, req: &AwsRequest) -> AwsResponse {
        let policy_name = get_param(req, "PolicyName").unwrap_or_default();
        let document = get_param(req, "PolicyDocument").unwrap_or_default();
        if policy_name.is_empty() {
            return AwsResponse::error(400, "MissingParameterValue", "PolicyName is required");
        }
        let state = self.get_state(req.account);
        if state.get_policy(&policy_name).is_some() {
            return AwsResponse::error(409, "EntityAlreadyExists",
                &format!("A policy with the name {} already exists.", policy_name));
        }
        let policy = Arc::new(Policy::new(req.account, policy_name.clone(), document));
        state.policies.write().insert(policy.policy_name.clone(), policy.clone());
        AwsResponse::xml(200, "CreatePolicy", format!(
            "<Policy>\
            <PolicyName>{}</PolicyName>\
            <PolicyId>{}</PolicyId>\
            <Arn>{}</Arn>\
            <Path>{}</Path>\
            <DefaultVersionId>v1</DefaultVersionId>\
            <AttachmentCount>0</AttachmentCount>\
            <CreateDate>{}</CreateDate>\
            </Policy>",
            policy_name, policy.policy_id, policy.arn, policy.path,
            chrono::Utc::now().to_rfc3339()
        ))
    }

    fn get_policy(&self, req: &AwsRequest) -> AwsResponse {
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_policy(&arn) {
            Some(policy) => AwsResponse::xml(200, "GetPolicy", format!(
                "<Policy>\
                <PolicyName>{}</PolicyName>\
                <PolicyId>{}</PolicyId>\
                <Arn>{}</Arn>\
                <Path>{}</Path>\
                <DefaultVersionId>{}</DefaultVersionId>\
                <AttachmentCount>{}</AttachmentCount>\
                <CreateDate>{}</CreateDate>\
                </Policy>",
                policy.policy_name, policy.policy_id, policy.arn, policy.path,
                *policy.default_version_id.read(), policy.attachments,
                chrono::Utc::now().to_rfc3339()
            )),
            None => AwsResponse::error(404, "NoSuchEntity",
                &format!("The policy with ARN {} cannot be found.", arn)),
        }
    }

    fn delete_policy(&self, req: &AwsRequest) -> AwsResponse {
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let state = self.get_state(req.account);
        let policy = match state.get_policy(&arn) {
            Some(p) => p,
            None => return AwsResponse::error(404, "NoSuchEntity", "Policy not found"),
        };
        state.policies.write().remove(&policy.policy_name);
        AwsResponse::xml(200, "DeletePolicy", String::new())
    }

    fn list_policies(&self, _req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(_req.account);
        let policies = state.policies.read().values().cloned().collect::<Vec<_>>();
        let mut body = String::from("<Policies>");
        for p in policies {
            body.push_str(&format!(
                "<member>\
                <PolicyName>{}</PolicyName>\
                <PolicyId>{}</PolicyId>\
                <Arn>{}</Arn>\
                <Path>{}</Path>\
                <DefaultVersionId>{}</DefaultVersionId>\
                <AttachmentCount>{}</AttachmentCount>\
                </member>",
                p.policy_name, p.policy_id, p.arn, p.path,
                *p.default_version_id.read(), p.attachments
            ));
        }
        body.push_str("</Policies>");
        AwsResponse::xml(200, "ListPolicies", body)
    }

    fn create_policy_version(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "CreatePolicyVersion", String::new())
    }

    fn get_policy_version(&self, req: &AwsRequest) -> AwsResponse {
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_policy(&arn) {
            Some(policy) => AwsResponse::xml(200, "GetPolicyVersion", format!(
                "<PolicyVersion>\
                <VersionId>{}</VersionId>\
                <IsDefaultVersion>true</IsDefaultVersion>\
                <CreateDate>{}</CreateDate>\
                <Document>{{}}</Document>\
                </PolicyVersion>",
                *policy.default_version_id.read(),
                chrono::Utc::now().to_rfc3339()
            )),
            None => AwsResponse::error(404, "NoSuchEntity", "Policy not found"),
        }
    }

    fn list_policy_versions(&self, req: &AwsRequest) -> AwsResponse {
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let state = self.get_state(req.account);
        match state.get_policy(&arn) {
            Some(policy) => AwsResponse::xml(200, "ListPolicyVersions", format!(
                "<Versions>\
                <member>\
                <VersionId>{}</VersionId>\
                <IsDefaultVersion>true</IsDefaultVersion>\
                <CreateDate>{}</CreateDate>\
                </member>\
                </Versions>",
                *policy.default_version_id.read(),
                chrono::Utc::now().to_rfc3339()
            )),
            None => AwsResponse::error(404, "NoSuchEntity", "Policy not found"),
        }
    }

    fn set_default_policy_version(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "SetDefaultPolicyVersion", String::new())
    }

    fn delete_policy_version(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "DeletePolicyVersion", String::new())
    }

    fn tag_policy(&self, req: &AwsRequest) -> AwsResponse {
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let tags = self.parse_query_list(&req.params, "Tags");
        let state = self.get_state(req.account);
        if let Some(policy) = state.get_policy(&arn) {
            let mut existing = policy.tags.write().clone();
            existing.extend(tags);
            *policy.tags.write() = existing;
        }
        AwsResponse::xml(200, "TagPolicy", String::new())
    }

    fn untag_policy(&self, req: &AwsRequest) -> AwsResponse {
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let keys: Vec<String> = req.params.get("TagKeys")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account);
        if let Some(policy) = state.get_policy(&arn) {
            policy.tags.write().retain(|t| {
                t.get("Key").and_then(|k| k.as_str())
                    .map(|k| !keys.contains(&k.to_string()))
                    .unwrap_or(true)
            });
        }
        AwsResponse::xml(200, "UntagPolicy", String::new())
    }

    fn list_policy_tags(&self, req: &AwsRequest) -> AwsResponse {
        let arn = get_param(req, "PolicyArn").unwrap_or_default();
        let state = self.get_state(req.account);
        let tags = state.get_policy(&arn)
            .map(|p| p.tags.read().clone())
            .unwrap_or_default();
        let mut body = String::from("<Tags>");
        for t in &tags {
            body.push_str(&format!("<member><Key>{}</Key><Value>{}</Value></member>",
                t.get("Key").and_then(|k| k.as_str()).unwrap_or(""),
                t.get("Value").and_then(|v| v.as_str()).unwrap_or("")));
        }
        body.push_str("</Tags>");
        AwsResponse::xml(200, "ListPolicyTags", body)
    }

    // ---- Account ----

    fn create_account_alias(&self, req: &AwsRequest) -> AwsResponse {
        let alias = get_param(req, "AccountAlias").unwrap_or_default();
        let state = self.get_state(req.account);
        *state.account_alias.write() = Some(alias);
        AwsResponse::xml(200, "CreateAccountAlias", String::new())
    }

    fn delete_account_alias(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account);
        *state.account_alias.write() = None;
        AwsResponse::xml(200, "DeleteAccountAlias", String::new())
    }

    fn list_account_aliases(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account);
        let aliases = state.account_alias.read().clone().unwrap_or_default();
        let mut body = String::from("<AccountAliases>");
        for a in [aliases] {
            if !a.is_empty() {
                body.push_str(&format!("<member>{}</member>", a));
            }
        }
        body.push_str("</AccountAliases>");
        AwsResponse::xml(200, "ListAccountAliases", body)
    }

    fn get_account_summary(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account);
        let users = state.users.read().len();
        let roles = state.roles.read().len();
        let groups = state.groups.read().len();
        AwsResponse::xml(200, "GetAccountSummary", format!(
            "<SummaryMap>\
            <key>AccountMFAEnabled</key>\
            <value>0</value>\
            <key>AccountAccessKeysPresent</key>\
            <value>0</value>\
            <key>AccountSigningCertificatesPresent</key>\
            <value>0</value>\
            <key>GroupsQuota</key>\
            <value>100</value>\
            <key>Groups</key>\
            <value>{}</value>\
            <key>UsersQuota</key>\
            <value>5000</value>\
            <key>Users</key>\
            <value>{}</value>\
            <key>RolesQuota</key>\
            <value>1000</value>\
            <key>Roles</key>\
            <value>{}</value>\
            <key>GlobalEndpointUseEnabled</key>\
            <value>Disabled</value>\
            </SummaryMap>",
            groups, users, roles
        ))
    }

    fn get_account_password_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "GetAccountPasswordPolicy", String::new())
    }

    fn update_account_password_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "UpdateAccountPasswordPolicy", String::new())
    }

    fn delete_account_password_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "DeleteAccountPasswordPolicy", String::new())
    }

    // ---- Misc ----

    fn get_access_key_last_used(&self, req: &AwsRequest) -> AwsResponse {
        let key_id = get_param(req, "AccessKeyId").unwrap_or_default();
        let username = get_param(req, "UserName").unwrap_or_default();
        AwsResponse::xml(200, "GetAccessKeyLastUsed", format!(
            "<AccessKeyLastUsed>\
            <AccessKeyId>{}</AccessKeyId>\
            <UserName>{}</UserName>\
            <LastUsedDate>1970-01-01T00:00:00Z</LastUsedDate>\
            <LastUsedRegion>us-east-1</LastUsedRegion>\
            <ServiceName>not used</ServiceName>\
            </AccessKeyLastUsed>",
            key_id, username
        ))
    }

    fn simulate_custom_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "SimulateCustomPolicy", format!(
            "<EvaluationResults>\
            <member>\
            <EvalDecision>allowed</EvalDecision>\
            <MatchedStatements/>\
            <EvalResourceName>*</EvalResourceName>\
            </member>\
            </EvaluationResults>"
        ))
    }

    fn simulate_principal_policy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "SimulatePrincipalPolicy", format!(
            "<EvaluationResults>\
            <member>\
            <EvalDecision>allowed</EvalDecision>\
            <MatchedStatements/>\
            <EvalResourceName>*</EvalResourceName>\
            </member>\
            </EvaluationResults>"
        ))
    }
    fn xml_saml_create(&self, req: &AwsRequest) -> AwsResponse {
        let name = get_param(req, "SAMLProviderName").unwrap_or_else(|| "unknown".into());
        AwsResponse::xml(200, "CreateSAMLProvider", format!(
            "<CreateSAMLProviderResult><SAMLProviderArn>arn:aws:iam::{}:saml-provider/{}</SAMLProviderArn></CreateSAMLProviderResult>",
            req.account, name
        ))
    }
    fn xml_saml_list(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "ListSAMLProviders",
            "<ListSAMLProvidersResult><SAMLProviderList/></ListSAMLProvidersResult>".into())
    }
    fn xml_mfa_list(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "ListMFADevices",
            "<ListMFADevicesResult><MFADevices/></ListMFADevicesResult>".into())
    }
    fn xml_ip_list(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "ListInstanceProfiles",
            "<ListInstanceProfilesResult><InstanceProfiles/></ListInstanceProfilesResult>".into())
    }
    fn xml_ssh_list(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "ListSSHPublicKeys",
            "<ListSSHPublicKeysResult><SSHPublicKeys/></ListSSHPublicKeysResult>".into())
    }
    fn xml_cert_list(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "ListServerCertificates",
            "<ListServerCertificatesResult><ServerCertificates/></ListServerCertificatesResult>".into())
    }
    fn xml_login_get(&self, req: &AwsRequest) -> AwsResponse {
        let user = get_param(req, "UserName").unwrap_or_else(|| "unknown".into());
        AwsResponse::xml(200, "GetLoginProfile", format!(
            "<GetLoginProfileResult><LoginProfile><UserName>{}</UserName><PasswordLastUsed>2024-01-01T00:00:00Z</PasswordLastUsed><PasswordResetRequired>false</PasswordResetRequired></LoginProfile></GetLoginProfileResult>",
            user
        ))
    }

    // ---- Stub operations (return empty XML for compatibility) ----
    fn xml_empty(&self, req: &AwsRequest, op: &str) -> AwsResponse {
        let root = format!("{}Result", op);
        AwsResponse::xml(200, op, format!("<{}/>", root))
    }
    fn xml_saml_get(&self, req: &AwsRequest) -> AwsResponse {
        let name = get_param(req, "SAMLProviderName").unwrap_or_else(|| "unknown".into());
        AwsResponse::xml(200, "GetSAMLProvider", format!(
            "<GetSAMLProviderResult><SAMLProviderArn>arn:aws:iam::{}:saml-provider/{}</SAMLProviderArn></GetSAMLProviderResult>",
            req.account, name
        ))
    }
    fn xml_oidc_create(&self, req: &AwsRequest) -> AwsResponse {
        let url = get_param(req, "Url").unwrap_or_else(|| "https://unknown".into());
        AwsResponse::xml(200, "CreateOpenIDConnectProvider", format!(
            "<CreateOpenIDConnectProviderResult><OpenIDConnectProviderArn>arn:aws:iam::{}:oidc-provider/{}</OpenIDConnectProviderArn></CreateOpenIDConnectProviderResult>",
            req.account, url
        ))
    }
    fn xml_mfa_create(&self, req: &AwsRequest) -> AwsResponse {
        let name = get_param(req, "VirtualMFADeviceName").unwrap_or_else(|| "unknown".into());
        AwsResponse::xml(200, "CreateVirtualMFADevice", format!(
            "<CreateVirtualMFADeviceResult><VirtualMFADevice><SerialNumber>arn:aws:iam::{}:mfa/{}</SerialNumber><VirtualMFADeviceName>{}</VirtualMFADeviceName></VirtualMFADevice><Base32StringSeed>ABCDEF123456</Base32StringSeed><QRCodePNGBase64>iVBORw0KGgo=</QRCodePNGBase64></CreateVirtualMFADeviceResult>",
            req.account, name, name
        ))
    }
    fn xml_ip_create(&self, req: &AwsRequest) -> AwsResponse {
        let name = get_param(req, "InstanceProfileName").unwrap_or_else(|| "unknown".into());
        let state = self.get_state(req.account);
        state.create_instance_profile(&name);
        AwsResponse::xml(200, "CreateInstanceProfile", format!(
            "<CreateInstanceProfileResult><InstanceProfile><InstanceProfileName>{}</InstanceProfileName><InstanceProfileArn>arn:aws:iam::{}:instance-profile/{}</InstanceProfileArn><Path>/</Path><Roles/></InstanceProfile></CreateInstanceProfileResult>",
            name, req.account, name
        ))
    }
    fn xml_ip_get(&self, req: &AwsRequest) -> AwsResponse {
        let name = get_param(req, "InstanceProfileName").unwrap_or_else(|| "unknown".into());
        let state = self.get_state(req.account);
        if state.instance_profiles.read().contains_key(&name) {
            let roles = state.instance_profiles.read().get(&name).cloned().unwrap_or_default();
            let mut roles_xml = String::new();
            for role_name in &roles {
                if let Some(role) = state.get_role(role_name) {
                    roles_xml.push_str(&self.role_xml(&*role));
                }
            }
            AwsResponse::xml(200, "GetInstanceProfile", format!(
                "<GetInstanceProfileResult><InstanceProfile><InstanceProfileName>{}</InstanceProfileName><InstanceProfileArn>arn:aws:iam::{}:instance-profile/{}</InstanceProfileArn><Path>/</Path><Roles>{}</Roles></InstanceProfile></GetInstanceProfileResult>",
                name, req.account, name, roles_xml
            ))
        } else {
            AwsResponse::error(404, "NoSuchEntity", &format!("The instance profile with name {} cannot be found.", name))
        }
    }
    fn ip_add_role(&self, req: &AwsRequest) -> AwsResponse {
        let name = get_param(req, "InstanceProfileName").unwrap_or_default();
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let state = self.get_state(req.account);
        if !state.instance_profiles.read().contains_key(&name) {
            return AwsResponse::error(404, "NoSuchEntity", &format!("Instance profile {} not found.", name));
        }
        if state.get_role(&role_name).is_none() {
            return AwsResponse::error(404, "NoSuchEntity", &format!("Role {} not found.", role_name));
        }
        let mut profiles = state.instance_profiles.write();
        if let Some(roles) = profiles.get_mut(&name) {
            if !roles.contains(&role_name) {
                roles.push(role_name);
            }
        }
        AwsResponse::xml(200, "AddRoleToInstanceProfile", String::new())
    }

    fn ip_remove_role(&self, req: &AwsRequest) -> AwsResponse {
        let name = get_param(req, "InstanceProfileName").unwrap_or_default();
        let role_name = get_param(req, "RoleName").unwrap_or_default();
        let state = self.get_state(req.account);
        let mut profiles = state.instance_profiles.write();
        if let Some(roles) = profiles.get_mut(&name) {
            roles.retain(|r| r != &role_name);
        }
        AwsResponse::xml(200, "RemoveRoleFromInstanceProfile", String::new())
    }

    fn ip_list(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account);
        let profiles = state.list_instance_profiles();
        let mut body = String::new();
        for p in &profiles {
            let name = p.get("InstanceProfileName").and_then(|n| n.as_str()).unwrap_or("");
            let arn = p.get("InstanceProfileArn").and_then(|a| a.as_str()).unwrap_or("");
            let roles = p.get("Roles").and_then(|r| r.as_array()).cloned().unwrap_or_default();
            let mut roles_xml = String::new();
            for role_name in roles.iter().filter_map(|r| r.as_str()) {
                if let Some(role) = state.get_role(role_name) {
                    roles_xml.push_str(&self.role_xml(&*role));
                }
            }
            body.push_str(&format!(
                "<InstanceProfile><InstanceProfileName>{}</InstanceProfileName><InstanceProfileArn>{}</InstanceProfileArn><Path>/</Path><Roles>{}</Roles></InstanceProfile>",
                name, arn, roles_xml
            ));
        }
        AwsResponse::xml(200, "ListInstanceProfiles", format!("<InstanceProfiles>{}</InstanceProfiles>", body))
    }

    fn xml_ssh_upload(&self, req: &AwsRequest) -> AwsResponse {
        let name = get_param(req, "SSHPublicKeyName").unwrap_or_else(|| "unknown".into());
        let user = get_param(req, "UserName").unwrap_or_else(|| "unknown".into());
        AwsResponse::xml(200, "UploadSSHPublicKey", format!(
            "<UploadSSHPublicKeyResult><SSHPublicKey><SSHPublicKeyId>{}-{}</SSHPublicKeyId><SSHPublicKeyName>{}</SSHPublicKeyName><UserName>{}</UserName><Status>Active</Status></SSHPublicKey></UploadSSHPublicKeyResult>",
            user, name, name, user
        ))
    }
    fn xml_ssh_get(&self, req: &AwsRequest) -> AwsResponse {
        let name = get_param(req, "SSHPublicKeyName").unwrap_or_else(|| "unknown".into());
        let user = get_param(req, "UserName").unwrap_or_else(|| "unknown".into());
        AwsResponse::xml(200, "GetSSHPublicKey", format!(
            "<GetSSHPublicKeyResult><SSHPublicKey><SSHPublicKeyId>{}-{}</SSHPublicKeyId><SSHPublicKeyName>{}</SSHPublicKeyName><UserName>{}</UserName><Status>Active</Status><SSHPublicKeyBody>ssh-rsa AAAAB3NzaC1yc2E=</SSHPublicKeyBody></SSHPublicKey></GetSSHPublicKeyResult>",
            user, name, name, user
        ))
    }
    fn xml_cert_upload(&self, req: &AwsRequest) -> AwsResponse {
        let name = get_param(req, "ServerCertificateName").unwrap_or_else(|| "unknown".into());
        AwsResponse::xml(200, "UploadServerCertificate", format!(
            "<UploadServerCertificateResult><ServerCertificateMetadata><ServerCertificateName>{}</ServerCertificateName><ServerCertificateId>cert-{}</ServerCertificateId><Arn>arn:aws:iam::{}:server-certificate/{}</Arn></ServerCertificateMetadata></UploadServerCertificateResult>",
            name, name, req.account, name
        ))
    }
    fn xml_cert_get(&self, req: &AwsRequest) -> AwsResponse {
        let name = get_param(req, "ServerCertificateName").unwrap_or_else(|| "unknown".into());
        AwsResponse::xml(200, "GetServerCertificate", format!(
            "<GetServerCertificateResult><ServerCertificateMetadata><ServerCertificateName>{}</ServerCertificateName><ServerCertificateId>cert-{}</ServerCertificateId><Arn>arn:aws:iam::{}:server-certificate/{}</Arn></ServerCertificateMetadata></GetServerCertificateResult>",
            name, name, req.account, name
        ))
    }
    fn xml_login_create(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::xml(200, "CreateLoginProfile",
            "<CreateLoginProfileResult><Password>TempPassword123!</Password><PasswordResetRequired>true</PasswordResetRequired></CreateLoginProfileResult>".into())
    }

}

impl Default for IamHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "iam".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
            query: String::new(),
        }
    }

    #[test]
    fn test_create_and_get_user() {
        let handler = IamHandler::new();
        handler.handle(make_req("CreateUser", json!({ "UserName": "testuser" })));
        let resp = handler.handle(make_req("GetUser", json!({ "UserName": "testuser" })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("testuser"));
    }

    #[test]
    fn test_create_and_list_roles() {
        let handler = IamHandler::new();
        handler.handle(make_req("CreateRole", json!({
            "RoleName": "test-role",
            "AssumeRolePolicyDocument": "{}"
        })));
        let resp = handler.handle(make_req("ListRoles", json!({})));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("test-role"));
    }

    #[test]
    fn test_create_and_list_policies() {
        let handler = IamHandler::new();
        handler.handle(make_req("CreatePolicy", json!({
            "PolicyName": "my-policy",
            "PolicyDocument": "{}"
        })));
        let resp = handler.handle(make_req("ListPolicies", json!({})));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("my-policy"));
    }

    #[test]
    fn test_access_keys() {
        let handler = IamHandler::new();
        handler.handle(make_req("CreateUser", json!({ "UserName": "keyuser" })));
        let resp = handler.handle(make_req("CreateAccessKey", json!({ "UserName": "keyuser" })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("AccessKeyId"));

        let resp = handler.handle(make_req("ListAccessKeys", json!({ "UserName": "keyuser" })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("keyuser"));
    }

    #[test]
    fn test_groups() {
        let handler = IamHandler::new();
        handler.handle(make_req("CreateUser", json!({ "UserName": "guser" })));
        handler.handle(make_req("CreateGroup", json!({ "GroupName": "devs" })));
        handler.handle(make_req("AddUserToGroup", json!({
            "GroupName": "devs", "UserName": "guser"
        })));
        let resp = handler.handle(make_req("ListUsersForGroup", json!({ "GroupName": "devs" })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("guser"));
    }
}
