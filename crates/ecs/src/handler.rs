//! Ecs operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::EcsState;
use crate::protocol::{AwsRequest, AwsResponse};

pub struct EcsHandler {
    state: RwLock<HashMap<(u64, String), EcsState>>,
}

impl EcsHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> EcsState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(EcsState::new).clone()
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let op = req.operation.as_str();
        match op {
            "CreateCapacityProvider" => self.createcapacityprovider(&req),
            "CreateCluster" => self.createcluster(&req),
            "CreateExpressGatewayService" => self.createexpressgatewayservice(&req),
            "CreateService" => self.createservice(&req),
            "CreateTaskSet" => self.createtaskset(&req),
            "DeleteAccountSetting" => self.deleteaccountsetting(&req),
            "DeleteAttributes" => self.deleteattributes(&req),
            "DeleteCapacityProvider" => self.deletecapacityprovider(&req),
            "DeleteCluster" => self.deletecluster(&req),
            "DeleteExpressGatewayService" => self.deleteexpressgatewayservice(&req),
            "DeleteService" => self.deleteservice(&req),
            "DeleteTaskDefinitions" => self.deletetaskdefinitions(&req),
            "DeleteTaskSet" => self.deletetaskset(&req),
            "DeregisterContainerInstance" => self.deregistercontainerinstance(&req),
            "DeregisterTaskDefinition" => self.deregistertaskdefinition(&req),
            "DescribeCapacityProviders" => self.describecapacityproviders(&req),
            "DescribeClusters" => self.describeclusters(&req),
            "DescribeContainerInstances" => self.describecontainerinstances(&req),
            "DescribeExpressGatewayService" => self.describeexpressgatewayservice(&req),
            "DescribeServiceDeployments" => self.describeservicedeployments(&req),
            "DescribeServiceRevisions" => self.describeservicerevisions(&req),
            "DescribeServices" => self.describeservices(&req),
            "DescribeTaskDefinition" => self.describetaskdefinition(&req),
            "DescribeTaskSets" => self.describetasksets(&req),
            "DescribeTasks" => self.describetasks(&req),
            "DiscoverPollEndpoint" => self.discoverpollendpoint(&req),
            "ExecuteCommand" => self.executecommand(&req),
            "GetTaskProtection" => self.gettaskprotection(&req),
            "ListAccountSettings" => self.listaccountsettings(&req),
            "ListAttributes" => self.listattributes(&req),
            "ListClusters" => self.listclusters(&req),
            "ListContainerInstances" => self.listcontainerinstances(&req),
            "ListServiceDeployments" => self.listservicedeployments(&req),
            "ListServices" => self.listservices(&req),
            "ListServicesByNamespace" => self.listservicesbynamespace(&req),
            "ListTagsForResource" => self.listtagsforresource(&req),
            "ListTaskDefinitionFamilies" => self.listtaskdefinitionfamilies(&req),
            "ListTaskDefinitions" => self.listtaskdefinitions(&req),
            "ListTasks" => self.listtasks(&req),
            "PutAccountSetting" => self.putaccountsetting(&req),
            "PutAccountSettingDefault" => self.putaccountsettingdefault(&req),
            "PutAttributes" => self.putattributes(&req),
            "PutClusterCapacityProviders" => self.putclustercapacityproviders(&req),
            "RegisterContainerInstance" => self.registercontainerinstance(&req),
            "RegisterTaskDefinition" => self.registertaskdefinition(&req),
            "RunTask" => self.runtask(&req),
            "StartTask" => self.starttask(&req),
            "StopServiceDeployment" => self.stopservicedeployment(&req),
            "StopTask" => self.stoptask(&req),
            "SubmitAttachmentStateChanges" => self.submitattachmentstatechanges(&req),
            "SubmitContainerStateChange" => self.submitcontainerstatechange(&req),
            "SubmitTaskStateChange" => self.submittaskstatechange(&req),
            "TagResource" => self.tagresource(&req),
            "UntagResource" => self.untagresource(&req),
            "UpdateCapacityProvider" => self.updatecapacityprovider(&req),
            "UpdateCluster" => self.updatecluster(&req),
            "UpdateClusterSettings" => self.updateclustersettings(&req),
            "UpdateContainerAgent" => self.updatecontaineragent(&req),
            "UpdateContainerInstancesState" => self.updatecontainerinstancesstate(&req),
            "UpdateExpressGatewayService" => self.updateexpressgatewayservice(&req),
            "UpdateService" => self.updateservice(&req),
            "UpdateServicePrimaryTaskSet" => self.updateserviceprimarytaskset(&req),
            "UpdateTaskProtection" => self.updatetaskprotection(&req),
            "UpdateTaskSet" => self.updatetaskset(&req),
            other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn createcapacityprovider(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn createcluster(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn createexpressgatewayservice(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn createservice(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn createtaskset(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deleteaccountsetting(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deleteattributes(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletecapacityprovider(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletecluster(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deleteexpressgatewayservice(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deleteservice(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletetaskdefinitions(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletetaskset(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deregistercontainerinstance(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deregistertaskdefinition(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describecapacityproviders(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeclusters(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describecontainerinstances(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeexpressgatewayservice(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeservicedeployments(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeservicerevisions(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeservices(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describetaskdefinition(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describetasksets(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describetasks(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn discoverpollendpoint(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn executecommand(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn gettaskprotection(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listaccountsettings(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listattributes(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listclusters(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listcontainerinstances(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listservicedeployments(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listservices(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listservicesbynamespace(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listtagsforresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listtaskdefinitionfamilies(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listtaskdefinitions(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listtasks(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putaccountsetting(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putaccountsettingdefault(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putattributes(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putclustercapacityproviders(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn registercontainerinstance(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn registertaskdefinition(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn runtask(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn starttask(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn stopservicedeployment(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn stoptask(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn submitattachmentstatechanges(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn submitcontainerstatechange(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn submittaskstatechange(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn tagresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn untagresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updatecapacityprovider(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updatecluster(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updateclustersettings(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updatecontaineragent(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updatecontainerinstancesstate(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updateexpressgatewayservice(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updateservice(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updateserviceprimarytaskset(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updatetaskprotection(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updatetaskset(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }
}

impl Default for EcsHandler {
    fn default() -> Self { Self::new() }
}
