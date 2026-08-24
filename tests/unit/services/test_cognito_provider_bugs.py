"""Tests for correctness bugs in the Cognito Identity Provider."""

import base64
import hashlib
import hmac

from robotocore.services.cognito.provider import (
    CognitoStore,
    _create_user_pool,
    _create_user_pool_client,
    _describe_user_pool_client,
    _update_user_pool_client,
)


class TestSecretHashBug:
    def test_secret_hash_uses_hmac_sha256(self):
        """_secret_hash should use HMAC-SHA256 as AWS requires, not plain SHA-256."""
        from robotocore.services.cognito.provider import _secret_hash

        username = "testuser"
        client_id = "abc123"
        client_secret = "supersecret"

        # Correct AWS implementation: HMAC-SHA256
        msg = (username + client_id).encode("utf-8")
        expected = base64.b64encode(
            hmac.new(client_secret.encode("utf-8"), msg, hashlib.sha256).digest()
        ).decode("utf-8")

        actual = _secret_hash(username, client_id, client_secret)
        assert actual == expected, (
            f"_secret_hash should use HMAC-SHA256 but got wrong result. "
            f"Expected {expected}, got {actual}"
        )


# ===========================================================================
# Bug 2: _create_user_pool_client doesn't initialize all updatable fields
# ===========================================================================


class TestCreateUserPoolClientInitializesAllFields:
    """When creating a user pool client, all fields that can be updated
    via _update_user_pool_client should be initialized. Otherwise, describe
    operations won't return those fields even after they've been updated.
    """

    def test_create_client_initializes_all_updatable_fields(self):
        """After creating a client, all updatable fields should exist
        (even if empty), so that describe operations return consistent results."""
        store = CognitoStore()
        region = "us-east-1"
        account_id = "123456789012"

        # First create a pool
        pool_result = _create_user_pool(store, {"PoolName": "test-pool"}, region, account_id)
        pool_id = pool_result["UserPool"]["Id"]

        # Create a client
        result = _create_user_pool_client(
            store,
            {"UserPoolId": pool_id, "ClientName": "test-client"},
            region,
            account_id,
        )
        client = result["UserPoolClient"]

        # These fields are in the updatable list but not initialized
        # They should be initialized to empty values
        assert "LogoutURLs" in client, "LogoutURLs should be initialized"
        assert "DefaultRedirectURI" in client, "DefaultRedirectURI should be initialized"
        assert "ReadAttributes" in client, "ReadAttributes should be initialized"
        assert "WriteAttributes" in client, "WriteAttributes should be initialized"
        assert "SupportedIdentityProviders" in client, (
            "SupportedIdentityProviders should be initialized"
        )
        assert "AllowedOAuthFlowsUserPoolClient" in client, (
            "AllowedOAuthFlowsUserPoolClient should be initialized"
        )
        assert "TokenValidityUnits" in client, "TokenValidityUnits should be initialized"
        assert "AccessTokenValidity" in client, "AccessTokenValidity should be initialized"
        assert "IdTokenValidity" in client, "IdTokenValidity should be initialized"
        assert "RefreshTokenValidity" in client, "RefreshTokenValidity should be initialized"

    def test_describe_returns_updated_fields(self):
        """After updating a client, describe should return the updated fields."""
        store = CognitoStore()
        region = "us-east-1"
        account_id = "123456789012"

        # First create a pool
        pool_result = _create_user_pool(store, {"PoolName": "test-pool"}, region, account_id)
        pool_id = pool_result["UserPool"]["Id"]

        # Create a client
        create_result = _create_user_pool_client(
            store,
            {"UserPoolId": pool_id, "ClientName": "test-client"},
            region,
            account_id,
        )
        client_id = create_result["UserPoolClient"]["ClientId"]

        # Update the client with new fields
        _update_user_pool_client(
            store,
            {
                "UserPoolId": pool_id,
                "ClientId": client_id,
                "LogoutURLs": ["https://example.com/logout"],
                "ReadAttributes": ["email", "name"],
                "AccessTokenValidity": 60,
            },
            region,
            account_id,
        )

        # Describe should return the updated fields
        describe_result = _describe_user_pool_client(
            store, {"UserPoolId": pool_id, "ClientId": client_id}, region, account_id
        )
        client = describe_result["UserPoolClient"]

        assert client.get("LogoutURLs") == ["https://example.com/logout"], (
            "LogoutURLs should be returned after update"
        )
        assert client.get("ReadAttributes") == ["email", "name"], (
            "ReadAttributes should be returned after update"
        )
        assert client.get("AccessTokenValidity") == 60, (
            "AccessTokenValidity should be returned after update"
        )
