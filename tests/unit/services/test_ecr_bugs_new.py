"""Tests for ECR provider bugs found during review.

Bug 1 (FIXED): DescribeRepositories pagination nextToken was hardcoded.
When the provider truncated results for pagination, it set nextToken to a
hardcoded string "pagination-token" instead of a meaningful token.
This broke pagination because the token didn't encode the actual position.
"""

import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from robotocore.services.ecr.provider import handle_ecr_request


def _make_request(action: str, body: dict | None = None) -> MagicMock:
    req = MagicMock()
    req.headers = {
        "x-amz-target": f"AmazonEC2ContainerRegistry_V20150921.{action}",
    }
    req.method = "POST"
    req.url = MagicMock()
    req.url.path = "/"
    req.query_params = {}
    payload = json.dumps(body or {}).encode()
    req.body = AsyncMock(return_value=payload)
    return req


@pytest.mark.asyncio
class TestDescribeRepositoriesPagination:
    """Bug 1 (FIXED): DescribeRepositories pagination nextToken was hardcoded."""

    async def test_next_token_encodes_position(self):
        """When results are truncated, nextToken should encode the position."""
        req = _make_request(
            "DescribeRepositories",
            {"maxResults": 5},
        )

        # Mock Moto returning 10 repositories
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.body = json.dumps({
            "repositories": [{"repositoryName": f"repo-{i}"} for i in range(10)]
        }).encode()

        with patch(
            "robotocore.services.ecr.provider.forward_to_moto",
            return_value=mock_response,
        ):
            resp = await handle_ecr_request(req, "us-east-1", "123456789012")
            assert resp.status_code == 200
            body = json.loads(resp.body)

            # The response should have a nextToken since we truncated
            assert "nextToken" in body, "Response should have nextToken when truncated"

            # The token should encode the position (5 = next index after first 5)
            assert body["nextToken"] == "5", (
                f"nextToken should encode the position, got: {body['nextToken']}"
            )

            # Also verify we only got 5 repos
            assert len(body["repositories"]) == 5

    async def test_next_token_is_used_for_offset(self):
        """When nextToken is provided, it should be used as the offset."""
        req = _make_request(
            "DescribeRepositories",
            {"maxResults": 5, "nextToken": "5"},
        )

        # Mock Moto returning 10 repositories
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.body = json.dumps({
            "repositories": [{"repositoryName": f"repo-{i}"} for i in range(10)]
        }).encode()

        with patch(
            "robotocore.services.ecr.provider.forward_to_moto",
            return_value=mock_response,
        ):
            resp = await handle_ecr_request(req, "us-east-1", "123456789012")
            assert resp.status_code == 200
            body = json.loads(resp.body)

            # Should get repos 5-9 (the second page)
            assert len(body["repositories"]) == 5
            assert body["repositories"][0]["repositoryName"] == "repo-5"

            # No more results, so no nextToken
            assert "nextToken" not in body
