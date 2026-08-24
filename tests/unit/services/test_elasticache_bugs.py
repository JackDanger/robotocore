"""Tests for ElastiCache provider bugs found during review.

Regression guard: SET with NX must succeed once a previously-set key has expired
(lazy expiry is already correctly checked before the NX existence check).

Bug 2: Action parameter in query string is ignored when content-type is form-urlencoded.
When the Action parameter is in the query string but other parameters are in the body,
the provider only looks at the body and ignores the query string.
"""

import time
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from robotocore.services.elasticache.provider import handle_elasticache_request
from robotocore.services.elasticache.redis_compat import RedisCompatStore


def _make_request(
    method: str, query_string: str = "", body: bytes = b"", content_type: str = ""
) -> MagicMock:
    """Create a mock request for ElastiCache."""
    req = MagicMock()
    req.method = method
    req.url = MagicMock()
    req.url.query = query_string
    req.headers = {}
    if content_type:
        req.headers["content-type"] = content_type
    req.body = AsyncMock(return_value=body)
    return req


class TestSetNxExpiryBug:
    """Regression guard (not a bug fix): lazy expiry is checked before NX existence."""

    def test_set_nx_on_expired_key_should_succeed(self):
        """SET with NX should succeed if the key has expired."""
        store = RedisCompatStore()

        # Set a key with a short expiry
        store.execute_command("SET", "mykey", "old_value", "EX", "1")

        # Wait for the key to expire
        time.sleep(1.1)

        # The key should be expired now
        assert store.execute_command("GET", "mykey") is None

        # SET with NX should succeed because the key has expired
        result = store.execute_command("SET", "mykey", "new_value", "NX")
        assert result == "OK"

        # Verify the new value was set
        assert store.execute_command("GET", "mykey") == "new_value"


@pytest.mark.asyncio
class TestActionParameterParsing:
    """Bug 2 (FIXED): Action parameter in query string was ignored when body is form-urlencoded."""

    async def test_action_in_query_string_with_form_body(self):
        """Action in query string should be recognized even with form body."""
        # Action in query string, other params in body
        req = _make_request(
            "POST",
            query_string="Action=CreateCacheCluster",
            body=b"CacheClusterId=my-cluster",
            content_type="application/x-www-form-urlencoded",
        )

        # Mock the forward_to_moto response
        mock_response = MagicMock()
        mock_response.status_code = 200

        with patch(
            "robotocore.services.elasticache.provider.forward_to_moto",
            return_value=mock_response,
        ):
            # Also patch _create_store_for_cluster to verify it's called
            with patch(
                "robotocore.services.elasticache.provider._create_store_for_cluster",
            ) as mock_create:
                resp = await handle_elasticache_request(req, "us-east-1", "123456789012")
                assert resp.status_code == 200

                # The Action parameter is in the query string, and the provider
                # should now recognize it even when the body is form-urlencoded.
                assert mock_create.called, (
                    "Action parameter in query string should be recognized "
                    "even when body is form-urlencoded."
                )
