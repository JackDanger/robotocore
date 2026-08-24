"""Native EC2 provider.

Intercepts operations that Moto doesn't implement or has bugs in:
- CreatePlacementGroup / DescribePlacementGroups / DeletePlacementGroup: Not implemented
- DetachVolume: Moto crashes when InstanceId is omitted
- DeleteVpcEndpoints: Moto crashes with NoneType.lower() error
- RunInstances: Optionally launches guest containers when guest execution is enabled
- TerminateInstances: Optionally cleans up guest containers

Also provides Packer-compatible virtual instance transport for opt-in
container-backed instances with SSH/SSM connectivity.
"""

from __future__ import annotations

import base64
import logging
import os
import threading
import uuid
from typing import Any
from urllib.parse import parse_qs
from xml.sax.saxutils import escape as xml_escape

from starlette.requests import Request
from starlette.responses import Response

from robotocore.providers.moto_bridge import forward_to_moto
from robotocore.services.ec2.guest.executor import (
    get_guest_executor,
    is_guest_executor_enabled,
)

logger = logging.getLogger(__name__)

# Optional packer transport - only loaded when enabled
logger = logging.getLogger(__name__)
_packer_transport_available = False
try:
    from .packer.ami_builder import get_ami_builder

    _packer_transport_available = True
except ImportError as e:
    logger.debug("Packer transport not available: %s", e)

# In-memory placement group store: {account_id: {region: {name: group}}}
_placement_groups: dict[str, dict[str, dict[str, dict]]] = {}
_placement_groups_lock = threading.Lock()

VALID_STRATEGIES = {"cluster", "spread", "partition"}

# Packer transport configuration
PACKER_TRANSPORT_ENABLED = os.environ.get("ROBOTOCORE_PACKER_TRANSPORT", "").lower() in (
    "1",
    "true",
    "yes",
    "enabled",
)

# Track instances with active transport: {instance_id: transport}
_instance_transports: dict[str, Any] = {}
_instance_transports_lock = threading.Lock()


async def handle_ec2_request(request: Request, region: str, account_id: str) -> Response:
    """Handle EC2 requests, intercepting unimplemented operations."""
    body = await request.body()
    params = parse_qs(body.decode("utf-8")) if body else {}
    # Also check query params
    for key, val in request.query_params.items():
        if key not in params:
            params[key] = [val]

    action = _get_param(params, "Action")

    # Handle RunInstances with guest execution
    if action == "RunInstances":
        return await _run_instances(request, params, region, account_id)

    # Handle TerminateInstances with guest cleanup
    if action == "TerminateInstances":
        return await _terminate_instances(request, params, region, account_id)

    handler = _ACTION_MAP.get(action)
    if handler:
        try:
            return handler(params, region, account_id)
        except NotImplementedError as e:
            xml = (
                f'<?xml version="1.0" encoding="UTF-8"?>'
                f"<Response><Errors><Error><Code>NotImplemented</Code>"
                f"<Message>{xml_escape(str(e))}</Message></Error></Errors></Response>"
            )
            return Response(content=xml, status_code=501, media_type="text/xml")
        except Exception as e:  # noqa: BLE001
            xml = (
                f'<?xml version="1.0" encoding="UTF-8"?>'
                f"<Response><Errors><Error><Code>InternalError</Code>"
                f"<Message>{xml_escape(str(e))}</Message></Error></Errors></Response>"
            )
            return Response(content=xml, status_code=500, media_type="text/xml")

    return await forward_to_moto(request, "ec2", account_id=account_id)


def _get_param(params: dict, key: str) -> str:
    vals = params.get(key, [])
    return vals[0] if vals else ""


def _ec2_error(code: str, message: str, status_code: int = 400) -> Response:
    """Return a standard EC2 XML error response."""
    xml = (
        f'<?xml version="1.0" encoding="UTF-8"?>'
        f"<Response><Errors><Error><Code>{xml_escape(code)}</Code>"
        f"<Message>{xml_escape(message)}</Message></Error></Errors>"
        f"<RequestID>{uuid.uuid4()}</RequestID></Response>"
    )
    return Response(content=xml, status_code=status_code, media_type="text/xml")


