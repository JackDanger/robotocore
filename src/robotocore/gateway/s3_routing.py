"""S3 virtual-hosted-style routing.

Parses Host headers to detect S3 virtual-hosted-style requests:
- ``mybucket.s3.localhost.robotocore.cloud`` -> bucket=mybucket
- ``mybucket.s3.us-east-1.amazonaws.com`` -> bucket=mybucket, region=us-east-1
- ``mybucket.s3.amazonaws.com`` -> bucket=mybucket

Also accepts ``mybucket.s3.localhost.localstack.cloud`` as a backwards-compatible alias.

Rewrites the ASGI scope so that downstream handlers see a path-style request.

This module uses the Rust implementation (robotocore_rust) when available,
with a pure Python fallback for compatibility.
"""

import os
import re
import threading
from typing import Any

# Try to import Rust implementation
try:
    import robotocore_rust
    _USE_RUST = True
except ImportError:
    _USE_RUST = False

# Fallback Python implementation constants
DEFAULT_S3_HOSTNAME = "s3.localhost.robotocore.cloud"
S3_LOCALSTACK_HOSTNAME = "s3.localhost.localstack.cloud"

# Patterns for virtual-hosted-style S3 requests (Python fallback)
_VHOST_RE = re.compile(
    r"^(?P<bucket>[a-zA-Z0-9][a-zA-Z0-9.\-]{1,61}[a-zA-Z0-9])"
    r"\.s3(?:\.(?P<region>[a-z]{2}-[a-z]+-\d+))?"
    r"\.(?P<rest>.+?)(?::\d+)?$"
)

_VHOST_CUSTOM_CACHE: tuple[re.Pattern, str] | None = None
_VHOST_CACHE_LOCK = threading.Lock()

_VHOST_LOCALSTACK_RE = re.compile(
    r"^(?P<bucket>[a-zA-Z0-9][a-zA-Z0-9.\-]{1,61}[a-zA-Z0-9])"
    rf"\.{re.escape(S3_LOCALSTACK_HOSTNAME)}(?::\d+)?$"
)


def _get_s3_hostname() -> str:
    """Return the configured S3 hostname base."""
    return os.environ.get("S3_HOSTNAME", DEFAULT_S3_HOSTNAME)


def _get_custom_pattern() -> tuple[re.Pattern, str]:
    """Build and cache the regex for the custom hostname."""
    global _VHOST_CUSTOM_CACHE
    base = _get_s3_hostname()
    if _VHOST_CUSTOM_CACHE is None or _VHOST_CUSTOM_CACHE[1] != base:
        with _VHOST_CACHE_LOCK:
            if _VHOST_CUSTOM_CACHE is None or _VHOST_CUSTOM_CACHE[1] != base:
                escaped = re.escape(base)
                pattern = re.compile(
                    r"^(?P<bucket>[a-zA-Z0-9][a-zA-Z0-9.\-]{1,61}[a-zA-Z0-9])"
                    rf"\.{escaped}(?::\d+)?$"
                )
                _VHOST_CUSTOM_CACHE = (pattern, base)
    return _VHOST_CUSTOM_CACHE


