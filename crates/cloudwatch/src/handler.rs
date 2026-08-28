//! Cloudwatch operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::CloudwatchState;
use crate::protocol::{AwsRequest, AwsResponse};

pub struct CloudwatchHandler {
    state: RwLock<HashMap<(u64, String), CloudwatchState>>,
}

impl CloudwatchHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> CloudwatchState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(CloudwatchState::new).clone()
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let op = req.operation.as_str();
        match op {
            "DeleteAlarmMuteRule" => self.deletealarmmuterule(&req),
            "DeleteAlarms" => self.deletealarms(&req),
            "DeleteAnomalyDetector" => self.deleteanomalydetector(&req),
            "DeleteDashboards" => self.deletedashboards(&req),
            "DeleteInsightRules" => self.deleteinsightrules(&req),
            "DeleteMetricStream" => self.deletemetricstream(&req),
            "DescribeAlarmContributors" => self.describealarmcontributors(&req),
            "DescribeAlarmHistory" => self.describealarmhistory(&req),
            "DescribeAlarms" => self.describealarms(&req),
            "DescribeAlarmsForMetric" => self.describealarmsformetric(&req),
            "DescribeAnomalyDetectors" => self.describeanomalydetectors(&req),
            "DescribeInsightRules" => self.describeinsightrules(&req),
            "DisableAlarmActions" => self.disablealarmactions(&req),
            "DisableInsightRules" => self.disableinsightrules(&req),
            "EnableAlarmActions" => self.enablealarmactions(&req),
            "EnableInsightRules" => self.enableinsightrules(&req),
            "GetAlarmMuteRule" => self.getalarmmuterule(&req),
            "GetDashboard" => self.getdashboard(&req),
            "GetInsightRuleReport" => self.getinsightrulereport(&req),
            "GetMetricData" => self.getmetricdata(&req),
            "GetMetricStatistics" => self.getmetricstatistics(&req),
            "GetMetricStream" => self.getmetricstream(&req),
            "GetMetricWidgetImage" => self.getmetricwidgetimage(&req),
            "ListAlarmMuteRules" => self.listalarmmuterules(&req),
            "ListDashboards" => self.listdashboards(&req),
            "ListManagedInsightRules" => self.listmanagedinsightrules(&req),
            "ListMetricStreams" => self.listmetricstreams(&req),
            "ListMetrics" => self.listmetrics(&req),
            "ListTagsForResource" => self.listtagsforresource(&req),
            "PutAlarmMuteRule" => self.putalarmmuterule(&req),
            "PutAnomalyDetector" => self.putanomalydetector(&req),
            "PutCompositeAlarm" => self.putcompositealarm(&req),
            "PutDashboard" => self.putdashboard(&req),
            "PutInsightRule" => self.putinsightrule(&req),
            "PutManagedInsightRules" => self.putmanagedinsightrules(&req),
            "PutMetricAlarm" => self.putmetricalarm(&req),
            "PutMetricData" => self.putmetricdata(&req),
            "PutMetricStream" => self.putmetricstream(&req),
            "SetAlarmState" => self.setalarmstate(&req),
            "StartMetricStreams" => self.startmetricstreams(&req),
            "StopMetricStreams" => self.stopmetricstreams(&req),
            "TagResource" => self.tagresource(&req),
            "UntagResource" => self.untagresource(&req),
            other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn deletealarmmuterule(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletealarms(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deleteanomalydetector(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletedashboards(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deleteinsightrules(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletemetricstream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describealarmcontributors(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describealarmhistory(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describealarms(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describealarmsformetric(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeanomalydetectors(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeinsightrules(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn disablealarmactions(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn disableinsightrules(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn enablealarmactions(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn enableinsightrules(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getalarmmuterule(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getdashboard(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getinsightrulereport(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getmetricdata(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getmetricstatistics(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getmetricstream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getmetricwidgetimage(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listalarmmuterules(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listdashboards(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listmanagedinsightrules(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listmetricstreams(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listmetrics(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listtagsforresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putalarmmuterule(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putanomalydetector(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putcompositealarm(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putdashboard(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putinsightrule(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putmanagedinsightrules(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putmetricalarm(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putmetricdata(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putmetricstream(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn setalarmstate(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn startmetricstreams(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn stopmetricstreams(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn tagresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn untagresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }
}

impl Default for CloudwatchHandler {
    fn default() -> Self { Self::new() }
}