def _create_placement_group(params: dict, region: str, account_id: str) -> Response:
    name = _get_param(params, "GroupName")
    strategy = _get_param(params, "Strategy") or "cluster"
    partition_count = _get_param(params, "PartitionCount")

    if not name:
        return _ec2_error(
            "MissingParameter",
            "The request must contain the parameter GroupName.",
        )

    if strategy not in VALID_STRATEGIES:
        return _ec2_error(
            "InvalidParameterValue",
            f"Value ({strategy}) for parameter strategy is invalid. "
            f"Unknown placement group strategy.",
        )

    with _placement_groups_lock:
        store = _placement_groups.setdefault(account_id, {}).setdefault(region, {})
        if name in store:
            return _ec2_error(
                "InvalidPlacementGroup.Duplicate",
                f"Placement group '{name}' already exists.",
            )

        group_id = f"pg-{uuid.uuid4().hex[:17]}"
        group = {
            "groupName": name,
            "strategy": strategy,
            "state": "available",
            "groupId": group_id,
            "partitionCount": partition_count or ("7" if strategy == "partition" else ""),
        }
        store[name] = group

    partition_count_xml = ""
    if group["partitionCount"]:
        partition_count_xml = (
            f"        <partitionCount>{group['partitionCount']}</partitionCount>\n"
        )

    xml = f"""<?xml version="1.0" encoding="UTF-8"?>
<CreatePlacementGroupResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{uuid.uuid4()}</requestId>
    <return>true</return>
    <placementGroup>
        <groupName>{name}</groupName>
        <state>available</state>
        <strategy>{strategy}</strategy>
        <groupId>{group_id}</groupId>
{partition_count_xml}    </placementGroup>
</CreatePlacementGroupResponse>"""
    return Response(content=xml, status_code=200, media_type="text/xml")


def _describe_placement_groups(params: dict, region: str, account_id: str) -> Response:
    with _placement_groups_lock:
        store = _placement_groups.get(account_id, {}).get(region, {})

        # Filter by GroupName.N
        names = []
        i = 1
        while True:
            name = _get_param(params, f"GroupName.{i}")
            if not name:
                break
            names.append(name)
            i += 1

        if names:
            # AWS returns an error if any requested name doesn't exist
            for n in names:
                if n not in store:
                    return _ec2_error(
                        "InvalidPlacementGroup.Unknown",
                        f"The placement group '{n}' is unknown.",
                    )
            groups = [store[n] for n in names]
        else:
            groups = list(store.values())

    items = ""
    for g in groups:
        partition_count_xml = ""
        if g.get("partitionCount"):
            partition_count_xml = (
                f"            <partitionCount>{g['partitionCount']}</partitionCount>\n"
            )
        items += f"""        <item>
            <groupName>{g["groupName"]}</groupName>
            <strategy>{g["strategy"]}</strategy>
            <state>{g["state"]}</state>
            <groupId>{g["groupId"]}</groupId>
{partition_count_xml}        </item>
"""

    xml = f"""<?xml version="1.0" encoding="UTF-8"?>
<DescribePlacementGroupsResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{uuid.uuid4()}</requestId>
    <placementGroupSet>
{items}    </placementGroupSet>
</DescribePlacementGroupsResponse>"""
    return Response(content=xml, status_code=200, media_type="text/xml")


def _delete_placement_group(params: dict, region: str, account_id: str) -> Response:
    name = _get_param(params, "GroupName")

    if not name:
        return _ec2_error(
            "MissingParameter",
            "The request must contain the parameter GroupName.",
        )

    with _placement_groups_lock:
        store = _placement_groups.get(account_id, {}).get(region, {})
        if name not in store:
            return _ec2_error(
                "InvalidPlacementGroup.Unknown",
                f"The placement group '{name}' is unknown.",
            )
        store.pop(name)

    xml = f"""<?xml version="1.0" encoding="UTF-8"?>
<DeletePlacementGroupResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{uuid.uuid4()}</requestId>
    <return>true</return>
</DeletePlacementGroupResponse>"""
    return Response(content=xml, status_code=200, media_type="text/xml")


