"""EC2 Guest Executor - Pluggable user-data execution for EC2 instances.

This module provides opt-in guest execution for EC2 instances, allowing
user-data scripts (cloud-init) to actually run inside containers when enabled.
"""

from robotocore.services.ec2.guest.executor import GuestExecutor, get_guest_executor
from robotocore.services.ec2.guest.plugin import EC2GuestExecutorPlugin

__all__ = ["GuestExecutor", "get_guest_executor", "EC2GuestExecutorPlugin"]
