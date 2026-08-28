//! Stepfunctions operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::StepfunctionsState;
use crate::protocol::{AwsRequest, AwsResponse};

pub struct StepfunctionsHandler {
    state: RwLock<HashMap<(u64, String), StepfunctionsState>>,
}

impl StepfunctionsHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> StepfunctionsState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(StepfunctionsState::new).clone()
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let op = req.operation.as_str();
        match op {
            "CreateActivity" => self.createactivity(&req),
            "CreateStateMachine" => self.createstatemachine(&req),
            "CreateStateMachineAlias" => self.createstatemachinealias(&req),
            "DeleteActivity" => self.deleteactivity(&req),
            "DeleteStateMachine" => self.deletestatemachine(&req),
            "DeleteStateMachineAlias" => self.deletestatemachinealias(&req),
            "DeleteStateMachineVersion" => self.deletestatemachineversion(&req),
            "DescribeActivity" => self.describeactivity(&req),
            "DescribeExecution" => self.describeexecution(&req),
            "DescribeMapRun" => self.describemaprun(&req),
            "DescribeStateMachine" => self.describestatemachine(&req),
            "DescribeStateMachineAlias" => self.describestatemachinealias(&req),
            "DescribeStateMachineForExecution" => self.describestatemachineforexecution(&req),
            "GetActivityTask" => self.getactivitytask(&req),
            "GetExecutionHistory" => self.getexecutionhistory(&req),
            "ListActivities" => self.listactivities(&req),
            "ListExecutions" => self.listexecutions(&req),
            "ListMapRuns" => self.listmapruns(&req),
            "ListStateMachineAliases" => self.liststatemachinealiases(&req),
            "ListStateMachineVersions" => self.liststatemachineversions(&req),
            "ListStateMachines" => self.liststatemachines(&req),
            "ListTagsForResource" => self.listtagsforresource(&req),
            "PublishStateMachineVersion" => self.publishstatemachineversion(&req),
            "RedriveExecution" => self.redriveexecution(&req),
            "SendTaskFailure" => self.sendtaskfailure(&req),
            "SendTaskHeartbeat" => self.sendtaskheartbeat(&req),
            "SendTaskSuccess" => self.sendtasksuccess(&req),
            "StartExecution" => self.startexecution(&req),
            "StartSyncExecution" => self.startsyncexecution(&req),
            "StopExecution" => self.stopexecution(&req),
            "TagResource" => self.tagresource(&req),
            "TestState" => self.teststate(&req),
            "UntagResource" => self.untagresource(&req),
            "UpdateMapRun" => self.updatemaprun(&req),
            "UpdateStateMachine" => self.updatestatemachine(&req),
            "UpdateStateMachineAlias" => self.updatestatemachinealias(&req),
            "ValidateStateMachineDefinition" => self.validatestatemachinedefinition(&req),
            other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn createactivity(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn createstatemachine(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn createstatemachinealias(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deleteactivity(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletestatemachine(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletestatemachinealias(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletestatemachineversion(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeactivity(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeexecution(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describemaprun(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describestatemachine(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describestatemachinealias(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describestatemachineforexecution(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getactivitytask(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getexecutionhistory(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listactivities(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listexecutions(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listmapruns(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn liststatemachinealiases(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn liststatemachineversions(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn liststatemachines(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listtagsforresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn publishstatemachineversion(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn redriveexecution(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn sendtaskfailure(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn sendtaskheartbeat(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn sendtasksuccess(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn startexecution(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn startsyncexecution(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn stopexecution(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn tagresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn teststate(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn untagresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updatemaprun(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updatestatemachine(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updatestatemachinealias(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn validatestatemachinedefinition(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }
}

impl Default for StepfunctionsHandler {
    fn default() -> Self { Self::new() }
}
