"""Tests for X-Ray provider bug fixes.

Each test targets a specific bug that has been fixed.
"""

import json
from unittest.mock import AsyncMock, MagicMock

import pytest

from robotocore.services.xray import provider as xray_provider
from robotocore.services.xray.provider import handle_xray_request


@pytest.fixture(autouse=True)
def _reset_xray_state():
    """Reset module-level state between tests to prevent cross-test pollution."""
    xray_provider._sampling_rules.clear()
    xray_provider._groups.clear()
    xray_provider._tags.clear()
    xray_provider._encryption_config.clear()
    yield
    xray_provider._sampling_rules.clear()
    xray_provider._groups.clear()
    xray_provider._tags.clear()
    xray_provider._encryption_config.clear()


def _make_request(method: str, path: str, body: dict | None = None) -> MagicMock:
    req = MagicMock()
    req.method = method
    req.url = MagicMock()
    req.url.path = path
    req.headers = {}
    req.query_params = {}
    payload = json.dumps(body or {}).encode() if body else b""
    req.body = AsyncMock(return_value=payload)
    return req


@pytest.mark.asyncio
class TestSamplingRulesAccountIsolation:
    """Bug: Sampling rules are not isolated by account/region.

    The _sampling_rules dictionary is a global singleton, so rules created
    in one account/region are visible in other accounts/regions.
    """

    async def test_sampling_rules_isolated_by_account(self):
        """Rules created in one account should not be visible in another."""
        # Create a rule in account 111111111111
        create_req = _make_request(
            "POST",
            "/CreateSamplingRule",
            {
                "SamplingRule": {
                    "RuleName": "test-rule",
                    "Priority": 100,
                    "FixedRate": 0.1,
                    "ReservoirSize": 5,
                    "ServiceName": "*",
                    "ServiceType": "*",
                    "Host": "*",
                    "ResourceARN": "*",
                    "HTTPMethod": "*",
                    "URLPath": "*",
                    "Version": 1,
                },
            },
        )
        resp = await handle_xray_request(
            create_req, "us-east-1", "111111111111"
        )
        assert resp.status_code == 200

        # Get rules from account 111111111111 - should find the rule
        get_req = _make_request("POST", "/GetSamplingRules")
        resp1 = await handle_xray_request(get_req, "us-east-1", "111111111111")
        body1 = json.loads(resp1.body)
        rule_names_1 = [
            r["SamplingRule"]["RuleName"] for r in body1["SamplingRuleRecords"]
        ]
        assert "test-rule" in rule_names_1

        # Get rules from account 222222222222 - should NOT find the rule
        # Bug: Currently this returns the rule from account 111111111111
        resp2 = await handle_xray_request(get_req, "us-east-1", "222222222222")
        body2 = json.loads(resp2.body)
        rule_names_2 = [
            r["SamplingRule"]["RuleName"] for r in body2["SamplingRuleRecords"]
        ]
        assert "test-rule" not in rule_names_2, (
            "Sampling rules should be isolated by account, "
            "but rule from account 111111111111 is visible in account 222222222222"
        )


@pytest.mark.asyncio
class TestGroupsAccountIsolation:
    """Bug: Groups are not isolated by account/region.

    The _groups dictionary is a global singleton, so groups created
    in one account/region are visible in other accounts/regions.
    """

    async def test_groups_isolated_by_account(self):
        """Groups created in one account should not be visible in another."""
        # Create a group in account 111111111111
        create_req = _make_request(
            "POST",
            "/CreateGroup",
            {
                "GroupName": "test-group",
                "FilterExpression": 'service("test")',
            },
        )
        resp = await handle_xray_request(
            create_req, "us-east-1", "111111111111"
        )
        assert resp.status_code == 200

        # Get groups from account 111111111111 - should find the group
        get_req = _make_request("POST", "/Groups")
        resp1 = await handle_xray_request(get_req, "us-east-1", "111111111111")
        body1 = json.loads(resp1.body)
        group_names_1 = [g["GroupName"] for g in body1["Groups"]]
        assert "test-group" in group_names_1

        # Get groups from account 222222222222 - should NOT find the group
        # Bug: Currently this returns the group from account 111111111111
        resp2 = await handle_xray_request(get_req, "us-east-1", "222222222222")
        body2 = json.loads(resp2.body)
        group_names_2 = [g["GroupName"] for g in body2["Groups"]]
        assert "test-group" not in group_names_2, (
            "Groups should be isolated by account, "
            "but group from account 111111111111 is visible in account 222222222222"
        )
