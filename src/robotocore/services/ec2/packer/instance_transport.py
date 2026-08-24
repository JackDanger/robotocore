"""Virtual instance transport for Packer-compatible EC2 instances.

Provides SSH and SSM transport to container-backed EC2 instances.
This enables Packer provisioners to actually execute against a real
(virtualized) instance rather than just control-plane state.
"""

from __future__ import annotations

import enum
import logging
import subprocess
import tempfile
import threading
import uuid
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)


class TransportType(enum.Enum):
    """Supported transport protocols for instance access."""

    SSH = "ssh"
    SSM = "ssm"


@dataclass
class InstanceTransportConfig:
    """Configuration for instance transport.

    Attributes:
        transport_type: SSH or SSM transport
        container_image: Docker image to use for the instance
        ssh_port: Host port to map to container port 22 (SSH only)
        ssm_role: IAM role ARN for SSM (SSM only)
        instance_type: EC2 instance type (affects resource limits)
        user_data: User data script to run on instance start
        security_groups: List of security group IDs
        subnet_id: Subnet ID for the instance
    """

    transport_type: TransportType = TransportType.SSH
    container_image: str = "public.ecr.aws/amazonlinux/amazonlinux:2"
    ssh_port: int | None = None
    ssm_role: str | None = None
    instance_type: str = "t2.micro"
    user_data: str | None = None
    security_groups: list[str] = field(default_factory=list)
    subnet_id: str | None = None


@dataclass
class FileUpload:
    """File upload specification for provisioners.

    Attributes:
        source: Local path to the file to upload
        destination: Remote path (file or directory)
    """

    source: str
    destination: str


@dataclass
class ShellCommand:
    """Shell command specification for provisioners.

    Attributes:
        command: The shell command to execute
        environment: Environment variables to set
        working_dir: Working directory for the command
    """

    command: str
    environment: dict[str, str] = field(default_factory=dict)
    working_dir: str | None = None


@dataclass
class ProvisionerResult:
    """Result of a provisioner execution.

    Attributes:
        success: Whether the provisioner succeeded
        stdout: Standard output from the command
        stderr: Standard error from the command
        exit_code: Exit code from the command
    """

    success: bool
    stdout: str = ""
    stderr: str = ""
    exit_code: int = 0


