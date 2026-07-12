"""Tests for correctness bugs in the Support provider."""

import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from robotocore.services.support import provider as support_module
from robotocore.services.support.provider import (
    _add_communication_to_case,
    _describe_communications,
    handle_support_request,
)

ACCOUNT_ID = "123456789012"


@pytest.fixture(autouse=True)
def _clear_state():
    """Clear global state before each test."""
    support_module._communications.clear()
    yield
    support_module._communications.clear()


# ===================================================================
# Bug 1: DescribeCommunications uses hardcoded region "us-east-1"
#        instead of the region parameter
# ===================================================================


class TestDescribeCommunicationsRegionBug:
    """DescribeCommunications should use the region parameter, not hardcoded us-east-1."""

    @patch("moto.backends.get_backend")
    def test_describe_communications_uses_region_parameter(self, mock_get_backend):
        """BUG: DescribeCommunications ignores region parameter when accessing Moto backend.

        When describing communications for a case in a non-us-east-1 region,
        the function should look up the case in that region's Moto backend,
        but it hardcodes "us-east-1" instead.
        """
        # Set up mock Moto backend for eu-west-1
        mock_case = MagicMock()
        mock_case.communication_body = "Initial case body"
        mock_case.submitted_by = "creator@example.com"
        mock_case.time_created = "2024-01-01T00:00:00Z"

        mock_eu_backend = MagicMock()
        mock_eu_backend.cases = {"case-eu-123": mock_case}

        mock_us_backend = MagicMock()
        mock_us_backend.cases = {}  # Empty in us-east-1

        def mock_backend_getter(service):
            return {
                ACCOUNT_ID: {
                    "eu-west-1": mock_eu_backend,
                    "us-east-1": mock_us_backend,
                }
            }

        mock_get_backend.side_effect = mock_backend_getter

        # Add a communication via the native provider in eu-west-1
        _add_communication_to_case(
            {"caseId": "case-eu-123", "communicationBody": "EU follow-up message"},
            "eu-west-1",
            ACCOUNT_ID,
        )

        # Describe communications in eu-west-1
        result = _describe_communications(
            {"caseId": "case-eu-123"},
            "eu-west-1",
            ACCOUNT_ID,
        )

        # Should find both the native communication AND the initial case from Moto
        # The bug causes it to look in us-east-1 instead of eu-west-1, so it won't find the case
        bodies = [c["body"] for c in result["communications"]]

        # This assertion will fail with the bug because the Moto case won't be found
        # (it's looking in us-east-1 instead of eu-west-1)
        assert "Initial case body" in bodies, (
            f"Expected to find initial case body from Moto backend in eu-west-1, "
            f"but got bodies: {bodies}. The function may be looking in the wrong region."
        )

        # Verify the native communication is also present
        assert "EU follow-up message" in bodies
