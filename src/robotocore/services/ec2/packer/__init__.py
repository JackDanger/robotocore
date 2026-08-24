"""Packer-compatible virtual instance transport for EC2.

This module provides a container-backed EC2 instance implementation that
Packer's `amazon-ebs` builder can connect to via SSH or SSM. It enables
real provisioner execution (file upload, shell commands) and AMI creation
with persisted state.

Design notes:
- GPU/driver fidelity is explicitly out of scope; target-hardware acceptance
  (GPU, NVIDIA drivers) remains a real-AWS test.
- This is a minimal implementation that may overlap with a future EC2
  guest/user-data executor feature.
"""

from __future__ import annotations

from .ami_builder import AmiBuilder, AmiBuildResult
from .instance_transport import (
    InstanceTransport,
    InstanceTransportConfig,
    TransportType,
    get_instance_transport,
)

__all__ = [
    "InstanceTransport",
    "InstanceTransportConfig",
    "TransportType",
    "get_instance_transport",
    "AmiBuilder",
    "AmiBuildResult",
]