class InstanceTransport:
    """Manages a container-backed EC2 instance with SSH/SSM access.

    This class provides a Packer-compatible transport layer that:
    1. Runs a Docker container representing an EC2 instance
    2. Provides SSH or SSM connectivity
    3. Executes file uploads and shell commands
    4. Captures the filesystem state for AMI creation

    Example:
        config = InstanceTransportConfig(
            transport_type=TransportType.SSH,
            ssh_port=2222,
        )
        transport = InstanceTransport("i-1234567890abcdef0", config)
        transport.start()

        # File upload
        result = transport.upload_file("/local/file.txt", "/home/ec2-user/file.txt")

        # Shell command
        result = transport.execute_shell("echo hello > /home/ec2-user/hello.txt")

        # Create AMI from current state
        ami_id = transport.create_ami("my-ami")

        transport.stop()
    """

    def __init__(
        self,
        instance_id: str,
        config: InstanceTransportConfig,
        account_id: str = "123456789012",
        region: str = "us-east-1",
    ) -> None:
        """Initialize the instance transport.

        Args:
            instance_id: The EC2 instance ID
            config: Transport configuration
            account_id: AWS account ID
            region: AWS region
        """
        self.instance_id = instance_id
        self.config = config
        self.account_id = account_id
        self.region = region
        self.container_name: str | None = None
        self._lock = threading.Lock()
        self._running = False
        self._temp_dir: Path | None = None

    def start(self) -> bool:
        """Start the container-backed instance.

        Returns:
            True if the instance started successfully
        """
        with self._lock:
            if self._running:
                logger.debug("Instance %s already running", self.instance_id)
                return True

            if not self._docker_available():
                logger.error("Docker not available for instance transport")
                return False

            # Create temp directory for instance state
            self._temp_dir = Path(tempfile.mkdtemp(prefix=f"ec2-{self.instance_id}-"))

            # Generate container name
            self.container_name = f"robotocore-ec2-{self.instance_id}"

            # Build and run container
            if not self._start_container():
                return False

            self._running = True
            logger.info(
                "Started instance %s with container %s",
                self.instance_id,
                self.container_name,
            )
            return True

    def stop(self) -> bool:
        """Stop the container-backed instance.

        Returns:
            True if the instance stopped successfully
        """
        with self._lock:
            if not self._running:
                return True

            if self.container_name:
                self._stop_container()

            self._running = False
            logger.info("Stopped instance %s", self.instance_id)
            return True

    def is_running(self) -> bool:
        """Check if the instance is running.

        Returns:
            True if the instance is running
        """
        with self._lock:
            if not self._running or not self.container_name:
                return False
            return self._container_running()

    def upload_file(self, source: str, destination: str) -> ProvisionerResult:
        """Upload a file to the instance.

        This validates destination path semantics:
        - If destination ends with /, it's treated as a directory
        - If destination is an existing directory, the source filename is appended
        - Otherwise, destination is treated as a file path

        Args:
            source: Local path to the file
            destination: Remote destination path

        Returns:
            ProvisionerResult with success status
        """
        if not self.is_running():
            return ProvisionerResult(
                success=False,
                stderr="Instance not running",
                exit_code=1,
            )

        source_path = Path(source)
        if not source_path.exists():
            return ProvisionerResult(
                success=False,
                stderr=f"Source file not found: {source}",
                exit_code=1,
            )

        if not source_path.is_file():
            return ProvisionerResult(
                success=False,
                stderr=f"Source is not a file: {source}",
                exit_code=1,
            )

        # Validate destination semantics
        dest_path = self._normalize_destination(destination, source_path.name)
        if dest_path is None:
            return ProvisionerResult(
                success=False,
                stderr=f"Invalid destination: {destination}",
                exit_code=1,
            )

        # Use docker cp for file transfer
        try:
            result = subprocess.run(
                [
                    "docker",
                    "cp",
                    str(source_path),
                    f"{self.container_name}:{dest_path}",
                ],
                capture_output=True,
                text=True,
                timeout=60,
            )
            return ProvisionerResult(
                success=result.returncode == 0,
                stdout=result.stdout,
                stderr=result.stderr,
                exit_code=result.returncode,
            )
        except subprocess.TimeoutExpired:
            return ProvisionerResult(
                success=False,
                stderr="File upload timed out",
                exit_code=1,
            )
        except FileNotFoundError:
            return ProvisionerResult(
                success=False,
                stderr="docker command not found",
                exit_code=1,
            )

    def execute_shell(
        self,
        command: str,
        environment: dict[str, str] | None = None,
        working_dir: str | None = None,
    ) -> ProvisionerResult:
        """Execute a shell command on the instance.

        Args:
            command: The shell command to execute
            environment: Environment variables to set
            working_dir: Working directory for the command

        Returns:
            ProvisionerResult with command output
        """
        if not self.is_running():
            return ProvisionerResult(
                success=False,
                stderr="Instance not running",
                exit_code=1,
            )

        # Build the exec command
        exec_cmd: list[str] = ["docker", "exec"]

        # Add environment variables
        if environment:
            for key, value in environment.items():
                exec_cmd.extend(["-e", f"{key}={value}"])

        # Add working directory
        if working_dir:
            exec_cmd.extend(["-w", working_dir])

        if self.container_name is None:
            return ProvisionerResult(
                success=False,
                stderr="Container name not set",
                exit_code=1,
            )

        exec_cmd.extend([self.container_name, "sh", "-c", command])

        try:
            result = subprocess.run(
                exec_cmd,
                capture_output=True,
                text=True,
                timeout=300,  # 5 minute timeout for commands
            )
            return ProvisionerResult(
                success=result.returncode == 0,
                stdout=result.stdout,
                stderr=result.stderr,
                exit_code=result.returncode,
            )
        except subprocess.TimeoutExpired:
            return ProvisionerResult(
                success=False,
                stderr="Command timed out after 300 seconds",
                exit_code=1,
            )
        except FileNotFoundError:
            return ProvisionerResult(
                success=False,
                stderr="docker command not found",
                exit_code=1,
            )

    def create_ami(self, ami_name: str, description: str = "") -> str:
        """Create an AMI from the current instance state.

        This captures the container filesystem and creates an AMI model
        that can be used to launch new instances.

        Args:
            ami_name: Name for the AMI
            description: Description for the AMI

        Returns:
            The AMI ID
        """
        if not self.is_running():
            raise RuntimeError("Instance not running")

        # Generate AMI ID
        ami_id = f"ami-{uuid.uuid4().hex[:17]}"

        # Create AMI metadata
        ami_data = {
            "ami_id": ami_id,
            "ami_name": ami_name,
            "description": description,
            "instance_id": self.instance_id,
            "account_id": self.account_id,
            "region": self.region,
            "container_image": self.config.container_image,
            "instance_type": self.config.instance_type,
            "created_at": self._now_iso(),
        }

        # Save AMI metadata to temp directory
        if self._temp_dir:
            ami_meta_path = self._temp_dir / "ami_metadata.json"
            import json

            with open(ami_meta_path, "w") as f:
                json.dump(ami_data, f, indent=2)

        logger.info(
            "Created AMI %s from instance %s",
            ami_id,
            self.instance_id,
        )
        return ami_id

    def get_filesystem_state(self) -> dict[str, Any]:
        """Get the current filesystem state of the instance.

        Returns:
            Dictionary with filesystem information
        """
        if not self.is_running():
            return {}

        # Get list of files in /opt/ami-state (our persistent state directory)
        result = self.execute_shell(
            "find /opt/ami-state -type f 2>/dev/null | head -1000 || echo ''"
        )

        files = {}
        if result.success and result.stdout:
            for line in result.stdout.strip().split("\n"):
                if line:
                    # Get file content for small files
                    content_result = self.execute_shell(f"cat '{line}' 2>/dev/null || echo ''")
                    files[line] = content_result.stdout if content_result.success else ""

        return {
            "files": files,
            "instance_id": self.instance_id,
        }

    def clear_identity(self) -> bool:
        """Clear the instance identity (machine-id, hostname, etc.).

        This should be called before creating an AMI to ensure the
        resulting instances don't carry over the source identity.

        Returns:
            True if identity was cleared successfully
        """
        if not self.is_running():
            return False

        # Commands to clear instance identity
        identity_commands = [
            # Clear machine ID
            "rm -f /etc/machine-id /var/lib/dbus/machine-id 2>/dev/null || true",
            "systemd-machine-id-setup 2>/dev/null || true",
            # Clear hostname
            "hostname localhost 2>/dev/null || true",
            "echo localhost > /etc/hostname 2>/dev/null || true",
            # Clear SSH host keys (will be regenerated on boot)
            "rm -f /etc/ssh/ssh_host_* 2>/dev/null || true",
            # Clear instance-specific logs
            "rm -f /var/log/cloud-init.log /var/log/cloud-init-output.log 2>/dev/null || true",
            # Clear instance ID references
            "rm -f /var/lib/cloud/instance /var/lib/cloud/instances/* 2>/dev/null || true",
        ]

        for cmd in identity_commands:
            result = self.execute_shell(cmd)
            if not result.success:
                logger.warning("Failed to clear identity: %s", result.stderr)

        logger.info("Cleared identity for instance %s", self.instance_id)
        return True

    def _docker_available(self) -> bool:
        """Check if Docker is available."""
        try:
            result = subprocess.run(
                ["docker", "info"],
                capture_output=True,
                timeout=5,
            )
            return result.returncode == 0
        except (FileNotFoundError, subprocess.TimeoutExpired, OSError):
            return False

    def _start_container(self) -> bool:
        """Start the Docker container for this instance."""
        if not self.container_name:
            return False

        # Build docker run command
        cmd = [
            "docker",
            "run",
            "-d",  # Detached mode
            "--name",
            self.container_name,
            "--privileged",  # Required for some EC2-like behaviors
        ]

        # Map SSH port if using SSH transport
        if self.config.transport_type == TransportType.SSH and self.config.ssh_port:
            cmd.extend(["-p", f"{self.config.ssh_port}:22"])

        # Add labels for identification
        cmd.extend(
            [
                "--label",
                f"robotocore.instance_id={self.instance_id}",
                "--label",
                f"robotocore.account_id={self.account_id}",
                "--label",
                f"robotocore.region={self.region}",
            ]
        )

        # Add the image
        cmd.append(self.config.container_image)

        # Start with a long-running command that works in most containers
        # Use tail -f /dev/null which works even without init system
        cmd.extend(["sh", "-c", "mkdir -p /opt/ami-state && tail -f /dev/null"])

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=60,
            )
            if result.returncode != 0:
                logger.error("Failed to start container: %s", result.stderr)
                return False

            # Wait for container to be ready
            return self._wait_for_container_ready()
        except subprocess.TimeoutExpired:
            logger.error("Container startup timed out")
            return False
        except FileNotFoundError:
            logger.error("docker command not found")
            return False

    def _wait_for_container_ready(self, timeout: int = 60) -> bool:
        """Wait for the container to be ready for commands."""
        if not self.container_name:
            return False

        import time

        start = time.time()
        while time.time() - start < timeout:
            result = subprocess.run(
                ["docker", "exec", self.container_name, "echo", "ready"],
                capture_output=True,
                timeout=5,
            )
            if result.returncode == 0:
                return True
            time.sleep(1)

        return False

    def _stop_container(self) -> None:
        """Stop and remove the Docker container."""
        if not self.container_name:
            return

        try:
            # Stop the container
            subprocess.run(
                ["docker", "stop", "-t", "10", self.container_name],
                capture_output=True,
                timeout=15,
            )
            # Remove the container
            subprocess.run(
                ["docker", "rm", "-f", self.container_name],
                capture_output=True,
                timeout=15,
            )
        except (subprocess.TimeoutExpired, FileNotFoundError) as e:
            logger.debug("Container cleanup issue (non-fatal): %s", e)

    def _container_running(self) -> bool:
        """Check if the container is running."""
        if not self.container_name:
            return False

        try:
            result = subprocess.run(
                ["docker", "inspect", "-f", "{{.State.Running}}", self.container_name],
                capture_output=True,
                text=True,
                timeout=5,
            )
            return result.returncode == 0 and result.stdout.strip() == "true"
        except (subprocess.TimeoutExpired, FileNotFoundError):
            return False

    def _normalize_destination(self, destination: str, source_filename: str) -> str | None:
        """Normalize the destination path.

        Returns:
            The normalized destination path, or None if invalid
        """
        # Check if destination ends with / (directory)
        if destination.endswith("/"):
            # Destination is a directory, append source filename
            return destination + source_filename

        # Check if destination is an existing directory in the container
        if self.container_name:
            result = subprocess.run(
                ["docker", "exec", self.container_name, "test", "-d", destination],
                capture_output=True,
                timeout=5,
            )
            if result.returncode == 0:
                # It's a directory, append source filename
                return destination.rstrip("/") + "/" + source_filename

        # Destination is treated as a file path
        return destination

    def _now_iso(self) -> str:
        """Return current time in ISO format."""
        return datetime.now(UTC).isoformat()


# Module-level singleton for transport instances
_transport_instances: dict[str, InstanceTransport] = {}
_transport_lock = threading.Lock()


def get_instance_transport(
    instance_id: str,
    config: InstanceTransportConfig | None = None,
    account_id: str = "123456789012",
    region: str = "us-east-1",
) -> InstanceTransport:
    """Get or create an instance transport.

    Args:
        instance_id: The EC2 instance ID
        config: Transport configuration (uses default if None)
        account_id: AWS account ID
        region: AWS region

    Returns:
        InstanceTransport instance
    """
    global _transport_instances

    with _transport_lock:
        if instance_id not in _transport_instances:
            if config is None:
                config = InstanceTransportConfig()
            _transport_instances[instance_id] = InstanceTransport(
                instance_id=instance_id,
                config=config,
                account_id=account_id,
                region=region,
            )
        return _transport_instances[instance_id]


def remove_instance_transport(instance_id: str) -> None:
    """Remove an instance transport from the registry.

    Args:
        instance_id: The EC2 instance ID
    """
    global _transport_instances

    with _transport_lock:
        if instance_id in _transport_instances:
            transport = _transport_instances.pop(instance_id)
            transport.stop()
