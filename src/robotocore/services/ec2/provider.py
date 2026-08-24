"""Native EC2 provider.

Intercepts operations that Moto doesn't implement or has bugs in:
- CreatePlacementGroup / DescribePlacementGroups / DeletePlacementGroup: Not implemented
- DetachVolume: Moto crashes when InstanceId is omitted
- DeleteVpcEndpoints: Moto crashes with NoneType.lower() error
- RunInstances: Capacity check gate, forwards to Moto on success
- RequestSpotInstances: Capacity check gate, forwards to Moto on success
"""

from __future__ import annotations

import logging
import threading
import uuid
from urllib.parse import parse_qs
from xml.sax.saxutils import escape as xml_escape

from starlette.requests import Request
from starlette.responses import Response

from robotocore.providers.moto_bridge import forward_to_moto
from robotocore.services.ec2.capacity import get_capacity_store

logger = logging.getLogger(__name__)

# In-memory placement group store: {account_id: {region: {name: group}}}
_placement_groups: dict[str, dict[str, dict[str, dict]]] = {}
_placement_groups_lock = threading.Lock()

VALID_STRATEGIES = {"cluster", "spread", "partition"}

# State handler registration flag
_default_state_handler_registered = False


def register_state_handler(manager=None) -> None:
    """Register EC2 native state save/load hooks with a state manager."""
    global _default_state_handler_registered

    is_default_manager = manager is None
    if manager is None:
        if _default_state_handler_registered:
            return
        from robotocore.state.manager import get_state_manager

        manager = get_state_manager()

    store = get_capacity_store()
    manager.register_native_handler("ec2_capacity", store.export_state, store.load_state)
    if is_default_manager:
        _default_state_handler_registered = True


async def handle_ec2_request(request: Request, region: str, account_id: str) -> Response:
    """Handle EC2 requests, intercepting unimplemented operations."""
    # Ensure state handler is registered
    register_state_handler()

    body = await request.body()
    params = parse_qs(body.decode("utf-8")) if body else {}
    # Also check query params
    for key, val in request.query_params.items():
        if key not in params:
            params[key] = [val]

    action = _get_param(params, "Action")
    handler = _ACTION_MAP.get(action)
    if handler:
        try:
            # Handle both sync and async handlers
            import inspect

            if inspect.iscoroutinefunction(handler):
                return await handler(request, params, region, account_id)
            else:
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


def _get_int_param(params: dict, key: str, default: int = 0) -> int:
    val = _get_param(params, key)
    if not val:
        return default
    try:
        return int(val)
    except ValueError:
        return default


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


async def _run_instances(request: Request, params: dict, region: str, account_id: str) -> Response:
    """RunInstances - capacity check gate, then forward to Moto."""
    from moto.backends import get_backend  # noqa: I001

    instance_type = _get_param(params, "InstanceType") or "m1.small"
    min_count = _get_int_param(params, "MinCount", 1)
    max_count = _get_int_param(params, "MaxCount", min_count)
    placement_az = _get_param(params, "Placement.AvailabilityZone")
    subnet_id = _get_param(params, "SubnetId")

    # Determine AZ
    az = placement_az
    if not az and subnet_id:
        try:
            backend = get_backend("ec2")[account_id][region]
            subnet = backend.get_subnet(subnet_id)
            az = subnet.availability_zone
        except Exception as exc:  # noqa: BLE001
            logger.debug("Could not get AZ from subnet: %s", exc)

    if not az:
        az = f"{region}a"

    # Check for market options (spot)
    market_type = _get_param(params, "InstanceMarketOptions.MarketType")
    is_spot = market_type == "spot"

    # Check capacity
    store = get_capacity_store()

    # Check if profile exists and is enabled
    profile = store.get_profile(account_id, region, instance_type, az)
    if profile and not profile.enabled:
        return _ec2_error(
            "Unsupported",
            "The requested configuration is currently not supported. "
            "Please check the documentation for supported configurations.",
            status_code=400,
        )

    # Check chaos override
    chaos_override = store.get_chaos_override()
    if chaos_override:
        error_code = chaos_override.get("error_code")
        if error_code == "InsufficientInstanceCapacity":
            return _ec2_error(
                "InsufficientInstanceCapacity",
                f"We currently do not have sufficient {instance_type} capacity "
                f"in the Availability Zone you requested ({az}). "
                f"Our system will be working on provisioning additional capacity. "
                f"You can currently get {instance_type} capacity by not specifying "
                f"an Availability Zone in your request or choosing "
                f"{region}b, {region}c.",
                status_code=500,
            )

    if is_spot:
        spot_available, _ = store.check_spot_capacity(account_id, region, instance_type, az)
        if not spot_available:
            return _ec2_error(
                "InsufficientInstanceCapacity",
                f"We currently do not have sufficient {instance_type} capacity "
                f"in the Availability Zone you requested ({az}). "
                f"Our system will be working on provisioning additional capacity. "
                f"You can currently get {instance_type} capacity by not specifying "
                f"an Availability Zone in your request or choosing "
                f"{region}b, {region}c.",
                status_code=500,
            )

    # Check on-demand capacity
    success, error_code = store.check_capacity(account_id, region, instance_type, az, max_count)
    if not success:
        if error_code == "InsufficientInstanceCapacity":
            return _ec2_error(
                "InsufficientInstanceCapacity",
                f"We currently do not have sufficient {instance_type} capacity "
                f"in the Availability Zone you requested ({az}). "
                f"Our system will be working on provisioning additional capacity. "
                f"You can currently get {instance_type} capacity by not specifying "
                f"an Availability Zone in your request or choosing "
                f"{region}b, {region}c.",
                status_code=500,
            )
        elif error_code == "Unsupported":
            return _ec2_error(
                "Unsupported",
                "The requested configuration is currently not supported. "
                "Please check the documentation for supported configurations.",
                status_code=400,
            )

    # Consume capacity
    if not store.consume_capacity(account_id, region, instance_type, az, max_count):
        return _ec2_error(
            "InsufficientInstanceCapacity",
            f"We currently do not have sufficient {instance_type} capacity "
            f"in the Availability Zone you requested ({az}). "
            f"Our system will be working on provisioning additional capacity. "
            f"You can currently get {instance_type} capacity by not specifying "
            f"an Availability Zone in your request or choosing "
            f"{region}b, {region}c.",
            status_code=500,
        )

    # Forward to Moto for actual instance creation
    try:
        return await forward_to_moto(request, "ec2", account_id=account_id)
    except Exception as e:  # noqa: BLE001
        logger.exception("Error in RunInstances")
        # Release capacity on failure
        store.release_capacity(account_id, region, instance_type, az, max_count)
        return _ec2_error(
            "InternalError",
            f"An internal error has occurred: {e}",
            status_code=500,
        )


