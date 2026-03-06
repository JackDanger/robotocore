"""Robotocore extension system — third-party plugins and service overrides."""

from robotocore.extensions.base import RobotocorePlugin
from robotocore.extensions.registry import (
    discover_extensions,
    get_extension_registry,
    register_extension,
)

__all__ = [
    "RobotocorePlugin",
    "discover_extensions",
    "get_extension_registry",
    "register_extension",
]
