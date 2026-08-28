//! Ecr operation handler.

use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::models::EcrState;
use crate::protocol::{AwsRequest, AwsResponse};

pub struct EcrHandler {
    state: RwLock<HashMap<(u64, String), EcrState>>,
}

impl EcrHandler {
    pub fn new() -> Self {
        Self { state: RwLock::new(HashMap::new()) }
    }

    fn get_state(&self, account: u64, region: &str) -> EcrState {
        let mut states = self.state.write();
        states.entry((account, region.to_string())).or_insert_with(EcrState::new).clone()
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let op = req.operation.as_str();
        match op {
            "BatchCheckLayerAvailability" => self.batchchecklayeravailability(&req),
            "BatchDeleteImage" => self.batchdeleteimage(&req),
            "BatchGetImage" => self.batchgetimage(&req),
            "BatchGetRepositoryScanningConfiguration" => self.batchgetrepositoryscanningconfiguration(&req),
            "CompleteLayerUpload" => self.completelayerupload(&req),
            "CreatePullThroughCacheRule" => self.createpullthroughcacherule(&req),
            "CreateRepository" => self.createrepository(&req),
            "CreateRepositoryCreationTemplate" => self.createrepositorycreationtemplate(&req),
            "DeleteLifecyclePolicy" => self.deletelifecyclepolicy(&req),
            "DeletePullThroughCacheRule" => self.deletepullthroughcacherule(&req),
            "DeleteRegistryPolicy" => self.deleteregistrypolicy(&req),
            "DeleteRepository" => self.deleterepository(&req),
            "DeleteRepositoryCreationTemplate" => self.deleterepositorycreationtemplate(&req),
            "DeleteRepositoryPolicy" => self.deleterepositorypolicy(&req),
            "DeleteSigningConfiguration" => self.deletesigningconfiguration(&req),
            "DeregisterPullTimeUpdateExclusion" => self.deregisterpulltimeupdateexclusion(&req),
            "DescribeImageReplicationStatus" => self.describeimagereplicationstatus(&req),
            "DescribeImageScanFindings" => self.describeimagescanfindings(&req),
            "DescribeImageSigningStatus" => self.describeimagesigningstatus(&req),
            "DescribeImages" => self.describeimages(&req),
            "DescribePullThroughCacheRules" => self.describepullthroughcacherules(&req),
            "DescribeRegistry" => self.describeregistry(&req),
            "DescribeRepositories" => self.describerepositories(&req),
            "DescribeRepositoryCreationTemplates" => self.describerepositorycreationtemplates(&req),
            "GetAccountSetting" => self.getaccountsetting(&req),
            "GetAuthorizationToken" => self.getauthorizationtoken(&req),
            "GetDownloadUrlForLayer" => self.getdownloadurlforlayer(&req),
            "GetLifecyclePolicy" => self.getlifecyclepolicy(&req),
            "GetLifecyclePolicyPreview" => self.getlifecyclepolicypreview(&req),
            "GetRegistryPolicy" => self.getregistrypolicy(&req),
            "GetRegistryScanningConfiguration" => self.getregistryscanningconfiguration(&req),
            "GetRepositoryPolicy" => self.getrepositorypolicy(&req),
            "GetSigningConfiguration" => self.getsigningconfiguration(&req),
            "InitiateLayerUpload" => self.initiatelayerupload(&req),
            "ListImageReferrers" => self.listimagereferrers(&req),
            "ListImages" => self.listimages(&req),
            "ListPullTimeUpdateExclusions" => self.listpulltimeupdateexclusions(&req),
            "ListTagsForResource" => self.listtagsforresource(&req),
            "PutAccountSetting" => self.putaccountsetting(&req),
            "PutImage" => self.putimage(&req),
            "PutImageScanningConfiguration" => self.putimagescanningconfiguration(&req),
            "PutImageTagMutability" => self.putimagetagmutability(&req),
            "PutLifecyclePolicy" => self.putlifecyclepolicy(&req),
            "PutRegistryPolicy" => self.putregistrypolicy(&req),
            "PutRegistryScanningConfiguration" => self.putregistryscanningconfiguration(&req),
            "PutReplicationConfiguration" => self.putreplicationconfiguration(&req),
            "PutSigningConfiguration" => self.putsigningconfiguration(&req),
            "RegisterPullTimeUpdateExclusion" => self.registerpulltimeupdateexclusion(&req),
            "SetRepositoryPolicy" => self.setrepositorypolicy(&req),
            "StartImageScan" => self.startimagescan(&req),
            "StartLifecyclePolicyPreview" => self.startlifecyclepolicypreview(&req),
            "TagResource" => self.tagresource(&req),
            "UntagResource" => self.untagresource(&req),
            "UpdateImageStorageClass" => self.updateimagestorageclass(&req),
            "UpdatePullThroughCacheRule" => self.updatepullthroughcacherule(&req),
            "UpdateRepositoryCreationTemplate" => self.updaterepositorycreationtemplate(&req),
            "UploadLayerPart" => self.uploadlayerpart(&req),
            "ValidatePullThroughCacheRule" => self.validatepullthroughcacherule(&req),
            other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn batchchecklayeravailability(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn batchdeleteimage(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn batchgetimage(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn batchgetrepositoryscanningconfiguration(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn completelayerupload(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn createpullthroughcacherule(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn createrepository(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn createrepositorycreationtemplate(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletelifecyclepolicy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletepullthroughcacherule(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deleteregistrypolicy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deleterepository(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deleterepositorycreationtemplate(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deleterepositorypolicy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deletesigningconfiguration(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn deregisterpulltimeupdateexclusion(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeimagereplicationstatus(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeimagescanfindings(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeimagesigningstatus(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeimages(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describepullthroughcacherules(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describeregistry(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describerepositories(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn describerepositorycreationtemplates(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getaccountsetting(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getauthorizationtoken(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getdownloadurlforlayer(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getlifecyclepolicy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getlifecyclepolicypreview(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getregistrypolicy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getregistryscanningconfiguration(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getrepositorypolicy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn getsigningconfiguration(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn initiatelayerupload(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listimagereferrers(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listimages(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listpulltimeupdateexclusions(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn listtagsforresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putaccountsetting(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putimage(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putimagescanningconfiguration(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putimagetagmutability(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putlifecyclepolicy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putregistrypolicy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putregistryscanningconfiguration(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putreplicationconfiguration(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn putsigningconfiguration(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn registerpulltimeupdateexclusion(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn setrepositorypolicy(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn startimagescan(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn startlifecyclepolicypreview(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn tagresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn untagresource(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updateimagestorageclass(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updatepullthroughcacherule(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn updaterepositorycreationtemplate(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn uploadlayerpart(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }

    fn validatepullthroughcacherule(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::error(501, "NotImplemented", "TODO")
    }
}

impl Default for EcrHandler {
    fn default() -> Self { Self::new() }
}
