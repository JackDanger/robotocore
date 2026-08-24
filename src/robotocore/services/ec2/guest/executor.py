"""EC2 Guest Executor - Container-backed user-data execution.

This module implements the core guest execution logic:
- Parsing user-data (shell scripts and MIME multi-part)
- Managing guest containers
- Executing user-data payloads
- Capturing execution evidence
"""

from __future__ import annotations

import base64
import logging
import os
import subprocess
import threading
import time
import uuid
from dataclasses import dataclass, field
from email import message_from_bytes
from typing import Any

logger = logging.getLogger(__name__)

# Environment variable to enable guest execution
ENV_GUEST_EXECUTOR = "ROBOTOCORE_EC2_GUEST_EXECUTOR"


def is_guest_executor_enabled() -> bool:
    """Check if guest execution is enabled.

    This checks the environment variable at runtime to allow enabling/disabling
    without restarting the server.
    """
    return os.environ.get(ENV_GUEST_EXECUTOR, "0") == "1"


# Container image for guest execution (systemd + basic tools)
DEFAULT_GUEST_IMAGE = os.environ.get("ROBOTOCORE_EC2_GUEST_IMAGE", "jrei/systemd-ubuntu:22.04")

# Network for IMDS access
GUEST_NETWORK_NAME = "robotocore-ec2-guest"


@dataclass
class ExecutionRecord:
    """A single execution record for a command or service action."""

    timestamp: str
    command: str
    stdout: str
    stderr: str
    exit_code: int
    duration_ms: float


@dataclass
class GuestExecutionResult:
    """Complete execution result for an EC2 instance's user-data."""

    instance_id: str
    account_id: str
    region: str
    start_time: str
    end_time: str | None = None
    commands: list[ExecutionRecord] = field(default_factory=list)
    status: str = "pending"  # pending, running, completed, failed
    error: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "instance_id": self.instance_id,
            "account_id": self.account_id,
            "region": self.region,
            "start_time": self.start_time,
            "end_time": self.end_time,
            "status": self.status,
            "error": self.error,
            "commands": [
                {
                    "timestamp": c.timestamp,
                    "command": c.command,
                    "stdout": c.stdout,
                    "stderr": c.stderr,
                    "exit_code": c.exit_code,
                    "duration_ms": c.duration_ms,
                }
                for c in self.commands
            ],
        }