def _parse_s3_vhost_python(host: str) -> dict | None:
    """Python fallback implementation of parse_s3_vhost."""
    if not host:
        return None

    host_no_port = host.rsplit(":", 1)[0] if ":" in host else host

    # Check custom hostname pattern first
    custom_re, base = _get_custom_pattern()
    m = custom_re.match(host)
    if m:
        return {"bucket": m.group("bucket")}

    # Check localstack.cloud alias
    m = _VHOST_LOCALSTACK_RE.match(host)
    if m:
        return {"bucket": m.group("bucket")}

    # Check standard AWS patterns
    m = _VHOST_RE.match(host)
    if m:
        result: dict = {"bucket": m.group("bucket")}
        if m.group("region"):
            result["region"] = m.group("region")
        else:
            rest = m.group("rest")
            region_match = re.search(r"(?:^|\.)((?:us|eu|ap|sa|ca|me|af|il)-[a-z]+-\d+)", rest)
            if region_match:
                result["region"] = region_match.group(1)
        return result

    # Check bare s3 pattern
    if ".s3." in host_no_port:
        parts = host_no_port.split(".s3.", 1)
        if parts[0] and not parts[0].startswith("."):
            bucket = parts[0]
            remainder = parts[1]
            region_match = re.search(r"(?:^|\.)(us|eu|ap|sa|ca|me|af|il)(-[a-z]+-\d+)", remainder)
            result = {"bucket": bucket}
            if region_match:
                result["region"] = region_match.group(1) + region_match.group(2)
            return result

    # S3 Express directory buckets and S3 Object Lambda
    if host_no_port.endswith(".localhost") or ".localhost:" in host:
        label = host_no_port.split(".localhost")[0]
        if label and "." not in label:
            return {"bucket": label}

    return None


def parse_s3_vhost(host: str) -> dict | None:
    """Parse an S3 virtual-hosted-style Host header.

    Returns a dict with keys ``bucket`` and optionally ``region``,
    or ``None`` if the host does not match any S3 pattern.

    Uses the Rust implementation when available for better performance.
    """
    if _USE_RUST:
        return robotocore_rust.parse_s3_vhost(host)
    return _parse_s3_vhost_python(host)


def is_s3_vhost_request(scope: dict) -> bool:
    """Check if an ASGI scope represents an S3 virtual-hosted-style request."""
    if _USE_RUST:
        return robotocore_rust.is_s3_vhost_request(scope)

    if scope.get("type") != "http":
        return False
    host = b""
    for key, val in scope.get("headers", []):
        if key == b"host":
            host = val
            break
    if not host:
        return False
    return parse_s3_vhost(host.decode("latin-1")) is not None


def rewrite_vhost_to_path(scope: dict) -> dict | None:
    """Rewrite a virtual-hosted-style S3 request scope to path-style.

    Returns a new scope dict with the path rewritten to include the bucket,
    or ``None`` if the Host header does not match.

    Uses the Rust implementation when available for better performance.
    """
    if _USE_RUST:
        return robotocore_rust.rewrite_vhost_to_path(scope)

    host = b""
    for key, val in scope.get("headers", []):
        if key == b"host":
            host = val
            break
    if not host:
        return None

    parsed = parse_s3_vhost(host.decode("latin-1"))
    if parsed is None:
        return None

    bucket = parsed["bucket"]
    original_path = scope.get("path", "/")

    if original_path == "/":
        new_path = f"/{bucket}"
    else:
        new_path = f"/{bucket}{original_path}"

    host_str = host.decode("latin-1")
    new_host = host_str[len(bucket) + 1 :]
    new_headers = [
        (b"host", new_host.encode("latin-1")) if k == b"host" else (k, v)
        for k, v in scope.get("headers", [])
    ]

    new_scope = dict(scope)
    new_scope["path"] = new_path
    new_scope["headers"] = new_headers
    qs = scope.get("query_string", b"")
    if qs:
        new_scope["raw_path"] = new_path.encode("utf-8") + b"?" + qs
    else:
        new_scope["raw_path"] = new_path.encode("utf-8")

    return new_scope


def get_s3_routing_config() -> dict:
    """Return the current S3 routing configuration as a JSON-serializable dict."""
    if _USE_RUST:
        return robotocore_rust.get_s3_routing_config()

    return {
        "s3_hostname": _get_s3_hostname(),
        "virtual_hosted_style": True,
        "website_hostname": f"s3-website.{_get_s3_hostname()}",
        "supported_patterns": [
            "<bucket>.s3.<hostname>",
            "<bucket>.s3.<region>.amazonaws.com",
            "<bucket>.s3.amazonaws.com",
        ],
    }


# Expose whether Rust implementation is available
__all__ = ["parse_s3_vhost", "is_s3_vhost_request", "rewrite_vhost_to_path", "get_s3_routing_config", "_USE_RUST"]