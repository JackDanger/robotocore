//! ECS operation handler.

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

    fn json_stub(&self, _req: &AwsRequest, field: &str) -> AwsResponse {
        AwsResponse::json(200, json!({ field: "" }))
    }

    fn json_stub_list(&self, _req: &AwsRequest, field: &str) -> AwsResponse {
        AwsResponse::json(200, json!({ field: [] }))
    }

    pub fn handle(&self, req: AwsRequest) -> AwsResponse {
        let op = req.operation.as_str();
        match op {
            "CreateCluster" => self.create_cluster(&req),
            "DeleteCluster" => self.delete_cluster(&req),
            "DescribeClusters" => self.describe_clusters(&req),
            "ListClusters" => self.list_clusters(&req),
            "UpdateCluster" => self.update_cluster(&req),
            "PutClusterCapacityProviders" => self.put_capacity_providers(&req),
            "PutClusterSettings" => self.put_cluster_settings(&req),
            "RegisterService" => self.register_service(&req),
            "DeregisterService" => self.deregister_service(&req),
            "DescribeServices" => self.describe_services(&req),
            "ListServices" => self.list_services(&req),
            "UpdateService" => self.update_service(&req),
            "RunTask" => self.run_task(&req),
            "StopTask" => self.stop_task(&req),
            "DescribeTasks" => self.describe_tasks(&req),
            "ListTasks" => self.list_tasks(&req),
            "RegisterContainerInstance" => self.register_container_instance(&req),
            "DeregisterContainerInstance" => self.deregister_container_instance(&req),
            "DescribeContainerInstances" => self.describe_container_instances(&req),
            "ListContainerInstances" => self.list_container_instances(&req),
            "PutTaskDefinition" => self.put_task_definition(&req),
            "DescribeTaskDefinition" => self.describe_task_definition(&req),
            "ListTaskDefinitions" => self.list_task_definitions(&req),
            "DeleteTaskDefinition" => self.delete_task_definition(&req),
            "ListTagsForResource" => self.list_tags(&req),
            "TagResource" => self.tag_resource(&req),
            "UntagResource" => self.untag_resource(&req),
                        "CreateCapacityProvider" => self.json_stub(&req, "CapacityProvider"),
            "CreateExpressGatewayService" => self.json_stub(&req, "ExpressGatewayService"),
            "DeleteExpressGatewayService" => self.json_stub(&req, "ExpressGatewayService"),
            "DescribeCapacityProviders" => self.json_stub(&req, "CapacityProviders"),
            "DescribeExpressGatewayService" => self.json_stub(&req, "ExpressGatewayService"),
            "DescribeServiceDeployments" => self.json_stub(&req, "ServiceDeployments"),
            "DescribeServiceRevisions" => self.json_stub(&req, "ServiceRevisions"),
            "DiscoverPollEndpoint" => self.json_stub(&req, "DiscoverPollEndpoint"),
            "ListAccountSettings" => self.json_stub_list(&req, "AccountSettings"),
            "ListAttributes" => self.json_stub_list(&req, "Attributes"),
            "ListServicesByNamespace" => self.json_stub_list(&req, "ServicesByNamespace"),
            "ListTaskDefinitionFamilies" => self.json_stub_list(&req, "TaskDefinitionFamilies"),
            "PutAccountSetting" => self.json_stub(&req, "AccountSetting"),
            "PutAccountSettingDefault" => self.json_stub(&req, "AccountSettingDefault"),
            "PutAttributes" => self.json_stub(&req, "Attributes"),
            "RegisterTaskDefinition" => self.json_stub(&req, "RegisterTaskDefinition"),
            "StopServiceDeployment" => self.json_stub(&req, "StopServiceDeployment"),
            "SubmitAttachmentStateChanges" => self.json_stub(&req, "SubmitAttachmentStateChanges"),
            "SubmitContainerStateChange" => self.json_stub(&req, "SubmitContainerStateChange"),
            "SubmitTaskStateChange" => self.json_stub(&req, "SubmitTaskStateChange"),
            "UpdateContainerInstancesState" => self.json_stub(&req, "ContainerInstancesState"),
            "UpdateExpressGatewayService" => self.json_stub(&req, "ExpressGatewayService"),
    "CreateService" => { AwsResponse::json(200, json!({"service": json!({"serviceArn": "", "serviceName": "", "clusterArn": "", "loadBalancers": json!([]), "serviceRegistries": json!([]), "status": "", "desiredCount": 0, "runningCount": 0, "pendingCount": 0, "launchType": "", "capacityProviderStrategy": json!([]), "platformVersion": "", "platformFamily": "", "taskDefinition": "", "deploymentConfiguration": json!({"deploymentCircuitBreaker": json!({"enable": false, "rollback": false}), "maximumPercent": 0, "minimumHealthyPercent": 0, "alarms": json!({"alarmNames": json!([]), "rollback": false, "enable": false}), "strategy": "", "bakeTimeInMinutes": 0, "lifecycleHooks": json!([]), "linearConfiguration": json!({"stepPercent": 0.0, "stepBakeTimeInMinutes": 0}), "canaryConfiguration": json!({"canaryPercent": 0.0, "canaryBakeTimeInMinutes": 0})}), "taskSets": json!([]), "deployments": json!([]), "roleArn": "", "events": json!([]), "createdAt": Value::Null, "currentServiceDeployment": "", "currentServiceRevisions": json!([]), "placementConstraints": json!([]), "placementStrategy": json!([]), "networkConfiguration": json!({"awsvpcConfiguration": json!({"subnets": json!([]), "securityGroups": json!([]), "assignPublicIp": ""})}), "healthCheckGracePeriodSeconds": 0, "schedulingStrategy": "", "deploymentController": json!({"type": ""}), "tags": json!([]), "createdBy": "", "enableECSManagedTags": false, "propagateTags": "", "enableExecuteCommand": false, "availabilityZoneRebalancing": "", "resourceManagementType": ""})})) },
    "CreateTaskSet" => { AwsResponse::json(200, json!({"taskSet": json!({"id": "", "taskSetArn": "", "serviceArn": "", "clusterArn": "", "startedBy": "", "externalId": "", "status": "", "taskDefinition": "", "computedDesiredCount": 0, "pendingCount": 0, "runningCount": 0, "createdAt": Value::Null, "updatedAt": Value::Null, "launchType": "", "capacityProviderStrategy": json!([]), "platformVersion": "", "platformFamily": "", "networkConfiguration": json!({"awsvpcConfiguration": json!({"subnets": json!([]), "securityGroups": json!([]), "assignPublicIp": ""})}), "loadBalancers": json!([]), "serviceRegistries": json!([]), "scale": json!({"value": 0.0, "unit": ""}), "stabilityStatus": "", "stabilityStatusAt": Value::Null, "tags": json!([]), "fargateEphemeralStorage": json!({"kmsKeyId": ""})})})) },
    "DeleteAccountSetting" => { AwsResponse::json(200, json!({"setting": json!({"name": "", "value": "", "principalArn": "", "type": ""})})) },
    "DeleteAttributes" => { AwsResponse::json(200, json!({"attributes": json!([])})) },
    "DeleteCapacityProvider" => { AwsResponse::json(200, json!({"capacityProvider": json!({"capacityProviderArn": "", "name": "", "cluster": "", "status": "", "autoScalingGroupProvider": json!({"autoScalingGroupArn": "", "managedScaling": json!({"status": "", "targetCapacity": 0, "minimumScalingStepSize": 0, "maximumScalingStepSize": 0, "instanceWarmupPeriod": 0}), "managedTerminationProtection": "", "managedDraining": ""}), "managedInstancesProvider": json!({"infrastructureRoleArn": "", "instanceLaunchTemplate": json!({"ec2InstanceProfileArn": "", "networkConfiguration": json!({"subnets": json!([]), "securityGroups": json!([])}), "storageConfiguration": json!({"storageSizeGiB": 0}), "monitoring": "", "capacityOptionType": "", "instanceRequirements": json!({"vCpuCount": json!({"min": 0, "max": 0}), "memoryMiB": json!({"min": 0, "max": 0}), "cpuManufacturers": json!([]), "memoryGiBPerVCpu": json!({"min": 0.0, "max": 0.0}), "excludedInstanceTypes": json!([]), "instanceGenerations": json!([]), "spotMaxPricePercentageOverLowestPrice": 0, "onDemandMaxPricePercentageOverLowestPrice": 0, "bareMetal": "", "burstablePerformance": "", "requireHibernateSupport": false, "networkInterfaceCount": json!({"min": 0, "max": 0}), "localStorage": "", "localStorageTypes": json!([]), "totalLocalStorageGB": json!({"min": 0.0, "max": 0.0}), "baselineEbsBandwidthMbps": json!({"min": 0, "max": 0}), "acceleratorTypes": json!([]), "acceleratorCount": json!({"min": 0, "max": 0}), "acceleratorManufacturers": json!([]), "acceleratorNames": json!([]), "acceleratorTotalMemoryMiB": json!({"min": 0, "max": 0}), "networkBandwidthGbps": json!({"min": 0.0, "max": 0.0}), "allowedInstanceTypes": json!([]), "maxSpotPriceAsPercentageOfOptimalOnDemandPrice": 0}), "fipsEnabled": false, "capacityReservations": json!({"reservationGroupArn": "", "reservationPreference": ""})}), "propagateTags": "", "infrastructureOptimization": json!({"scaleInAfter": 0})}), "updateStatus": "", "updateStatusReason": "", "tags": json!([]), "type": ""})})) },
    "DeleteService" => { AwsResponse::json(200, json!({"service": json!({"serviceArn": "", "serviceName": "", "clusterArn": "", "loadBalancers": json!([]), "serviceRegistries": json!([]), "status": "", "desiredCount": 0, "runningCount": 0, "pendingCount": 0, "launchType": "", "capacityProviderStrategy": json!([]), "platformVersion": "", "platformFamily": "", "taskDefinition": "", "deploymentConfiguration": json!({"deploymentCircuitBreaker": json!({"enable": false, "rollback": false}), "maximumPercent": 0, "minimumHealthyPercent": 0, "alarms": json!({"alarmNames": json!([]), "rollback": false, "enable": false}), "strategy": "", "bakeTimeInMinutes": 0, "lifecycleHooks": json!([]), "linearConfiguration": json!({"stepPercent": 0.0, "stepBakeTimeInMinutes": 0}), "canaryConfiguration": json!({"canaryPercent": 0.0, "canaryBakeTimeInMinutes": 0})}), "taskSets": json!([]), "deployments": json!([]), "roleArn": "", "events": json!([]), "createdAt": Value::Null, "currentServiceDeployment": "", "currentServiceRevisions": json!([]), "placementConstraints": json!([]), "placementStrategy": json!([]), "networkConfiguration": json!({"awsvpcConfiguration": json!({"subnets": json!([]), "securityGroups": json!([]), "assignPublicIp": ""})}), "healthCheckGracePeriodSeconds": 0, "schedulingStrategy": "", "deploymentController": json!({"type": ""}), "tags": json!([]), "createdBy": "", "enableECSManagedTags": false, "propagateTags": "", "enableExecuteCommand": false, "availabilityZoneRebalancing": "", "resourceManagementType": ""})})) },
    "DeleteTaskDefinitions" => { AwsResponse::json(200, json!({"taskDefinitions": json!([]), "failures": json!([])})) },
    "DeleteTaskSet" => { AwsResponse::json(200, json!({"taskSet": json!({"id": "", "taskSetArn": "", "serviceArn": "", "clusterArn": "", "startedBy": "", "externalId": "", "status": "", "taskDefinition": "", "computedDesiredCount": 0, "pendingCount": 0, "runningCount": 0, "createdAt": Value::Null, "updatedAt": Value::Null, "launchType": "", "capacityProviderStrategy": json!([]), "platformVersion": "", "platformFamily": "", "networkConfiguration": json!({"awsvpcConfiguration": json!({"subnets": json!([]), "securityGroups": json!([]), "assignPublicIp": ""})}), "loadBalancers": json!([]), "serviceRegistries": json!([]), "scale": json!({"value": 0.0, "unit": ""}), "stabilityStatus": "", "stabilityStatusAt": Value::Null, "tags": json!([]), "fargateEphemeralStorage": json!({"kmsKeyId": ""})})})) },
    "DeregisterTaskDefinition" => { AwsResponse::json(200, json!({"taskDefinition": json!({"taskDefinitionArn": "", "containerDefinitions": json!([]), "family": "", "taskRoleArn": "", "executionRoleArn": "", "networkMode": "", "revision": 0, "volumes": json!([]), "status": "", "requiresAttributes": json!([]), "placementConstraints": json!([]), "compatibilities": json!([]), "runtimePlatform": json!({"cpuArchitecture": "", "operatingSystemFamily": ""}), "requiresCompatibilities": json!([]), "cpu": "", "memory": "", "inferenceAccelerators": json!([]), "pidMode": "", "ipcMode": "", "proxyConfiguration": json!({"type": "", "containerName": "", "properties": json!([])}), "registeredAt": Value::Null, "deregisteredAt": Value::Null, "registeredBy": "", "ephemeralStorage": json!({"sizeInGiB": 0}), "enableFaultInjection": false})})) },
    "DescribeTaskSets" => { AwsResponse::json(200, json!({"taskSets": json!([]), "failures": json!([])})) },
    "ExecuteCommand" => { AwsResponse::json(200, json!({"clusterArn": "", "containerArn": "", "containerName": "", "interactive": false, "session": json!({"sessionId": "", "streamUrl": "", "tokenValue": ""}), "taskArn": ""})) },
    "GetTaskProtection" => { AwsResponse::json(200, json!({"protectedTasks": json!([]), "failures": json!([])})) },
    "ListServiceDeployments" => { AwsResponse::json(200, json!({"serviceDeployments": json!([]), "nextToken": ""})) },
    "StartTask" => { AwsResponse::json(200, json!({"tasks": json!([]), "failures": json!([])})) },
    "UpdateCapacityProvider" => { AwsResponse::json(200, json!({"capacityProvider": json!({"capacityProviderArn": "", "name": "", "cluster": "", "status": "", "autoScalingGroupProvider": json!({"autoScalingGroupArn": "", "managedScaling": json!({"status": "", "targetCapacity": 0, "minimumScalingStepSize": 0, "maximumScalingStepSize": 0, "instanceWarmupPeriod": 0}), "managedTerminationProtection": "", "managedDraining": ""}), "managedInstancesProvider": json!({"infrastructureRoleArn": "", "instanceLaunchTemplate": json!({"ec2InstanceProfileArn": "", "networkConfiguration": json!({"subnets": json!([]), "securityGroups": json!([])}), "storageConfiguration": json!({"storageSizeGiB": 0}), "monitoring": "", "capacityOptionType": "", "instanceRequirements": json!({"vCpuCount": json!({"min": 0, "max": 0}), "memoryMiB": json!({"min": 0, "max": 0}), "cpuManufacturers": json!([]), "memoryGiBPerVCpu": json!({"min": 0.0, "max": 0.0}), "excludedInstanceTypes": json!([]), "instanceGenerations": json!([]), "spotMaxPricePercentageOverLowestPrice": 0, "onDemandMaxPricePercentageOverLowestPrice": 0, "bareMetal": "", "burstablePerformance": "", "requireHibernateSupport": false, "networkInterfaceCount": json!({"min": 0, "max": 0}), "localStorage": "", "localStorageTypes": json!([]), "totalLocalStorageGB": json!({"min": 0.0, "max": 0.0}), "baselineEbsBandwidthMbps": json!({"min": 0, "max": 0}), "acceleratorTypes": json!([]), "acceleratorCount": json!({"min": 0, "max": 0}), "acceleratorManufacturers": json!([]), "acceleratorNames": json!([]), "acceleratorTotalMemoryMiB": json!({"min": 0, "max": 0}), "networkBandwidthGbps": json!({"min": 0.0, "max": 0.0}), "allowedInstanceTypes": json!([]), "maxSpotPriceAsPercentageOfOptimalOnDemandPrice": 0}), "fipsEnabled": false, "capacityReservations": json!({"reservationGroupArn": "", "reservationPreference": ""})}), "propagateTags": "", "infrastructureOptimization": json!({"scaleInAfter": 0})}), "updateStatus": "", "updateStatusReason": "", "tags": json!([]), "type": ""})})) },
    "UpdateClusterSettings" => { AwsResponse::json(200, json!({"cluster": json!({"clusterArn": "", "clusterName": "", "configuration": json!({"executeCommandConfiguration": json!({"kmsKeyId": "", "logging": "", "logConfiguration": json!({"cloudWatchLogGroupName": "", "cloudWatchEncryptionEnabled": false, "s3BucketName": "", "s3EncryptionEnabled": false, "s3KeyPrefix": ""})}), "managedStorageConfiguration": json!({"kmsKeyId": "", "fargateEphemeralStorageKmsKeyId": ""})}), "status": "", "registeredContainerInstancesCount": 0, "runningTasksCount": 0, "pendingTasksCount": 0, "activeServicesCount": 0, "statistics": json!([]), "tags": json!([]), "settings": json!([]), "capacityProviders": json!([]), "defaultCapacityProviderStrategy": json!([]), "attachments": json!([]), "attachmentsStatus": "", "serviceConnectDefaults": json!({"namespace": ""})})})) },
    "UpdateContainerAgent" => { AwsResponse::json(200, json!({"containerInstance": json!({"containerInstanceArn": "", "ec2InstanceId": "", "capacityProviderName": "", "version": 0, "versionInfo": json!({"agentVersion": "", "agentHash": "", "dockerVersion": ""}), "remainingResources": json!([]), "registeredResources": json!([]), "status": "", "statusReason": "", "agentConnected": false, "runningTasksCount": 0, "pendingTasksCount": 0, "agentUpdateStatus": "", "attributes": json!([]), "registeredAt": Value::Null, "attachments": json!([]), "tags": json!([]), "healthStatus": json!({"overallStatus": "", "details": json!([])})})})) },
    "UpdateServicePrimaryTaskSet" => { AwsResponse::json(200, json!({"taskSet": json!({"id": "", "taskSetArn": "", "serviceArn": "", "clusterArn": "", "startedBy": "", "externalId": "", "status": "", "taskDefinition": "", "computedDesiredCount": 0, "pendingCount": 0, "runningCount": 0, "createdAt": Value::Null, "updatedAt": Value::Null, "launchType": "", "capacityProviderStrategy": json!([]), "platformVersion": "", "platformFamily": "", "networkConfiguration": json!({"awsvpcConfiguration": json!({"subnets": json!([]), "securityGroups": json!([]), "assignPublicIp": ""})}), "loadBalancers": json!([]), "serviceRegistries": json!([]), "scale": json!({"value": 0.0, "unit": ""}), "stabilityStatus": "", "stabilityStatusAt": Value::Null, "tags": json!([]), "fargateEphemeralStorage": json!({"kmsKeyId": ""})})})) },
    "UpdateTaskProtection" => { AwsResponse::json(200, json!({"protectedTasks": json!([]), "failures": json!([])})) },
    "UpdateTaskSet" => { AwsResponse::json(200, json!({"taskSet": json!({"id": "", "taskSetArn": "", "serviceArn": "", "clusterArn": "", "startedBy": "", "externalId": "", "status": "", "taskDefinition": "", "computedDesiredCount": 0, "pendingCount": 0, "runningCount": 0, "createdAt": Value::Null, "updatedAt": Value::Null, "launchType": "", "capacityProviderStrategy": json!([]), "platformVersion": "", "platformFamily": "", "networkConfiguration": json!({"awsvpcConfiguration": json!({"subnets": json!([]), "securityGroups": json!([]), "assignPublicIp": ""})}), "loadBalancers": json!([]), "serviceRegistries": json!([]), "scale": json!({"value": 0.0, "unit": ""}), "stabilityStatus": "", "stabilityStatusAt": Value::Null, "tags": json!([]), "fargateEphemeralStorage": json!({"kmsKeyId": ""})})})) },
other => AwsResponse::error(400, "ValidationException",
                &format!("The operation {} is not implemented", other)),
        }
    }

    fn create_cluster(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("clusterName")
            .and_then(|v| v.as_str()).unwrap_or("default").to_string();
        let state = self.get_state(req.account, &req.region);
        let arn = format!("arn:aws:ecs:{}:{}:cluster/{}", req.region, req.account, name);
        let cluster = json!({
            "clusterArn": arn,
            "clusterName": name,
            "status": "ACTIVE",
            "registeredContainerInstancesCount": 0,
            "runningTasksCount": 0,
            "pendingTasksCount": 0,
            "activeServicesCount": 0,
        });
        state.clusters.write().insert(name.clone(), cluster.clone());
        AwsResponse::json(200, json!({ "cluster": cluster }))
    }

    fn delete_cluster(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("cluster")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut clusters = state.clusters.write();
        if let Some(cluster) = clusters.remove(name) {
            AwsResponse::json(200, json!({ "cluster": cluster }))
        } else {
            AwsResponse::error(400, "ClusterNotFoundException",
                &format!("Cluster {name} not found"))
        }
    }

    fn describe_clusters(&self, req: &AwsRequest) -> AwsResponse {
        let names: Vec<String> = req.params.get("clusters")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let clusters = state.clusters.read();
        let result: Vec<Value> = if names.is_empty() {
            clusters.values().cloned().collect()
        } else {
            names.iter()
                .filter_map(|n| clusters.get(n).cloned())
                .collect()
        };
        let missing: Vec<Value> = if !names.is_empty() {
            names.iter()
                .filter(|n| !clusters.contains_key(n.as_str()))
                .map(|n| json!({
                    "clusterArn": format!("arn:aws:ecs:{}:{}:cluster/{}", req.region, req.account, n),
                    "clusterName": n,
                    "status": "INACTIVE",
                }))
                .collect()
        } else {
            vec![]
        };
        let mut all = result;
        all.extend(missing);
        AwsResponse::json(200, json!({ "clusters": all }))
    }

    fn list_clusters(&self, _req: &AwsRequest) -> AwsResponse {
        let states = self.state.read();
        let mut arns = Vec::new();
        for (_key, state) in states.iter() {
            let clusters = state.clusters.read();
            for (name, _) in clusters.iter() {
                arns.push(name.clone());
            }
        }
        AwsResponse::json(200, json!({ "clusterArns": arns, "nextToken": Value::Null }))
    }

    fn update_cluster(&self, req: &AwsRequest) -> AwsResponse {
        let name = req.params.get("cluster")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut clusters = state.clusters.write();
        if let Some(cluster) = clusters.get_mut(name) {
            if let Some(settings) = req.params.get("settings") {
                cluster["settings"] = settings.clone();
            }
            AwsResponse::json(200, json!({ "cluster": cluster.clone() }))
        } else {
            AwsResponse::error(400, "ClusterNotFoundException",
                &format!("Cluster {name} not found"))
        }
    }

    fn put_capacity_providers(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({ "cluster": Value::Null }))
    }

    fn put_cluster_settings(&self, _req: &AwsRequest) -> AwsResponse {
        AwsResponse::json(200, json!({ "cluster": Value::Null }))
    }

    fn register_service(&self, req: &AwsRequest) -> AwsResponse {
        let cluster = req.params.get("cluster")
            .and_then(|v| v.as_str()).unwrap_or("default");
        let name = req.params.get("serviceName")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let state = self.get_state(req.account, &req.region);
        let arn = format!("arn:aws:ecs:{}:{}:service/{}/{}", req.region, req.account, cluster, name);
        let service = json!({
            "serviceArn": arn,
            "serviceName": name,
            "clusterArn": format!("arn:aws:ecs:{}:{}:cluster/{}", req.region, req.account, cluster),
            "status": "ACTIVE",
            "desiredCount": req.params.get("desiredCount")
                .and_then(|v| v.as_u64()).unwrap_or(1),
            "runningCount": 0,
            "pendingCount": 0,
        });
        state.services.write().insert(arn.clone(), service.clone());
        AwsResponse::json(200, json!({ "service": service }))
    }

    fn deregister_service(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("service")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut services = state.services.write();
        if let Some(service) = services.remove(arn) {
            AwsResponse::json(200, json!({ "service": service }))
        } else {
            AwsResponse::error(400, "ServiceNotFoundException",
                &format!("Service {arn} not found"))
        }
    }

    fn describe_services(&self, req: &AwsRequest) -> AwsResponse {
        let arns: Vec<String> = req.params.get("services")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let services = state.services.read();
        let result: Vec<Value> = if arns.is_empty() {
            services.values().cloned().collect()
        } else {
            arns.iter()
                .filter_map(|a| services.get(a).cloned())
                .collect()
        };
        AwsResponse::json(200, json!({ "services": result, "failures": [] }))
    }

    fn list_services(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let services = state.services.read();
        let arns: Vec<String> = services.keys().cloned().collect();
        AwsResponse::json(200, json!({ "serviceArns": arns, "nextToken": Value::Null }))
    }

    fn update_service(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("service")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut services = state.services.write();
        if let Some(service) = services.get_mut(arn) {
            if let Some(count) = req.params.get("desiredCount") {
                service["desiredCount"] = count.clone();
            }
            AwsResponse::json(200, json!({ "service": service.clone() }))
        } else {
            AwsResponse::error(400, "ServiceNotFoundException",
                &format!("Service {arn} not found"))
        }
    }

    fn run_task(&self, req: &AwsRequest) -> AwsResponse {
        let cluster = req.params.get("cluster")
            .and_then(|v| v.as_str()).unwrap_or("default");
        let task_def = req.params.get("taskDefinition")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let arn = format!("arn:aws:ecs:{}:{}:task/{}/{}", req.region, req.account, cluster, uuid::Uuid::new_v4().simple());
        let task = json!({
            "taskArn": arn,
            "clusterArn": format!("arn:aws:ecs:{}:{}:cluster/{}", req.region, req.account, cluster),
            "taskDefinitionArn": task_def,
            "lastStatus": "PENDING",
            "desiredStatus": "RUNNING",
            "createdAt": chrono::Utc::now().to_rfc3339(),
        });
        let state = self.get_state(req.account, &req.region);
        state.tasks.write().insert(arn.clone(), task.clone());
        AwsResponse::json(200, json!({
            "tasks": [task],
            "failures": []
        }))
    }

    fn stop_task(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("task")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut tasks = state.tasks.write();
        if let Some(task) = tasks.get_mut(arn) {
            task["lastStatus"] = json!("STOPPED");
            task["desiredStatus"] = json!("STOPPED");
            AwsResponse::json(200, json!({ "task": task.clone() }))
        } else {
            AwsResponse::error(400, "TaskNotFoundException",
                &format!("Task {arn} not found"))
        }
    }

    fn describe_tasks(&self, req: &AwsRequest) -> AwsResponse {
        let arns: Vec<String> = req.params.get("tasks")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let tasks = state.tasks.read();
        let result: Vec<Value> = if arns.is_empty() {
            tasks.values().cloned().collect()
        } else {
            arns.iter()
                .filter_map(|a| tasks.get(a).cloned())
                .collect()
        };
        AwsResponse::json(200, json!({ "tasks": result, "failures": [] }))
    }

    fn list_tasks(&self, req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(req.account, &req.region);
        let tasks = state.tasks.read();
        let arns: Vec<String> = tasks.keys().cloned().collect();
        AwsResponse::json(200, json!({ "taskArns": arns, "nextToken": Value::Null }))
    }

    fn register_container_instance(&self, req: &AwsRequest) -> AwsResponse {
        let cluster = req.params.get("cluster")
            .and_then(|v| v.as_str()).unwrap_or("default");
        let arn = format!("arn:aws:ecs:{}:{}:container-instance/{}", req.region, req.account, uuid::Uuid::new_v4().simple());
        let instance = json!({
            "containerInstanceArn": arn,
            "clusterArn": format!("arn:aws:ecs:{}:{}:cluster/{}", req.region, req.account, cluster),
            "status": "REGISTERED",
            "version": 1,
            "capacityProviders": [],
        });
        let state = self.get_state(req.account, &req.region);
        state.container_instances.write().insert(arn.clone(), instance.clone());
        AwsResponse::json(200, json!({ "containerInstance": instance }))
    }

    fn deregister_container_instance(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("containerInstance")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut instances = state.container_instances.write();
        if let Some(mut instance) = instances.remove(arn) {
            instance["status"] = json!("DEREGISTERED");
            AwsResponse::json(200, json!({ "containerInstance": instance }))
        } else {
            AwsResponse::error(400, "ContainerInstanceNotFoundException",
                &format!("Container instance {arn} not found"))
        }
    }

    fn describe_container_instances(&self, req: &AwsRequest) -> AwsResponse {
        let arns: Vec<String> = req.params.get("containerInstances")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let instances = state.container_instances.read();
        let result: Vec<Value> = if arns.is_empty() {
            instances.values().cloned().collect()
        } else {
            arns.iter()
                .filter_map(|a| instances.get(a).cloned())
                .collect()
        };
        AwsResponse::json(200, json!({ "containerInstances": result, "failures": [] }))
    }

    fn list_container_instances(&self, _req: &AwsRequest) -> AwsResponse {
        let state = self.get_state(0, "");
        let states = self.state.read();
        let mut arns = Vec::new();
        for (_key, state) in states.iter() {
            let instances = state.container_instances.read();
            for (arn, _) in instances.iter() {
                arns.push(arn.clone());
            }
        }
        AwsResponse::json(200, json!({ "containerInstanceArns": arns, "nextToken": Value::Null }))
    }

    fn put_task_definition(&self, req: &AwsRequest) -> AwsResponse {
        let family = req.params.get("family")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let revision = req.params.get("revision")
            .and_then(|v| v.as_i64()).map(|r| r as u64 + 1)
            .unwrap_or(1);
        let arn = format!("arn:aws:ecs:{}:{}:task-definition/{}/{}", req.region, req.account, family, revision);
        let task_def = json!({
            "taskDefinitionArn": arn,
            "family": family,
            "revision": revision,
            "status": "ACTIVE",
            "containerDefinitions": req.params.get("containerDefinitions")
                .cloned().unwrap_or(json!([])),
            "requiresCompatibilities": req.params.get("requiresCompatibilities")
                .cloned().unwrap_or(json!(["FARGATE"])),
        });
        let state = self.get_state(req.account, &req.region);
        state.task_definitions.write().insert(arn.clone(), task_def.clone());
        AwsResponse::json(200, json!({ "taskDefinition": task_def }))
    }

    fn describe_task_definition(&self, req: &AwsRequest) -> AwsResponse {
        let family = req.params.get("taskDefinition")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let defs = state.task_definitions.read();
        if let Some((arn, def)) = defs.iter().find(|(_, def)| {
            def.get("family").and_then(|f| f.as_str()) == Some(family)
        }) {
            let mut v = def.clone();
            v["taskDefinitionArn"] = json!(arn);
            AwsResponse::json(200, json!({ "taskDefinition": v }))
        } else {
            AwsResponse::error(400, "TaskDefinitionNotFoundException",
                &format!("Task definition {family} not found"))
        }
    }

    fn list_task_definitions(&self, req: &AwsRequest) -> AwsResponse {
        let family = req.params.get("familyPrefix")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let defs = state.task_definitions.read();
        let arns: Vec<String> = defs.iter()
            .filter(|(_, def)| {
                let f = def.get("family").and_then(|f| f.as_str()).unwrap_or("");
                family.is_empty() || f.starts_with(family)
            })
            .map(|(arn, _)| arn.clone())
            .collect();
        AwsResponse::json(200, json!({ "taskDefinitionArns": arns, "nextToken": Value::Null }))
    }

    fn delete_task_definition(&self, req: &AwsRequest) -> AwsResponse {
        let family = req.params.get("taskDefinition")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut defs = state.task_definitions.write();
        let key = defs.iter()
            .find(|(_, def)| def.get("family").and_then(|f| f.as_str()) == Some(family))
            .map(|(arn, _)| arn.clone());
        if let Some(arn) = key {
            let mut def = defs.remove(&arn).unwrap();
            def["status"] = json!("INACTIVE");
            AwsResponse::json(200, json!({ "taskDefinition": def }))
        } else {
            AwsResponse::error(400, "TaskDefinitionNotFoundException",
                &format!("Task definition {family} not found"))
        }
    }

    fn list_tags(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("resourceArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let tags = state.tags.read().get(arn).cloned().unwrap_or_default();
        AwsResponse::json(200, json!({ "tags": tags }))
    }

    fn tag_resource(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("resourceArn")
            .and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let tags: Vec<Value> = req.params.get("tags")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut all_tags = state.tags.write();
        let entry = all_tags.entry(arn).or_insert_with(Vec::new);
        entry.extend(tags);
        AwsResponse::json(200, Value::Null)
    }

    fn untag_resource(&self, req: &AwsRequest) -> AwsResponse {
        let arn = req.params.get("resourceArn")
            .and_then(|v| v.as_str()).unwrap_or_default();
        let keys: Vec<String> = req.params.get("propagateTags")
            .or(req.params.get("tagKeys"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let state = self.get_state(req.account, &req.region);
        let mut all_tags = state.tags.write();
        if let Some(tags) = all_tags.get_mut(arn) {
            tags.retain(|t| {
                t.get("key").and_then(|k| k.as_str())
                    .map(|k| !keys.contains(&k.to_string()))
                    .unwrap_or(true)
            });
        }
        AwsResponse::json(200, Value::Null)
    }
}

impl Default for EcsHandler {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use serde_json::json;

    fn make_req(operation: &str, params: Value) -> AwsRequest {
        AwsRequest {
            service: "ecs".to_string(),
            operation: operation.to_string(),
            account: 123456789012,
            region: "us-east-1".to_string(),
            params,
            body: Bytes::new(),
        }
    }

    #[test]
    fn test_cluster_and_service() {
        let handler = EcsHandler::new();
        handler.handle(make_req("CreateCluster", json!({
            "clusterName": "test-cluster"
        })));
        let resp = handler.handle(make_req("DescribeClusters", json!({
            "clusters": ["test-cluster"]
        })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("test-cluster"));
    }

    #[test]
    fn test_run_task() {
        let handler = EcsHandler::new();
        handler.handle(make_req("CreateCluster", json!({
            "clusterName": "task-cluster"
        })));
        let resp = handler.handle(make_req("RunTask", json!({
            "cluster": "task-cluster",
            "taskDefinition": "my-task:1"
        })));
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("taskArn"));
    }
}
