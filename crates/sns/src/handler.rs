//! SNS operation handler.

use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::{SnsState, Subscription, Topic};
use crate::protocol::{AwsRequest, AwsResponse};

/// The SNS service handler.
pub struct SnsHandler {
    state: RwLock<HashMap<(u64, String), SnsState>>,
}

impl SnsHandler {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(HashMap::new()),
        }
    }

    fn get_state(&self, account: u64, region: &str) -> SnsState {
        let mut states = self.state.write();
        states
            .entry((account, region.to_string()))
            .or_insert_with(SnsState::new)
            .clone()
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let operation = req.operation.as_str();
        match operation {
            "CreateTopic" => self.create_topic(&req),
            "DeleteTopic" => self.delete_topic(&req),
            "GetTopicAttributes" => self.get_topic_attributes(&req),
            "SetTopicAttributes" => self.set_topic_attributes(&req),
            "ListTopics" => self.list_topics(&req),
            "Publish" => self.publish(&req),
            "Subscribe" => self.subscribe(&req),
            "Unsubscribe" => self.unsubscribe(&req),
            "ListSubscriptions" => self.list_subscriptions(&req),
            "ListSubscriptionsByTopic" => self.list_subscriptions_by_topic(&req),
            "GetSubscriptionAttributes" => self.get_subscription_attributes(&req),
            "SetSubscriptionAttributes" => self.set_subscription_attributes(&req),
                "TagResource" => self.tag_resource(&req),
    "UntagResource" => self.untag_resource(&req),
    "ListTagsForResource" => self.list_tags_for_resource(&req),
    "PublishBatch" => AwsResponse::query_success("PublishBatch", "<Successful/><Failed/>".to_string()),
    "CreatePlatformApplication" => self.create_platform_application(&req),
    "ListPlatformApplications" => self.list_platform_applications(&req),
    "DeletePlatformApplication" => self.delete_platform_application(&req),
    "GetPlatformApplication" => self.get_platform_application(&req),
    "UpdatePlatformApplication" => self.update_platform_application(&req),
other => AwsResponse::error(
                400,
                "InvalidAction",
                &format!("The action {} is not supported", other),
            ),
        }
    }

    fn create_topic(&self, req: &AwsRequest) -> AwsResponse {
        let name = req
            .params
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if name.is_empty() {
            return AwsResponse::error(400, "InvalidParameter", "Topic name cannot be empty");
        }

        let fifo = name.ends_with(".fifo");
        let topic = Arc::new(Topic::new(name.clone(), req.account, req.region.clone(), fifo));
        let state = self.get_state(req.account, &req.region);
        state.put_topic(topic.clone());

        let body = format!(
            "<TopicArn>{}</TopicArn>",
            topic.arn
        );
        AwsResponse::query_success("CreateTopic", body)
    }

    fn delete_topic(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req
            .params
            .get("TopicArn")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let state = self.get_state(req.account, &req.region);
        if state.get_topic(&arn).is_none() {
            return AwsResponse::error(
                400,
                "InvalidParameter",
                "Topic arn does not exist in discoveries",
            );
        }

        state.delete_topic(&arn);
        let body = String::new();
        AwsResponse::query_success("DeleteTopic", body)
    }

    fn get_topic_attributes(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req
            .params
            .get("TopicArn")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let state = self.get_state(req.account, &req.region);
        let topic = match state.get_topic(&arn) {
            Some(t) => t,
            None => {
                return AwsResponse::error(
                    400,
                    "InvalidParameter",
                    "Topic arn does not exist in discoveries",
                );
            }
        };

        let mut body = String::from("<Attributes>");
        body.push_str(&format!("<entry><key>TopicArn</key><value>{}</value></entry>", topic.arn));
        body.push_str(&format!("<entry><key>Owner</key><value>{}</value></entry>", topic.owner));
        if let Some(dn) = topic.display_name.read().as_ref() {
            body.push_str(&format!("<entry><key>DisplayName</key><value>{}</value></entry>", dn));
        } else {
            body.push_str(&format!("<entry><key>DisplayName</key><value>{}</value></entry>", topic.name));
        }
        if let Some(policy) = topic.policy.read().as_ref() {
            body.push_str(&format!("<entry><key>Policy</key><value>{}</value></entry>", policy));
        }
        body.push_str(&format!("<entry><key>SubscriptionsConfirmed</key><value>0</value></entry>"));
        body.push_str(&format!("<entry><key>SubscriptionsPending</key><value>0</value></entry>"));
        body.push_str(&format!("<entry><key>SubscriptionsDeleted</key><value>0</value></entry>"));
        body.push_str("</Attributes>");

        AwsResponse::query_success("GetTopicAttributes", body)
    }

    fn set_topic_attributes(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req
            .params
            .get("TopicArn")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let state = self.get_state(req.account, &req.region);
        let topic = match state.get_topic(&arn) {
            Some(t) => t,
            None => {
                return AwsResponse::error(
                    400,
                    "InvalidParameter",
                    "Topic arn does not exist in discoveries",
                );
            }
        };

        let attr_name = req.params.get("AttributeName").and_then(|v| v.as_str()).unwrap_or("");
        let attr_value = req.params.get("AttributeValue").and_then(|v| v.as_str()).unwrap_or("");
        // URL-decode the attribute value (query protocol encodes spaces as +)
        let attr_value = attr_value.replace('+', " ");
        if attr_name == "DisplayName" && !attr_value.is_empty() {
            *topic.display_name.write() = Some(attr_value.to_string());
        }
        if attr_name == "Policy" && !attr_value.is_empty() {
            *topic.policy.write() = Some(attr_value.to_string());
        }

        let body = String::new();
        AwsResponse::query_success("SetTopicAttributes", body)
    }

    fn list_topics(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let topics = state.list_topics();

        let mut body = String::from("<Topics>");
        for topic in &topics {
            body.push_str(&format!("<member><TopicArn>{}</TopicArn></member>", topic.arn));
        }
        body.push_str("</Topics>");

        AwsResponse::query_success("ListTopics", body)
    }

    fn publish(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req
            .params
            .get("TopicArn")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let state = self.get_state(req.account, &req.region);
        if state.get_topic(&arn).is_none() {
            return AwsResponse::error(
                400,
                "InvalidParameter",
                "Topic arn does not exist in discoveries",
            );
        }

        let message_id = uuid::Uuid::new_v4().to_string();
        let body = format!("<MessageId>{}</MessageId>", message_id);
        AwsResponse::query_success("Publish", body)
    }

    fn subscribe(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req
            .params
            .get("TopicArn")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let state = self.get_state(req.account, &req.region);
        let topic = match state.get_topic(&arn) {
            Some(t) => t,
            None => {
                return AwsResponse::error(
                    400,
                    "InvalidParameter",
                    "Topic arn does not exist in discoveries",
                );
            }
        };

        let protocol = req
            .params
            .get("Protocol")
            .and_then(|v| v.as_str())
            .unwrap_or("email")
            .to_string();
        let endpoint = req
            .params
            .get("Endpoint")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let subscription_arn = format!("arn:aws:sns:{}:{}:{}:{}:{}:00000000-0000-0000-0000-000000000000",
            req.region, req.account, arn.rsplit(':').next().unwrap_or(""), protocol, uuid::Uuid::new_v4());

        let sub = Subscription {
            subscription_arn: subscription_arn.clone(),
            topic_arn: arn,
            protocol,
            endpoint,
            owner: req.account,
            confirmed: true,
            created: chrono::Utc::now().timestamp() as u64,
        };

        state.add_subscription(sub);
        let body = format!("<SubscriptionArn>{}</SubscriptionArn>", subscription_arn);
        AwsResponse::query_success("Subscribe", body)
    }

    fn unsubscribe(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req
            .params
            .get("SubscriptionArn")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if arn.is_empty() {
            return AwsResponse::error(400, "InvalidParameter", "Subscription arn cannot be empty");
        }

        let state = self.get_state(req.account, &req.region);
        state.remove_subscription(&arn);
        let body = String::new();
        AwsResponse::query_success("Unsubscribe", body)
    }

    fn list_subscriptions(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let subs = state.list_subscriptions();
        let mut body = String::from("<Subscriptions>");
        for sub in &subs {
            body.push_str(&format!(
                "<member><SubscriptionArn>{}</SubscriptionArn><TopicArn>{}</TopicArn><Endpoint>{}</Endpoint><Protocol>{}</Protocol><Owner>{}</Owner><RawMessageDelivery>false</RawMessageDelivery></member>",
                sub.subscription_arn, sub.topic_arn, sub.endpoint, sub.protocol, sub.owner
            ));
        }
        body.push_str("</Subscriptions>");
        AwsResponse::query_success("ListSubscriptions", body)
    }

    fn list_subscriptions_by_topic(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req
            .params
            .get("TopicArn")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let state = self.get_state(req.account, &req.region);
        let _topic = match state.get_topic(&arn) {
            Some(t) => t,
            None => {
                return AwsResponse::error(
                    400,
                    "InvalidParameter",
                    "Topic arn does not exist in discoveries",
                );
            }
        };

        let subs = state.list_subscriptions();
        let mut body = String::from("<Subscriptions>");
        for sub in &subs {
            if sub.topic_arn == arn {
                body.push_str(&format!(
                    "<member><SubscriptionArn>{}</SubscriptionArn><TopicArn>{}</TopicArn><Endpoint>{}</Endpoint><Protocol>{}</Protocol><Owner>{}</Owner><RawMessageDelivery>false</RawMessageDelivery></member>",
                    sub.subscription_arn, sub.topic_arn, sub.endpoint, sub.protocol, sub.owner
                ));
            }
        }
        body.push_str("</Subscriptions>");
        AwsResponse::query_success("ListSubscriptionsByTopic", body)
    }

    fn get_subscription_attributes(&self, req: &AwsRequest) -> AwsResponse {
        let sub_arn = req.params.get("SubscriptionArn")
            .and_then(|v| v.as_str()).unwrap_or("");
        let state = self.get_state(req.account, &req.region);
        let subs = state.list_subscriptions();
        if let Some(sub) = subs.iter().find(|s| s.subscription_arn == sub_arn) {
            let body = format!(
                "<Attributes><entry><key>SubscriptionArn</key><value>{}</value></entry><entry><key>TopicArn</key><value>{}</value></entry><entry><key>Protocol</key><value>{}</value></entry><entry><key>Endpoint</key><value>{}</value></entry><entry><key>RawMessageDelivery</key><value>false</value></entry><entry><key>Owner</key><value>{}</value></entry></Attributes>",
                sub.subscription_arn, sub.topic_arn, sub.protocol, sub.endpoint, req.account
            );
            AwsResponse::query_success("GetSubscriptionAttributes", body)
        } else {
            AwsResponse::error(400, "InvalidParameter",
                "Subscription arn does not exist")
        }
    }

    fn set_subscription_attributes(&self, _req: &AwsRequest) -> AwsResponse {
        let body = String::new();
        AwsResponse::query_success("SetSubscriptionAttributes", body)
    }

    fn tag_resource(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("ResourceArn").and_then(|v| v.as_str()).unwrap_or_default();

        // Parse tags from query protocol format (Tags.member.N.Key/Value)
        let tags: Vec<(String, String)> = {
            let mut result = vec![];
            let mut i = 1;
            loop {
                let key_name = format!("Tags.member.{}.Key", i);
                let val_name = format!("Tags.member.{}.Value", i);
                match (req.params.get(&key_name), req.params.get(&val_name)) {
                    (Some(k), Some(v)) => {
                        result.push((k.as_str().unwrap_or("").to_string(), v.as_str().unwrap_or("").to_string()));
                        i += 1;
                    }
                    _ => break,
                }
            }
            result
        };
        let state = self.get_state(req.account, &req.region);
        if let Some(topic) = state.topics.read().get(arn).cloned() {
            let mut existing = topic.tags.write();
            for (key, val) in &tags {
                if let Some(e) = existing.iter_mut().find(|(k, _)| *k == *key) {
                    e.1 = val.clone();
                } else {
                    existing.push((key.clone(), val.clone()));
                }
            }
        }
        AwsResponse::query_success("TagResource", String::new())
    }

    fn untag_resource(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("ResourceArn").and_then(|v| v.as_str()).unwrap_or_default();
        let keys: Vec<String> = req.params.get("TagKeys")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|k| k.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        if let Some(topic) = state.topics.read().get(arn).cloned() {
            let mut existing = topic.tags.write();
            existing.retain(|(k, _)| !keys.contains(&k.to_string()));
        }
        AwsResponse::query_success("UntagResource", String::new())
    }

    fn list_tags_for_resource(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("ResourceArn").and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let tags = state.get_topic(arn).map(|t| t.tags.read().clone()).unwrap_or_default();
        let mut xml = String::new();
        xml.push_str("<Tags>");
        for (key, val) in &tags {
            xml.push_str(&format!("<member><Key>{}</Key><Value>{}</Value></member>", key, val));
        }
        xml.push_str("</Tags>");
        AwsResponse::query_success("ListTagsForResource", xml)
    }

    fn create_platform_application(&self, req: &AwsRequest) -> AwsResponse {
        let platform = req.params.get("Platform")
            .and_then(|v| v.as_str()).unwrap_or("custom");
        let name = req.params.get("Name")
            .and_then(|v| v.as_str()).unwrap_or("app");
        let arn = format!("arn:aws:sns:{}:{}:application:{}:{}", req.region, req.account, platform, name);
        // Store the application (simplified - just return the ARN)
        AwsResponse::query_success("CreatePlatformApplication", format!("<PlatformApplicationArn>{}</PlatformApplicationArn>", arn))
    }

    fn list_platform_applications(&self, _req: &AwsRequest) -> AwsResponse {
        // Return empty list (no storage implemented)
        AwsResponse::query_success("ListPlatformApplications", "<PlatformApplications/>".to_string())
    }

    fn delete_platform_application(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::query_success("DeletePlatformApplication", String::new())
    }

    fn get_platform_application(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("PlatformApplicationArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        AwsResponse::query_success("GetPlatformApplication", format!(
            "<PlatformApplication><PlatformApplicationArn>{}</PlatformApplicationArn></PlatformApplication>",
            arn
        ))
    }

    fn update_platform_application(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::query_success("UpdatePlatformApplication", String::new())
    }
}

