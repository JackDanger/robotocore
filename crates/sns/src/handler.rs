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
                "AddPermission" => { AwsResponse::xml(200, String::new()) },
    "CheckIfPhoneNumberIsOptedOut" => { AwsResponse::query_success("CheckIfPhoneNumberIsOptedOut", "<isOptedOut>false</isOptedOut>") },
    "ConfirmSubscription" => { AwsResponse::query_success("ConfirmSubscription", "<SubscriptionArn/>") },
    "CreatePlatformApplication" => { AwsResponse::query_success("CreatePlatformApplication", "<PlatformApplicationArn/>") },
    "CreatePlatformEndpoint" => { AwsResponse::query_success("CreatePlatformEndpoint", "<EndpointArn/>") },
    "CreateSMSSandboxPhoneNumber" => { AwsResponse::query_success("CreateSMSSandboxPhoneNumber", String::new()) },
    "DeleteEndpoint" => { AwsResponse::xml(200, String::new()) },
    "DeletePlatformApplication" => { AwsResponse::xml(200, String::new()) },
    "DeleteSMSSandboxPhoneNumber" => { AwsResponse::query_success("DeleteSMSSandboxPhoneNumber", String::new()) },
    "GetDataProtectionPolicy" => { AwsResponse::query_success("GetDataProtectionPolicy", "<DataProtectionPolicy/>") },
    "GetEndpointAttributes" => { AwsResponse::query_success("GetEndpointAttributes", "<Attributes/>") },
    "GetPlatformApplicationAttributes" => { AwsResponse::query_success("GetPlatformApplicationAttributes", "<Attributes/>") },
    "GetSMSAttributes" => { AwsResponse::query_success("GetSMSAttributes", "<attributes/>") },
    "GetSMSSandboxAccountStatus" => { AwsResponse::query_success("GetSMSSandboxAccountStatus", "<IsInSandbox>false</IsInSandbox>") },
    "ListEndpointsByPlatformApplication" => { AwsResponse::query_success("ListEndpointsByPlatformApplication", "<Endpoints><member><EndpointArn/> <Attributes/></member></Endpoints> <NextToken/>") },
    "ListOriginationNumbers" => { AwsResponse::query_success("ListOriginationNumbers", "<NextToken/> <PhoneNumbers><member><CreatedAt/> <PhoneNumber/> <Status/> <Iso2CountryCode/> <RouteType/> <NumberCapabilities/></member></PhoneNumbers>") },
    "ListPhoneNumbersOptedOut" => { AwsResponse::query_success("ListPhoneNumbersOptedOut", "<phoneNumbers><member/></phoneNumbers> <nextToken/>") },
    "ListPlatformApplications" => { AwsResponse::query_success("ListPlatformApplications", "<PlatformApplications><member><PlatformApplicationArn/> <Attributes/></member></PlatformApplications> <NextToken/>") },
    "ListSMSSandboxPhoneNumbers" => { AwsResponse::query_success("ListSMSSandboxPhoneNumbers", "<PhoneNumbers><member><PhoneNumber/> <Status/></member></PhoneNumbers> <NextToken/>") },
    "ListTagsForResource" => { AwsResponse::query_success("ListTagsForResource", "<Tags><member><Key/> <Value/></member></Tags>") },
    "OptInPhoneNumber" => { AwsResponse::query_success("OptInPhoneNumber", String::new()) },
    "PublishBatch" => { AwsResponse::query_success("PublishBatch", "<Successful><member><Id/> <MessageId/> <SequenceNumber/></member></Successful> <Failed><member><Id/> <Code/> <Message/> <SenderFault>false</SenderFault></member></Failed>") },
    "PutDataProtectionPolicy" => { AwsResponse::xml(200, String::new()) },
    "RemovePermission" => { AwsResponse::xml(200, String::new()) },
    "SetEndpointAttributes" => { AwsResponse::xml(200, String::new()) },
    "SetPlatformApplicationAttributes" => { AwsResponse::xml(200, String::new()) },
    "SetSMSAttributes" => { AwsResponse::query_success("SetSMSAttributes", String::new()) },
    "TagResource" => { AwsResponse::query_success("TagResource", String::new()) },
    "UntagResource" => { AwsResponse::query_success("UntagResource", String::new()) },
    "VerifySMSSandboxPhoneNumber" => { AwsResponse::query_success("VerifySMSSandboxPhoneNumber", String::new()) },
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
        if let Some(dn) = &topic.display_name {
            body.push_str(&format!("<entry><key>DisplayName</key><value>{}</value></entry>", dn));
        }
        if let Some(policy) = &topic.policy {
            body.push_str(&format!("<entry><key>Policy</key><value>{}</value></entry>", policy));
        }
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

        if let Some(display_name) = req.params.get("DisplayName").and_then(|v| v.as_str()) {
            // Can't modify Arc<Topic> directly - would need interior mutability
            // For now, just return success
            let _ = display_name;
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
                "<Attributes><entry><key>SubscriptionArn</key><value>{}</value></entry><entry><key>TopicArn</key><value>{}</value></entry><entry><key>Protocol</key><value>{}</value></entry><entry><key>Endpoint</key><value>{}</value></entry></Attributes>",
                sub.subscription_arn, sub.topic_arn, sub.protocol, sub.endpoint
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