class GuestExecutor:
    """Manages guest container lifecycle and user-data execution.

    This is the core class that:
    1. Creates/destroys guest containers for EC2 instances
    2. Parses and executes user-data (shell scripts, MIME multi-part)
    3. Captures execution evidence
    4. Provides IMDS access inside containers
    """

    def __init__(self) -> None:
        self._executions: dict[str, GuestExecutionResult] = {}
        self._containers: dict[str, str] = {}  # instance_id -> container_id
        self._lock = threading.Lock()
        self._network_created = False

    def _ensure_network(self) -> None:
        """Ensure the guest network exists for IMDS access."""
        if self._network_created:
            return

        try:
            # Check if network exists
            result = subprocess.run(
                ["docker", "network", "ls", "--format", "{{.Name}}"],
                capture_output=True,
                text=True,
                check=False,
            )
            if GUEST_NETWORK_NAME in result.stdout:
                self._network_created = True
                return

            # Create network
            subprocess.run(
                ["docker", "network", "create", GUEST_NETWORK_NAME],
                capture_output=True,
                check=True,
            )
            self._network_created = True
            logger.info(f"Created guest network: {GUEST_NETWORK_NAME}")
        except subprocess.CalledProcessError as e:
            logger.warning(f"Failed to create guest network: {e}")

    def _pull_image_if_needed(self, image: str) -> None:
        """Pull the guest image if not already present."""
        try:
            result = subprocess.run(
                ["docker", "images", "--format", "{{.Repository}}:{{.Tag}}", image],
                capture_output=True,
                text=True,
                check=False,
            )
            if image not in result.stdout:
                logger.info(f"Pulling guest image: {image}")
                subprocess.run(
                    ["docker", "pull", image],
                    capture_output=True,
                    check=False,
                )
        except subprocess.CalledProcessError as e:
            logger.warning(f"Failed to check/pull image: {e}")

    def _create_guest_container(
        self,
        instance_id: str,
        instance_type: str,
        image: str,
        block_devices: list[dict],
        iam_role_arn: str | None,
        account_id: str,
        region: str,
    ) -> str | None:
        """Create a guest container for an EC2 instance.

        Returns the container ID or None if creation failed.
        """
        self._ensure_network()
        self._pull_image_if_needed(image)

        # Generate container name
        container_name = f"robotocore-ec2-{instance_id}"

        # Build docker run command
        cmd = [
            "docker",
            "run",
            "-d",
            "--name",
            container_name,
            "--network",
            GUEST_NETWORK_NAME,
            "--privileged",  # Required for systemd and block device access
            "--cgroupns=host",
            "-v",
            "/sys/fs/cgroup:/sys/fs/cgroup:rw",
            "-e",
            "container=docker",
        ]

        # Add IMDS endpoint environment variable
        # The IMDS will be accessible via the host's IP on the guest network
        cmd.extend(["-e", "IMDS_ENDPOINT=http://169.254.169.254"])

        # Add IAM role info if present
        if iam_role_arn:
            cmd.extend(["-e", f"IAM_ROLE_ARN={iam_role_arn}"])

        # Add block device volumes
        for device in block_devices:
            device_name = device.get("DeviceName", "/dev/xvdf")
            volume_size = device.get("Ebs", {}).get("VolumeSize", "8")
            # Create a volume for this device
            volume_name = f"ec2-vol-{instance_id}-{device_name.replace('/', '_')}"
            try:
                # Create volume if it doesn't exist
                vol_result = subprocess.run(
                    [
                        "docker",
                        "volume",
                        "create",
                        "-d",
                        "local",
                        "--opt",
                        f"size={volume_size}G",
                        volume_name,
                    ],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                if vol_result.returncode == 0:
                    # Mount the volume at the device path
                    cmd.extend(["-v", f"{volume_name}:{device_name}"])
            except subprocess.CalledProcessError as e:
                logger.warning(f"Failed to create volume for {device_name}: {e}")

        # Add the image
        cmd.append(image)

        try:
            result = subprocess.run(cmd, capture_output=True, text=True, check=True)
            container_id = result.stdout.strip()
            logger.info(f"Created guest container {container_id[:12]} for instance {instance_id}")
            return container_id
        except subprocess.CalledProcessError as e:
            logger.error(f"Failed to create guest container: {e.stderr}")
            return None

    def _destroy_guest_container(self, instance_id: str) -> bool:
        """Destroy the guest container for an EC2 instance."""
        container_id = self._containers.get(instance_id)
        if not container_id:
            return False

        try:
            # Stop and remove container
            subprocess.run(
                ["docker", "stop", "-t", "10", container_id],
                capture_output=True,
                check=False,
            )
            subprocess.run(
                ["docker", "rm", "-f", container_id],
                capture_output=True,
                check=False,
            )
            logger.info(f"Destroyed guest container for instance {instance_id}")
            return True
        except subprocess.CalledProcessError as e:
            logger.warning(f"Failed to destroy guest container: {e}")
            return False

    def _parse_user_data(self, user_data: str | bytes | None) -> list[dict]:
        """Parse user-data into executable parts.

        Supports:
        - Plain shell scripts (starting with #!)
        - MIME multi-part messages (cloud-init format)
        - Base64-encoded data
        """
        if not user_data:
            return []

        # Decode if base64
        data = user_data
        if isinstance(data, str):
            try:
                # Try to decode as base64
                decoded = base64.b64decode(data, validate=True)
                data = decoded
            except Exception:
                data = data.encode("utf-8")
        elif isinstance(data, bytes):
            try:
                # Check if it's base64-encoded bytes
                decoded = base64.b64decode(data, validate=True)
                data = decoded
            except Exception:
                pass

        if not isinstance(data, bytes):
            data = data.encode("utf-8") if isinstance(data, str) else b""

        # Check for MIME multi-part
        if b"Content-Type: multipart/mixed" in data[:1024]:
            return self._parse_mime_multipart(data)

        # Check for shebang (shell script)
        if data.startswith(b"#!"):
            return [{"type": "shell", "content": data.decode("utf-8", errors="replace")}]

        # Check for cloud-init config (starts with #cloud-config)
        if data.startswith(b"#cloud-config"):
            return [{"type": "cloud-config", "content": data.decode("utf-8", errors="replace")}]

        # Default: treat as shell script
        return [{"type": "shell", "content": data.decode("utf-8", errors="replace")}]

    def _parse_mime_multipart(self, data: bytes) -> list[dict]:
        """Parse MIME multi-part user-data."""
        parts = []
        try:
            msg = message_from_bytes(data)
            for part in msg.walk():
                content_type = part.get_content_type()
                payload = part.get_payload(decode=True)
                if not payload:
                    continue

                content = payload.decode("utf-8", errors="replace")

                if content_type == "text/x-shellscript" or content.startswith("#!/"):
                    parts.append({"type": "shell", "content": content})
                elif content_type == "text/cloud-config":
                    parts.append({"type": "cloud-config", "content": content})
                elif content_type == "text/x-include-url":
                    parts.append({"type": "include-url", "content": content})
                else:
                    parts.append({"type": "shell", "content": content})
        except Exception as e:
            logger.warning(f"Failed to parse MIME multi-part: {e}")
            # Fallback: treat as shell script
            parts.append({"type": "shell", "content": data.decode("utf-8", errors="replace")})

        return parts

    def _execute_in_container(
        self,
        container_id: str,
        command: str,
        timeout: int = 300,
    ) -> ExecutionRecord:
        """Execute a command inside the guest container."""
        start_time = time.time()
        timestamp = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())

        try:
            # Execute command via docker exec
            result = subprocess.run(
                ["docker", "exec", container_id, "/bin/bash", "-c", command],
                capture_output=True,
                text=True,
                timeout=timeout,
                check=False,
            )
            duration_ms = (time.time() - start_time) * 1000

            return ExecutionRecord(
                timestamp=timestamp,
                command=command,
                stdout=result.stdout,
                stderr=result.stderr,
                exit_code=result.returncode,
                duration_ms=duration_ms,
            )
        except subprocess.TimeoutExpired:
            duration_ms = (time.time() - start_time) * 1000
            return ExecutionRecord(
                timestamp=timestamp,
                command=command,
                stdout="",
                stderr=f"Command timed out after {timeout}s",
                exit_code=-1,
                duration_ms=duration_ms,
            )
        except Exception as e:
            duration_ms = (time.time() - start_time) * 1000
            return ExecutionRecord(
                timestamp=timestamp,
                command=command,
                stdout="",
                stderr=str(e),
                exit_code=-1,
                duration_ms=duration_ms,
            )

    def _execute_shell_part(
        self,
        container_id: str,
        content: str,
        result: GuestExecutionResult,
    ) -> None:
        """Execute a shell script part inside the container."""
        # Write script to a file in the container
        script_path = f"/tmp/userdata-{uuid.uuid4().hex[:8]}.sh"

        # Escape the content for safe transfer
        escaped_content = content.replace("'", "'\"'\"'")
        write_cmd = f"echo '{escaped_content}' > {script_path} && chmod +x {script_path}"

        write_result = self._execute_in_container(container_id, write_cmd)
        if write_result.exit_code != 0:
            result.commands.append(
                ExecutionRecord(
                    timestamp=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                    command=f"write_script:{script_path}",
                    stdout=write_result.stdout,
                    stderr=write_result.stderr,
                    exit_code=write_result.exit_code,
                    duration_ms=write_result.duration_ms,
                )
            )
            return

        # Execute the script
        exec_result = self._execute_in_container(container_id, f"bash {script_path}")
        result.commands.append(exec_result)

        # Clean up
        self._execute_in_container(container_id, f"rm -f {script_path}")

    def _execute_cloud_config_part(
        self,
        container_id: str,
        content: str,
        result: GuestExecutionResult,
    ) -> None:
        """Execute a cloud-config part (simplified - just run as shell for now)."""
        # For now, treat cloud-config as shell script
        # A full implementation would parse the YAML and execute cloud-init modules
        self._execute_shell_part(container_id, content, result)

    def launch_instance(
        self,
        instance_id: str,
        account_id: str,
        region: str,
        user_data: str | bytes | None,
        instance_type: str = "t2.micro",
        block_device_mappings: list[dict] | None = None,
        iam_instance_profile: dict | None = None,
    ) -> GuestExecutionResult | None:
        """Launch a guest container for an EC2 instance and execute user-data.

        This is called when RunInstances creates a new instance and guest
        execution is enabled.
        """
        if not is_guest_executor_enabled():
            return None

        result = GuestExecutionResult(
            instance_id=instance_id,
            account_id=account_id,
            region=region,
            start_time=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            status="running",
        )

        with self._lock:
            self._executions[instance_id] = result

        # Extract IAM role ARN if present
        iam_role_arn = None
        if iam_instance_profile:
            iam_role_arn = iam_instance_profile.get("Arn")

        # Create the guest container
        container_id = self._create_guest_container(
            instance_id=instance_id,
            instance_type=instance_type,
            image=DEFAULT_GUEST_IMAGE,
            block_devices=block_device_mappings or [],
            iam_role_arn=iam_role_arn,
            account_id=account_id,
            region=region,
        )

        if not container_id:
            result.status = "failed"
            result.error = "Failed to create guest container"
            result.end_time = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            return result

        with self._lock:
            self._containers[instance_id] = container_id

        # Wait a moment for container to be ready
        time.sleep(1)

        # Parse and execute user-data
        parts = self._parse_user_data(user_data)
        for part in parts:
            if part["type"] == "shell":
                self._execute_shell_part(container_id, part["content"], result)
            elif part["type"] == "cloud-config":
                self._execute_cloud_config_part(container_id, part["content"], result)

        result.status = "completed"
        result.end_time = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())

        return result

    def terminate_instance(self, instance_id: str) -> bool:
        """Terminate the guest container for an EC2 instance."""
        with self._lock:
            if instance_id in self._executions:
                self._executions[instance_id].status = "terminated"
                self._executions[instance_id].end_time = time.strftime(
                    "%Y-%m-%dT%H:%M:%SZ", time.gmtime()
                )

            success = self._destroy_guest_container(instance_id)
            self._containers.pop(instance_id, None)
            return success

    def get_execution_result(self, instance_id: str) -> GuestExecutionResult | None:
        """Get the execution result for an instance."""
        with self._lock:
            return self._executions.get(instance_id)

    def list_executions(
        self,
        account_id: str | None = None,
        region: str | None = None,
    ) -> list[GuestExecutionResult]:
        """List all execution results, optionally filtered."""
        with self._lock:
            results = list(self._executions.values())
            if account_id:
                results = [r for r in results if r.account_id == account_id]
            if region:
                results = [r for r in results if r.region == region]
            return results

    def get_container_id(self, instance_id: str) -> str | None:
        """Get the container ID for an instance."""
        with self._lock:
            return self._containers.get(instance_id)


# Global executor instance
_guest_executor: GuestExecutor | None = None


def get_guest_executor() -> GuestExecutor:
    """Get the global guest executor instance."""
    global _guest_executor
    if _guest_executor is None:
        _guest_executor = GuestExecutor()
    return _guest_executor
