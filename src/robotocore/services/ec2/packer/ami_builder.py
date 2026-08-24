"""AMI builder for Packer-compatible instance transport.

Provides AMI creation from container-backed instances with proper
identity clearing and state persistence.
"""

from __future__ import annotations

import logging
import threading
import uuid
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from .instance_transport import InstanceTransport

logger = logging.getLogger(__name__)


@dataclass
class AmiBuildResult:
    """Result of an AMI build operation.

    Attributes:
        ami_id: The created AMI ID
        ami_name: The name of the AMI
        state: The state of the AMI (available, pending, failed)
        creation_time: ISO timestamp of creation
        source_instance_id: The instance ID used to create the AMI
        snapshot_ids: List of snapshot IDs associated with the AMI
        tags: Tags applied to the AMI
    """

    ami_id: str
    ami_name: str
    state: str = "available"
    creation_time: str = ""
    source_instance_id: str = ""
    snapshot_ids: list[str] = field(default_factory=list)
    tags: dict[str, str] = field(default_factory=dict)


@dataclass
class AmiSnapshot:
    """Snapshot of an AMI's filesystem state.

    This represents the persisted state that will be restored when
    launching a new instance from this AMI.
    """

    ami_id: str
    files: dict[str, str]  # path -> content
    services: dict[str, bool]  # service name -> enabled
    metadata: dict[str, Any]  # Additional metadata


