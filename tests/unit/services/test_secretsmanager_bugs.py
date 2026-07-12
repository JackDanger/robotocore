"""Tests for correctness bugs in the SecretsManager native provider."""

import json
from unittest.mock import AsyncMock, MagicMock

import pytest

from robotocore.services.secretsmanager.provider import handle_secretsmanager_request


def _make_request(action: str, body: dict | None = None) -> MagicMock:
    req = MagicMock()
    req.headers = {"x-amz-target": f"secretsmanager.{action}"}
    req.method = "POST"
    req.url = MagicMock()
    req.url.path = "/"
    req.query_params = {}
    payload = json.dumps(body or {}).encode()
    req.body = AsyncMock(return_value=payload)
    return req


def _get_backend():
    from moto.backends import get_backend  # noqa: I001

    return get_backend("secretsmanager")["123456789012"]["us-east-1"]


def _create_test_secret(
    backend, name: str, secret_string: str | None = None, secret_binary: str | None = None
):
    """Helper to create a secret in Moto backend."""
    backend.create_secret(
        name=name,
        secret_string=secret_string,
        secret_binary=secret_binary,
        description=None,
        tags=None,
        kms_key_id=None,
        client_request_token=None,
        replica_regions=[],
        force_overwrite=False,
    )


def _delete_test_secret(backend, name: str):
    """Helper to force-delete a test secret."""
    backend.delete_secret(
        secret_id=name,
        recovery_window_in_days=None,
        force_delete_without_recovery=True,
    )


# ===========================================================================
# Bug 1: RotateSecret doesn't preserve secret_binary
# ===========================================================================


@pytest.mark.asyncio
class TestRotateSecretPreservesBinary:
    """RotateSecret must preserve both secret_string AND secret_binary when creating
    the new AWSPENDING version. Currently only secret_string is copied.
    """

    async def test_rotate_preserves_secret_binary(self):
        """When rotating a secret with secret_binary, the new version should
        preserve the binary data, not lose it."""
        backend = _get_backend()
        _create_test_secret(backend, "binary-test-secret", secret_binary="aGVsbG8=")

        req = _make_request(
            "RotateSecret",
            {
                "SecretId": "binary-test-secret",
                "RotationLambdaARN": "arn:aws:lambda:us-east-1:123456789012:function:myRotator",
                "RotationRules": {"AutomaticallyAfterDays": 30},
            },
        )
        resp = await handle_secretsmanager_request(req, "us-east-1", "123456789012")
        assert resp.status_code == 200

        # Check that the secret still has its binary data in the new version
        secret = backend.secrets.get("binary-test-secret")
        # Find the AWSPENDING version
        pending_version = None
        for vid, vdata in secret.versions.items():
            if "AWSPENDING" in vdata.get("version_stages", []):
                pending_version = vdata
                break

        assert pending_version is not None, "AWSPENDING version should exist after rotation"
        # The binary data should be preserved
        assert pending_version.get("secret_binary") == "aGVsbG8=", (
            "secret_binary should be preserved in the new AWSPENDING version"
        )

        # Cleanup
        _delete_test_secret(backend, "binary-test-secret")

    async def test_rotate_preserves_secret_string(self):
        """When rotating a secret with secret_string, the new version should
        preserve the string data (this already works, testing for regression)."""
        backend = _get_backend()
        _create_test_secret(backend, "string-test-secret", secret_string="my-secret-value")

        req = _make_request(
            "RotateSecret",
            {
                "SecretId": "string-test-secret",
                "RotationLambdaARN": "arn:aws:lambda:us-east-1:123456789012:function:myRotator",
                "RotationRules": {"AutomaticallyAfterDays": 30},
            },
        )
        resp = await handle_secretsmanager_request(req, "us-east-1", "123456789012")
        assert resp.status_code == 200

        # Check that the secret still has its string data in the new version
        secret = backend.secrets.get("string-test-secret")
        # Find the AWSPENDING version
        pending_version = None
        for vid, vdata in secret.versions.items():
            if "AWSPENDING" in vdata.get("version_stages", []):
                pending_version = vdata
                break

        assert pending_version is not None, "AWSPENDING version should exist after rotation"
        # The string data should be preserved
        assert pending_version.get("secret_string") == "my-secret-value", (
            "secret_string should be preserved in the new AWSPENDING version"
        )

        # Cleanup
        _delete_test_secret(backend, "string-test-secret")