impl Default for SnsHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "sns".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_create_and_list_topics() {
        let handler = SnsHandler::new();

        let req = make_req("CreateTopic", json!({"Name": "test-topic"}));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("test-topic"));

        let req = make_req("ListTopics", json!({}));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("test-topic"));
    }

    #[test]
    fn test_publish_to_nonexistent_topic() {
        let handler = SnsHandler::new();
        let req = make_req("Publish", json!({
            "TopicArn": "arn:aws:sns:us-east-1:123456789012:nonexistent",
            "Message": "hello"
        }));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 400);
        assert!(resp.body.contains("InvalidParameter"));
    }

    #[test]
    fn test_subscribe_and_unsubscribe() {
        let handler = SnsHandler::new();

        // Create topic
        let req = make_req("CreateTopic", json!({"Name": "test-sub-topic"}));
        let resp = handler.handle(req);
        let topic_arn = {
            use regex::Regex;
            let re = Regex::new(r"<TopicArn>(.*?)</TopicArn>").unwrap();
            re.captures(&resp.body).unwrap().get(1).unwrap().as_str().to_string()
        };

        // Subscribe
        let req = make_req("Subscribe", json!({
            "TopicArn": topic_arn,
            "Protocol": "sqs",
            "Endpoint": "arn:aws:sqs:us-east-1:123456789012:my-queue"
        }));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("SubscriptionArn"));

        // Unsubscribe
        let req = make_req("Unsubscribe", json!({
            "SubscriptionArn": "arn:aws:sns:us-east-1:123456789012:test:sub:00000000-0000-0000-0000-000000000000"
        }));
        let resp = handler.handle(req);
        assert_eq!(resp.status, 200);
    }
}