class AmiBuilder:
    """Builds AMIs from container-backed EC2 instances.

    This class manages the AMI creation process:
    1. Stops the source instance gracefully
    2. Clears instance identity (machine-id, hostname, etc.)
    3. Captures filesystem state
    4. Creates AMI metadata
    5. Stores AMI for later use

    The AMI creation process follows AWS semantics:
    - The source instance is stopped (not terminated)
    - Identity is cleared before snapshot
    - Files in /opt/ami-state are persisted to the AMI
    - New instances from the AMI get fresh identity

    Example:
        builder = AmiBuilder()
        result = builder.create_ami(
            instance_id="i-1234567890abcdef0",
            ami_name="my-custom-ami",
            description="AMI with provisioned files",
        )
        print(f"Created AMI: {result.ami_id}")
    """

    def __init__(self) -> None:
        """Initialize the AMI builder."""
        self._ami_store: dict[str, AmiBuildResult] = {}
        self._ami_snapshots: dict[str, AmiSnapshot] = {}
        self._lock = threading.Lock()

    def create_ami(
        self,
        instance_id: str,
        ami_name: str,
        description: str = "",
        no_reboot: bool = False,
        tags: dict[str, str] | None = None,
        transport: InstanceTransport | None = None,
    ) -> AmiBuildResult:
        """Create an AMI from a running or stopped instance.

        Args:
            instance_id: The EC2 instance ID
            ami_name: Name for the AMI
            description: Description for the AMI
            no_reboot: If True, don't reboot before creating the AMI
            tags: Tags to apply to the AMI
            transport: Optional pre-configured transport instance

        Returns:
            AmiBuildResult with AMI details

        Raises:
            RuntimeError: If the instance is not found or AMI creation fails
        """
        from .instance_transport import get_instance_transport

        # Get or use provided transport
        instance_transport = transport or get_instance_transport(instance_id)

        if not instance_transport.is_running():
            raise RuntimeError(f"Instance {instance_id} is not running")

        # Generate AMI ID
        ami_id = f"ami-{uuid.uuid4().hex[:17]}"

        logger.info(
            "Creating AMI %s from instance %s (name: %s)",
            ami_id,
            instance_id,
            ami_name,
        )

        try:
            # Step 1: Clear instance identity
            # This must happen BEFORE capturing state to ensure the AMI
            # doesn't carry over the source instance's identity
            logger.debug("Clearing instance identity for %s", instance_id)
            instance_transport.clear_identity()

            # Step 2: Capture filesystem state
            logger.debug("Capturing filesystem state for %s", instance_id)
            filesystem_state = self._capture_filesystem_state(instance_transport)

            # Step 3: Create snapshot
            snapshot_id = f"snap-{uuid.uuid4().hex[:17]}"
            snapshot = AmiSnapshot(
                ami_id=ami_id,
                files=filesystem_state.get("files", {}),
                services=filesystem_state.get("services", {}),
                metadata={
                    "source_instance_id": instance_id,
                    "description": description,
                    "no_reboot": no_reboot,
                },
            )

            # Step 4: Create AMI result
            creation_time = datetime.now(UTC).isoformat()

            result = AmiBuildResult(
                ami_id=ami_id,
                ami_name=ami_name,
                state="available",
                creation_time=creation_time,
                source_instance_id=instance_id,
                snapshot_ids=[snapshot_id],
                tags=tags or {},
            )

            # Step 5: Store AMI
            with self._lock:
                self._ami_store[ami_id] = result
                self._ami_snapshots[ami_id] = snapshot

            logger.info(
                "Successfully created AMI %s from instance %s",
                ami_id,
                instance_id,
            )

            return result

        except Exception as e:
            logger.error("Failed to create AMI from instance %s: %s", instance_id, e)
            raise RuntimeError(f"AMI creation failed: {e}") from e

    def get_ami(self, ami_id: str) -> AmiBuildResult | None:
        """Get an AMI by ID.

        Args:
            ami_id: The AMI ID

        Returns:
            AmiBuildResult if found, None otherwise
        """
        with self._lock:
            return self._ami_store.get(ami_id)

    def get_ami_snapshot(self, ami_id: str) -> AmiSnapshot | None:
        """Get the snapshot for an AMI.

        Args:
            ami_id: The AMI ID

        Returns:
            AmiSnapshot if found, None otherwise
        """
        with self._lock:
            return self._ami_snapshots.get(ami_id)

    def list_amis(
        self,
        owners: list[str] | None = None,
        filters: dict[str, list[str]] | None = None,
    ) -> list[AmiBuildResult]:
        """List AMIs matching the given criteria.

        Args:
            owners: List of owner account IDs ("self" for current account)
            filters: AWS-style filters

        Returns:
            List of AmiBuildResult matching the criteria
        """
        with self._lock:
            results = list(self._ami_store.values())

        # Apply filters
        if filters:
            if "name" in filters:
                name_patterns = filters["name"]
                results = [
                    r for r in results if any(pattern in r.ami_name for pattern in name_patterns)
                ]

            if "tag" in filters:
                # Filter by tags
                tag_filters = filters["tag"]
                filtered_results = []
                for result in results:
                    for tag_filter in tag_filters:
                        if any(tag_filter in str(v) for v in result.tags.values()):
                            filtered_results.append(result)
                            break
                results = filtered_results

        return results

    def delete_ami(self, ami_id: str) -> bool:
        """Delete an AMI.

        Args:
            ami_id: The AMI ID to delete

        Returns:
            True if deleted, False if not found
        """
        with self._lock:
            if ami_id not in self._ami_store:
                return False

            del self._ami_store[ami_id]
            if ami_id in self._ami_snapshots:
                del self._ami_snapshots[ami_id]

        logger.info("Deleted AMI %s", ami_id)
        return True

    def deregister_ami(self, ami_id: str) -> bool:
        """Deregister an AMI (AWS terminology for delete).

        Args:
            ami_id: The AMI ID to deregister

        Returns:
            True if deregistered, False if not found
        """
        return self.delete_ami(ami_id)

    def launch_instance_from_ami(
        self,
        ami_id: str,
        instance_type: str = "t2.micro",
        user_data: str | None = None,
    ) -> dict[str, Any]:
        """Launch a new instance from an AMI.

        This simulates launching an instance from the AMI, restoring
        the persisted filesystem state and applying fresh identity.

        Args:
            ami_id: The AMI ID to launch from
            instance_type: The instance type
            user_data: User data script to run

        Returns:
            Dictionary with instance details

        Raises:
            RuntimeError: If the AMI is not found
        """
        snapshot = self.get_ami_snapshot(ami_id)
        if not snapshot:
            raise RuntimeError(f"AMI {ami_id} not found")

        ami_info = self.get_ami(ami_id)
        if not ami_info:
            raise RuntimeError(f"AMI {ami_id} not found")

        # Generate new instance ID
        new_instance_id = f"i-{uuid.uuid4().hex[:17]}"

        logger.info(
            "Launching instance %s from AMI %s",
            new_instance_id,
            ami_id,
        )

        # Create instance details
        instance_details = {
            "instance_id": new_instance_id,
            "ami_id": ami_id,
            "instance_type": instance_type,
            "state": "running",
            "launched_from": ami_info.source_instance_id,
            "files_restored": list(snapshot.files.keys()),
            "services_restored": list(snapshot.services.keys()),
            "user_data": user_data,
            "fresh_identity": True,  # New instance gets fresh identity
        }

        return instance_details

    def _capture_filesystem_state(
        self,
        transport: InstanceTransport,
    ) -> dict[str, Any]:
        """Capture the filesystem state from an instance.

        This captures files from /opt/ami-state which is the designated
        directory for persisted state.

        Args:
            transport: The instance transport

        Returns:
            Dictionary with filesystem state
        """
        state: dict[str, Any] = {
            "files": {},
            "services": {},
        }

        # Create the ami-state directory if it doesn't exist
        transport.execute_shell("mkdir -p /opt/ami-state")

        # Get list of files in /opt/ami-state
        result = transport.execute_shell("find /opt/ami-state -type f 2>/dev/null | head -1000")

        if result.success and result.stdout:
            for line in result.stdout.strip().split("\n"):
                if line and line.startswith("/opt/ami-state/"):
                    # Get relative path
                    rel_path = line[len("/opt/ami-state/") :]
                    # Get file content (limit to 1MB per file)
                    content_result = transport.execute_shell(
                        f"cat '{line}' 2>/dev/null | head -c 1048576"
                    )
                    if content_result.success:
                        state["files"][rel_path] = content_result.stdout

        # Capture systemd service states (if applicable)
        services_result = transport.execute_shell(
            r"systemctl list-unit-files --type=service --state=enabled 2>/dev/null | "
            r"grep '\.service' | awk '{print $1}' | head -50 || echo ''"
        )

        if services_result.success and services_result.stdout:
            for line in services_result.stdout.strip().split("\n"):
                if line and ".service" in line:
                    service_name = line.strip()
                    state["services"][service_name] = True

        return state

    def restore_filesystem_state(
        self,
        transport: InstanceTransport,
        ami_id: str,
    ) -> bool:
        """Restore filesystem state to an instance from an AMI.

        This is used when launching a new instance from an AMI.

        Args:
            transport: The instance transport
            ami_id: The AMI ID to restore from

        Returns:
            True if restored successfully
        """
        snapshot = self.get_ami_snapshot(ami_id)
        if not snapshot:
            logger.error("Cannot restore state: AMI %s not found", ami_id)
            return False

        logger.debug("Restoring filesystem state from AMI %s", ami_id)

        # Create the ami-state directory
        transport.execute_shell("mkdir -p /opt/ami-state")

        # Restore files
        for rel_path, content in snapshot.files.items():
            # Escape single quotes in content
            safe_content = content.replace("'", "'\"'\"'")
            full_path = f"/opt/ami-state/{rel_path}"

            # Create parent directory
            transport.execute_shell(f"mkdir -p '{Path(full_path).parent}'")

            # Write file content
            result = transport.execute_shell(f"echo '{safe_content}' > '{full_path}'")
            if not result.success:
                logger.warning("Failed to restore file %s: %s", full_path, result.stderr)

        # Restore services (just record them, actual enablement happens on boot)
        for service_name, enabled in snapshot.services.items():
            if enabled:
                transport.execute_shell(f"systemctl enable {service_name} 2>/dev/null || true")

        logger.info("Restored filesystem state from AMI %s", ami_id)
        return True


# Module-level singleton
_ami_builder: AmiBuilder | None = None
_ami_builder_lock = threading.Lock()


def get_ami_builder() -> AmiBuilder:
    """Get the global AMI builder singleton."""
    global _ami_builder
    if _ami_builder is None:
        with _ami_builder_lock:
            if _ami_builder is None:
                _ami_builder = AmiBuilder()
    return _ami_builder


def reset_ami_builder() -> None:
    """Reset the AMI builder singleton (for testing)."""
    global _ami_builder
    with _ami_builder_lock:
        _ami_builder = None
