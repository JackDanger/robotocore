"""EC2 Guest Executor Plugin.

This plugin integrates the guest executor into Robotocore's plugin system,
allowing it to intercept RunInstances and TerminateInstances operations.
"""

from __future__ import annotations

import logging

from starlette.requests import Request
from starlette.responses import JSONResponse, Response

from robotocore.extensions.base import RobotocorePlugin
from robotocore.services.ec2.guest.executor import (
    get_guest_executor,
    is_guest_executor_enabled,
)

logger = logging.getLogger(__name__)


class EC2GuestExecutorPlugin(RobotocorePlugin):
    """Plugin that enables EC2 guest execution for user-data scripts.

    When enabled via ROBOTOCORE_EC2_GUEST_EXECUTOR=1, this plugin:
    1. Intercepts RunInstances to launch guest containers
    2. Intercepts TerminateInstances to clean up guest containers
    3. Provides a /_robotocore/ec2/guest/executions endpoint for retrieving
       execution evidence
    """

    name = "ec2-guest-executor"
    version = "1.0.0"
    api_version = "1.0"
    description = "EC2 guest execution for user-data scripts"
    priority = 50  # Run before default EC2 provider

    def __init__(self) -> None:
        self._executor = get_guest_executor()
        self._enabled = is_guest_executor_enabled()

    def get_capabilities(self) -> set[str]:
        return {"custom_routes", "request_hooks"}

    def get_custom_routes(self) -> list[tuple[str, str, callable]]:
        """Return custom HTTP routes for guest execution management."""
        return [
            ("/_robotocore/ec2/guest/executions", "GET", self._list_executions),
            ("/_robotocore/ec2/guest/executions/{instance_id}", "GET", self._get_execution),
        ]

    async def _list_executions(self, request: Request) -> Response:
        """List all guest executions."""
        account_id = request.query_params.get("account_id")
        region = request.query_params.get("region")

        results = self._executor.list_executions(
            account_id=account_id,
            region=region,
        )

        return JSONResponse(
            {
                "executions": [r.to_dict() for r in results],
                "count": len(results),
                "enabled": self._enabled,
            }
        )

    async def _get_execution(self, request: Request) -> Response:
        """Get execution details for a specific instance."""
        instance_id = request.path_params["instance_id"]
        result = self._executor.get_execution_result(instance_id)

        if result is None:
            return JSONResponse(
                {"error": f"No execution found for instance {instance_id}"},
                status_code=404,
            )

        return JSONResponse(result.to_dict())

    def on_request(self, request: Request, context: dict) -> Request | Response | None:
        """Intercept EC2 requests to handle guest execution.

        This is called for every request. We only process RunInstances
        responses to trigger guest container creation.
        """
        if not self._enabled:
            return None

        service = context.get("service_name")
        if service != "ec2":
            return None

        # We can't intercept the request directly for RunInstances
        # because we need the response to get the instance ID.
        # The actual interception happens in on_response.
        return None

    def on_response(
        self,
        request: Request,
        response: Response,
        context: dict,
    ) -> Response | None:
        """Intercept EC2 responses to trigger guest execution.

        For RunInstances: Launch guest container after successful creation.
        For TerminateInstances: Clean up guest container.
        """
        if not self._enabled:
            return None

        service = context.get("service_name")
        if service != "ec2":
            return None

        operation = context.get("operation")
        if not operation:
            # Try to extract from request body for EC2
            try:
                from urllib.parse import parse_qs

                body = request._body if hasattr(request, "_body") else b""
                if body:
                    params = parse_qs(body.decode("utf-8"))
                    operation = params.get("Action", [""])[0]
            except Exception:
                pass  # best-effort sniffing — leave operation unset if the body can't be parsed

        if operation == "RunInstances":
            self._handle_run_instances_response(request, response, context)
        elif operation == "TerminateInstances":
            self._handle_terminate_instances_response(request, response, context)

        return None

    def _handle_run_instances_response(
        self,
        request: Request,
        response: Response,
        context: dict,
    ) -> None:
        """Handle RunInstances response to launch guest container."""
        if response.status_code != 200:
            return

        try:
            # Parse the response to get instance details
            # EC2 returns XML, so we need to parse it
            import xml.etree.ElementTree as ET

            body = response.body if hasattr(response, "body") else b""
            if not body:
                return

            root = ET.fromstring(body)

            # Find instance IDs in the response
            # EC2 RunInstances returns a reservation with instances
            ns = {"ec2": "http://ec2.amazonaws.com/doc/2016-11-15/"}

            # Try with namespace first
            instances = root.findall(".//ec2:instanceId", ns)
            if not instances:
                # Try without namespace
                instances = root.findall(".//instanceId")

            if not instances:
                return

            # Also extract user data from the request
            user_data = None
            instance_type = "t2.micro"
            block_device_mappings = []
            iam_instance_profile = None

            try:
                from urllib.parse import parse_qs

                req_body = request._body if hasattr(request, "_body") else b""
                if req_body:
                    params = parse_qs(req_body.decode("utf-8"))
                    user_data = params.get("UserData", [None])[0]
                    instance_type = params.get("InstanceType", ["t2.micro"])[0]
                    # Parse block device mappings and IAM profile if present
                    # This is simplified - full parsing would handle all EC2 parameters
            except Exception:
                pass  # best-effort request-body parsing — fall back to the defaults set above

            account_id = context.get("account_id", "123456789012")
            region = context.get("region", "us-east-1")

            # Launch guest container for each instance
            for inst_elem in instances:
                instance_id = inst_elem.text
                if instance_id:
                    logger.info(f"Launching guest container for instance {instance_id}")

                    # Run in background to not block the response
                    import threading

                    def launch():
                        self._executor.launch_instance(
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
            logger.warning(f"Failed to process RunInstances response for guest execution: {e}")

    def _handle_terminate_instances_response(
        self,
        request: Request,
        response: Response,
        context: dict,
    ) -> None:
        """Handle TerminateInstances response to clean up guest containers."""
        if response.status_code != 200:
            return

        try:
            # Parse the response to get terminated instance IDs
            import xml.etree.ElementTree as ET

            body = response.body if hasattr(response, "body") else b""
            if not body:
                return

            root = ET.fromstring(body)

            # Find instance IDs
            ns = {"ec2": "http://ec2.amazonaws.com/doc/2016-11-15/"}
            instances = root.findall(".//ec2:instanceId", ns)
            if not instances:
                instances = root.findall(".//instanceId")

            for inst_elem in instances:
                instance_id = inst_elem.text
                if instance_id:
                    logger.info(f"Terminating guest container for instance {instance_id}")
                    self._executor.terminate_instance(instance_id)

        except Exception as e:
            logger.warning(f"Failed to process TerminateInstances response: {e}")


# Plugin instance for discovery
plugin = EC2GuestExecutorPlugin()