def _detach_volume(params: dict, region: str, account_id: str) -> Response:
    """DetachVolume — handle missing InstanceId by finding it from the volume."""
    from moto.backends import get_backend  # noqa: I001

    volume_id = _get_param(params, "VolumeId")
    instance_id = _get_param(params, "InstanceId")
    device = _get_param(params, "Device")

    backend = get_backend("ec2")[account_id][region]
    volume = backend.get_volume(volume_id)

    if not instance_id and volume.attachment:
        instance_id = volume.attachment.instance.id
    if not device and volume.attachment:
        device = volume.attachment.device

    attachment = backend.detach_volume(volume_id, instance_id, device)

    att_vol_id = attachment.volume.id if hasattr(attachment.volume, "id") else volume_id
    att_inst_id = attachment.instance.id if hasattr(attachment.instance, "id") else instance_id

    xml = f"""<?xml version="1.0" encoding="UTF-8"?>
<DetachVolumeResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{uuid.uuid4()}</requestId>
    <volumeId>{att_vol_id}</volumeId>
    <instanceId>{att_inst_id}</instanceId>
    <device>{attachment.device}</device>
    <status>{attachment.status}</status>
</DetachVolumeResponse>"""
    return Response(content=xml, status_code=200, media_type="text/xml")


def _delete_vpc_endpoints(params: dict, region: str, account_id: str) -> Response:
    """DeleteVpcEndpoints — handle Moto NoneType.lower() bug."""
    from moto.backends import get_backend  # noqa: I001

    endpoint_ids = []
    i = 1
    while True:
        eid = _get_param(params, f"VpcEndpointId.{i}")
        if not eid:
            break
        endpoint_ids.append(eid)
        i += 1

    backend = get_backend("ec2")[account_id][region]

    # Delete each endpoint manually, working around the Moto bug
    for eid in endpoint_ids:
        for ep in list(backend.vpc_end_points.values()):
            if ep.id == eid:
                ep.state = "deleted"
                break

    xml = f"""<?xml version="1.0" encoding="UTF-8"?>
<DeleteVpcEndpointsResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{uuid.uuid4()}</requestId>
    <unsuccessful/>
</DeleteVpcEndpointsResponse>"""
    return Response(content=xml, status_code=200, media_type="text/xml")


def _create_image(params: dict, region: str, account_id: str) -> Response:
    """CreateImage — support Packer-compatible AMI creation with identity clearing.

    When ROBOTOCORE_PACKER_TRANSPORT is enabled, this creates an AMI
    from a container-backed instance with proper identity clearing.
    """
    instance_id = _get_param(params, "InstanceId")
    ami_name = _get_param(params, "Name")
    description = _get_param(params, "Description")
    no_reboot = _get_param(params, "NoReboot") == "true"

    if not instance_id:
        return _ec2_error(
            "MissingParameter",
            "The request must contain the parameter InstanceId.",
        )

    if not ami_name:
        return _ec2_error(
            "MissingParameter",
            "The request must contain the parameter Name.",
        )

    # Check if packer transport is enabled and this instance has a transport
    if PACKER_TRANSPORT_ENABLED and _packer_transport_available:
        try:
            with _instance_transports_lock:
                transport = _instance_transports.get(instance_id)

            if transport and transport.is_running():
                # Use the AMI builder to create the AMI
                builder = get_ami_builder()
                result = builder.create_ami(
                    instance_id=instance_id,
                    ami_name=ami_name,
                    description=description,
                    no_reboot=no_reboot,
                    transport=transport,
                )

                # Return the response with the new AMI ID
                xml = f"""<?xml version="1.0" encoding="UTF-8"?>
<CreateImageResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{uuid.uuid4()}</requestId>
    <imageId>{result.ami_id}</imageId>
</CreateImageResponse>"""
                return Response(content=xml, status_code=200, media_type="text/xml")
        except Exception as e:  # noqa: BLE001
            logger.warning("Packer transport CreateImage failed, falling back to Moto: %s", e)

    # Fall back to Moto's CreateImage
    from moto.backends import get_backend  # noqa: I001

    backend = get_backend("ec2")[account_id][region]

    # Parse tag specifications from request
    tag_specifications: list[dict[str, Any]] = []
    i = 1
    while True:
        resource_type = _get_param(params, f"TagSpecification.{i}.ResourceType")
        if not resource_type:
            break
        tags = []
        j = 1
        while True:
            key = _get_param(params, f"TagSpecification.{i}.Tag.{j}.Key")
            if not key:
                break
            value = _get_param(params, f"TagSpecification.{i}.Tag.{j}.Value")
            tags.append({"Key": key, "Value": value})
            j += 1
        if tags:
            tag_specifications.append({"ResourceType": resource_type, "Tags": tags})
        i += 1

    # Create the image using Moto's backend
    ami = backend.create_image(
        instance_id=instance_id,
        name=ami_name,
        description=description,
        tag_specifications=tag_specifications,
    )

    xml = f"""<?xml version="1.0" encoding="UTF-8"?>
<CreateImageResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{uuid.uuid4()}</requestId>
    <imageId>{ami.id}</imageId>
</CreateImageResponse>"""
    return Response(content=xml, status_code=200, media_type="text/xml")


