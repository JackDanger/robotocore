"""Instance Metadata Service (IMDS) for guest containers.

Provides a minimal IMDS implementation that can be accessed from inside
guest containers at 169.254.169.254.
"""

from __future__ import annotations

import json
import logging
import time
from typing import Any

logger = logging.getLogger(__name__)


class IMDSServer:
    """Minimal IMDS server for guest containers.

    This provides the essential IMDS endpoints that user-data scripts
    typically rely on:
    - /latest/meta-data/instance-id
    - /latest/meta-data/instance-type
    - /latest/meta-data/iam/security-credentials/{role}
    - /latest/meta-data/placement/availability-zone
    - /latest/meta-data/local-ipv4
    - /latest/meta-data/public-ipv4
    - /latest/meta-data/block-device-mapping/
    """

    def __init__(self) -> None:
        self._instance_data: dict[str, dict[str, Any]] = {}

    def register_instance(
        self,
        instance_id: str,
        instance_type: str,
        account_id: str,
        region: str,
        availability_zone: str,
        iam_role_arn: str | None = None,
        private_ip: str = "10.0.0.1",
        public_ip: str | None = None,
        block_devices: list[dict] | None = None,
    ) -> None:
        """Register an instance with the IMDS."""
        self._instance_data[instance_id] = {
            "instance_id": instance_id,
            "instance_type": instance_type,
            "account_id": account_id,
            "region": region,
            "availability_zone": availability_zone,
            "iam_role_arn": iam_role_arn,
            "private_ip": private_ip,
            "public_ip": public_ip,
            "block_devices": block_devices or [],
            "created_at": time.time(),
        }

    def unregister_instance(self, instance_id: str) -> None:
        """Unregister an instance from the IMDS."""
        self._instance_data.pop(instance_id, None)

    def get_metadata(self, instance_id: str, path: str) -> str | None:
        """Get metadata for an instance at the given path."""
        data = self._instance_data.get(instance_id)
        if not data:
            return None

        path = path.strip("/")

        # Handle versioned paths
        if path.startswith("latest/"):
            path = path[7:]  # Remove 'latest/'
        elif path.startswith("1.0/"):
            path = path[4:]

        if path == "meta-data/instance-id":
            return data["instance_id"]

        if path == "meta-data/instance-type":
            return data["instance_type"]

        if path == "meta-data/placement/availability-zone":
            return data["availability_zone"]

        if path == "meta-data/local-ipv4":
            return data["private_ip"]

        if path == "meta-data/public-ipv4":
            return data["public_ip"] or ""

        if path == "meta-data/hostname":
            return f"ip-{data['private_ip'].replace('.', '-')}.ec2.internal"

        if path == "meta-data/local-hostname":
            return f"ip-{data['private_ip'].replace('.', '-')}.ec2.internal"

        if path == "meta-data/public-hostname":
            if data["public_ip"]:
                return f"ec2-{data['public_ip'].replace('.', '-')}.compute-1.amazonaws.com"
            return ""

        if path == "meta-data/ami-id":
            return "ami-12345678"  # Placeholder

        if path == "meta-data/ami-launch-index":
            return "0"

        if path == "meta-data/reservation-id":
            return f"r-{data['instance_id'].split('-')[1]}"

        if path == "meta-data/account-id":
            return data["account_id"]

        if path == "meta-data/iam/info":
            if data["iam_role_arn"]:
                return json.dumps(
                    {
                        "Code": "Success",
                        "LastUpdated": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                        "InstanceProfileArn": data["iam_role_arn"],
                        "InstanceProfileId": "AIP" + data["instance_id"].replace("-", ""),
                    }
                )
            return None

        if path.startswith("meta-data/iam/security-credentials/"):
            # Return mock credentials
            return json.dumps(
                {
                    "Code": "Success",
                    "Type": "AWS-HMAC",
                    "AccessKeyId": f"ASIA{data['instance_id'].replace('-', '').upper()[:16]}",
                    "SecretAccessKey": "mock-secret-key-" + data["instance_id"],
                    "Token": "mock-session-token-" + data["instance_id"],
                    "Expiration": time.strftime(
                        "%Y-%m-%dT%H:%M:%SZ", time.gmtime(time.time() + 3600)
                    ),
                    "LastUpdated": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                }
            )

        if path == "meta-data/block-device-mapping/":
            devices = data.get("block_devices", [])
            return "\n".join(d.get("DeviceName", "").split("/")[-1] for d in devices)

        if path.startswith("meta-data/block-device-mapping/"):
            device_name = path.split("/")[-1]
            for dev in data.get("block_devices", []):
                if dev.get("DeviceName", "").endswith(device_name):
                    return dev.get("Ebs", {}).get("VolumeId", "vol-mock")
            return None

        if path == "meta-data/":
            # Return list of available metadata
            return """ami-id
ami-launch-index
ami-manifest-path
block-device-mapping/
hostname
iam/
instance-action
instance-id
instance-type
local-hostname
local-ipv4
metrics/
network/
placement/
profile
public-hostname
public-ipv4
public-keys/
reservation-id
security-groups
"""

        return None

    def get_user_data(self, instance_id: str) -> str | None:
        """Get user-data for an instance."""
        # User-data is not stored in IMDS, this is just a placeholder
        return None


# Global IMDS server instance
_imds_server: IMDSServer | None = None


def get_imds_server() -> IMDSServer:
    """Get the global IMDS server instance."""
    global _imds_server
    if _imds_server is None:
        _imds_server = IMDSServer()
    return _imds_server