async def _request_spot_instances(
    request: Request, params: dict, region: str, account_id: str
) -> Response:
    """RequestSpotInstances - capacity check gate, then forward to Moto."""
    from moto.backends import get_backend  # noqa: I001

    instance_type = _get_param(params, "LaunchSpecification.InstanceType") or "m1.small"
    count = _get_int_param(params, "InstanceCount", 1)
    placement_az = _get_param(params, "LaunchSpecification.Placement.AvailabilityZone")
    subnet_id = _get_param(params, "LaunchSpecification.SubnetId")

    # Determine AZ
    az = placement_az
    if not az and subnet_id:
        try:
            backend = get_backend("ec2")[account_id][region]
            subnet = backend.get_subnet(subnet_id)
            az = subnet.availability_zone
        except Exception as exc:  # noqa: BLE001
            logger.debug("Could not get AZ from subnet: %s", exc)

    if not az:
        az = f"{region}a"

    store = get_capacity_store()

    # Check if profile exists and is enabled
    profile = store.get_profile(account_id, region, instance_type, az)
    if profile and not profile.enabled:
        # Return spot request with capacity-not-available status
        # Build response manually since we're not forwarding to Moto
        return _build_spot_capacity_unavailable_response(params, count)

    # Check chaos override
    chaos_override = store.get_chaos_override()
    if chaos_override:
        error_code = chaos_override.get("error_code")
        if error_code == "InsufficientInstanceCapacity":
            return _build_spot_capacity_unavailable_response(params, count)

    # Check spot capacity
    spot_available, _ = store.check_spot_capacity(account_id, region, instance_type, az)
    if not spot_available:
        return _build_spot_capacity_unavailable_response(params, count)

    # Consume capacity
    if not store.consume_capacity(account_id, region, instance_type, az, count):
        return _build_spot_capacity_unavailable_response(params, count)

    # Forward to Moto for actual spot request creation
    try:
        return await forward_to_moto(request, "ec2", account_id=account_id)
    except Exception as e:  # noqa: BLE001
        logger.exception("Error in RequestSpotInstances")
        # Release capacity on failure
        store.release_capacity(account_id, region, instance_type, az, count)
        return _ec2_error(
            "InternalError",
            f"An internal error has occurred: {e}",
            status_code=500,
        )


def _build_spot_capacity_unavailable_response(params: dict, count: int) -> Response:
    """Build a spot capacity unavailable response."""
    spot_price = _get_param(params, "SpotPrice")
    image_id = _get_param(params, "LaunchSpecification.ImageId")
    instance_type = _get_param(params, "LaunchSpecification.InstanceType") or "m1.small"

    requests_xml = ""
    for _ in range(count):
        request_id = f"sir-{uuid.uuid4().hex[:17]}"
        requests_xml += f"""        <item>
            <spotInstanceRequestId>{request_id}</spotInstanceRequestId>
            <spotPrice>{spot_price or "0.05"}</spotPrice>
            <type>one-time</type>
            <state>open</state>
            <status>
                <code>capacity-not-available</code>
                <updateTime>2024-01-01T00:00:00.000Z</updateTime>
                <message>There is no Spot capacity available that matches your request.</message>
            </status>
            <instanceId/>
            <availabilityZoneGroup/>
            <launchSpecification>
                <imageId>{image_id or "ami-12345678"}</imageId>
                <instanceType>{instance_type}</instanceType>
            </launchSpecification>
            <launchGroup/>
            <createTime>2024-01-01T00:00:00.000Z</createTime>
            <productDescription>Linux/UNIX</productDescription>
        </item>
"""

    xml = f"""<?xml version="1.0" encoding="UTF-8"?>
<RequestSpotInstancesResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">
    <requestId>{uuid.uuid4()}</requestId>
    <spotInstanceRequestSet>
{requests_xml}    </spotInstanceRequestSet>
</RequestSpotInstancesResponse>"""
    return Response(content=xml, status_code=200, media_type="text/xml")


_ACTION_MAP = {
    "CreatePlacementGroup": _create_placement_group,
    "DescribePlacementGroups": _describe_placement_groups,
    "DeletePlacementGroup": _delete_placement_group,
    "DetachVolume": _detach_volume,
    "DeleteVpcEndpoints": _delete_vpc_endpoints,
    "RunInstances": _run_instances,
    "RequestSpotInstances": _request_spot_instances,
}