_ACTION_MAP = {
    "CreatePlacementGroup": _create_placement_group,
    "DescribePlacementGroups": _describe_placement_groups,
    "DeletePlacementGroup": _delete_placement_group,
    "DetachVolume": _detach_volume,
    "DeleteVpcEndpoints": _delete_vpc_endpoints,
    "CreateImage": _create_image,
}


async def _run_instances(request: Request, params: dict, region: str, account_id: str) -> Response:
    """RunInstances - forward to Moto, then optionally launch guest container."""
    # First, let Moto create the instances
    response = await forward_to_moto(request, "ec2", account_id=account_id)

    # If guest execution is disabled, just return the response
    if not is_guest_executor_enabled():
        return response

    # If the response is not successful, return it as-is
    if response.status_code != 200:
        return response

    # Parse the response to get instance IDs
    try:
        import xml.etree.ElementTree as ET

        body = response.body if hasattr(response, "body") else b""
        if not body:
            return response

        root = ET.fromstring(body)

        # Find instance IDs by iterating through the XML tree
        # The structure is: RunInstancesResponse > instancesSet > item > instanceId
        instances = []
        for child in root:
            if "instancesSet" in child.tag:
                for item in child:
                    if "item" in item.tag:
                        for subchild in item:
                            if "instanceId" in subchild.tag:
                                instances.append(subchild)
                                break

        if not instances:
            return response

        # Extract parameters from the request
        user_data = _get_param(params, "UserData")
        instance_type = _get_param(params, "InstanceType") or "t2.micro"

        # Parse block device mappings
        block_device_mappings = []
        i = 1
        while True:
            device_name = _get_param(params, f"BlockDeviceMapping.{i}.DeviceName")
            if not device_name:
                break
            volume_size = _get_param(params, f"BlockDeviceMapping.{i}.Ebs.VolumeSize") or "8"
            block_device_mappings.append(
                {
                    "DeviceName": device_name,
                    "Ebs": {"VolumeSize": volume_size},
                }
            )
            i += 1

        # Parse IAM instance profile
        iam_instance_profile = None
        iam_profile_arn = _get_param(params, "IamInstanceProfile.Arn")
        iam_profile_name = _get_param(params, "IamInstanceProfile.Name")
        if iam_profile_arn or iam_profile_name:
            iam_instance_profile = {
                "Arn": iam_profile_arn,
                "Name": iam_profile_name,
            }

        # Decode base64 user-data if present
        if user_data:
            try:
                user_data = base64.b64decode(user_data).decode("utf-8")
            except Exception:
                pass  # Keep as-is if not valid base64

        # Launch guest containers for each instance
        executor = get_guest_executor()
        for inst_elem in instances:
            instance_id = inst_elem.text if hasattr(inst_elem, "text") else inst_elem
            if instance_id:
                logger.debug(f"Launching guest container for instance {instance_id}")

                # Run in background thread to not block response
                import threading

                def launch():
                    executor.launch_instance(
                        instance_id=instance_id,
                        account_id=account_id,
                        region=region,
                        user_data=user_data,
                        instance_type=instance_type,
                        block_device_mappings=block_device_mappings,
                        iam_instance_profile=iam_instance_profile,
                    )

                threading.Thread(target=launch, daemon=True).start()

    except Exception as e:
        logger.warning(f"Failed to launch guest container: {e}")

    return response


async def _terminate_instances(
    request: Request, params: dict, region: str, account_id: str
) -> Response:
    """TerminateInstances - forward to Moto, then clean up guest containers."""
    # First, let Moto terminate the instances
    response = await forward_to_moto(request, "ec2", account_id=account_id)

    # If guest execution is disabled, just return the response
    if not is_guest_executor_enabled():
        return response

    # Parse the request to get instance IDs
    instance_ids = []
    i = 1
    while True:
        inst_id = _get_param(params, f"InstanceId.{i}")
        if not inst_id:
            break
        instance_ids.append(inst_id)
        i += 1

    # Clean up guest containers
    executor = get_guest_executor()
    for instance_id in instance_ids:
        logger.info(f"Terminating guest container for instance {instance_id}")
        executor.terminate_instance(instance_id)

    return response
