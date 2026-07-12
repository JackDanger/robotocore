"""Tests for correctness bugs found and fixed in API Gateway v2 provider.

Each test documents a specific bug that has been fixed.
"""

from robotocore.services.apigatewayv2.provider import (
    _create_api,
    _create_stage,
    _delete_api,
    _update_stage,
)

REGION = "us-east-1"
ACCOUNT = "123456789012"


# ===================================================================
# Bug 1: _update_stage ignores AccessLogSettings and Tags fields
# ===================================================================


class TestUpdateStageMissingFields:
    """Test that AccessLogSettings and Tags are properly updated in _update_stage."""

    def test_update_stage_access_log_settings(self):
        """AccessLogSettings field should be updated when provided in params."""
        # Create an API first
        api_result = _create_api(
            {"Name": "test-api", "ProtocolType": "HTTP"}, REGION, ACCOUNT
        )
        api_id = api_result["ApiId"]

        # Create a stage with AccessLogSettings
        create_params = {
            "StageName": "prod",
            "AccessLogSettings": {
                "DestinationArn": "arn:aws:logs:us-east-1:123456789012:log-group:old-group",
                "Format": "{\"requestId\": \"$context.requestId\"}",
            },
        }
        result = _create_stage(api_id, create_params, REGION, ACCOUNT)
        assert result["AccessLogSettings"]["DestinationArn"].endswith("old-group")

        # Update the AccessLogSettings
        update_params = {
            "AccessLogSettings": {
                "DestinationArn": "arn:aws:logs:us-east-1:123456789012:log-group:new-group",
                "Format": "{\"updated\": true}",
            }
        }
        result = _update_stage(api_id, "prod", update_params, REGION, ACCOUNT)

        # Bug: AccessLogSettings was silently ignored in _update_stage
        # After fix, the AccessLogSettings should be updated
        assert (
            result["AccessLogSettings"]["DestinationArn"].endswith("new-group")
        ), f"Expected updated DestinationArn, got: {result.get('AccessLogSettings')}"
        assert (
            result["AccessLogSettings"]["Format"] == '{"updated": true}'
        ), f"Expected updated Format, got: {result.get('AccessLogSettings')}"

    def test_update_stage_tags(self):
        """Tags field should be updated when provided in params."""
        # Create an API first
        api_result = _create_api(
            {"Name": "test-api", "ProtocolType": "HTTP"}, REGION, ACCOUNT
        )
        api_id = api_result["ApiId"]

        # Create a stage with Tags
        create_params = {"StageName": "prod", "Tags": {"env": "old-env", "team": "backend"}}
        result = _create_stage(api_id, create_params, REGION, ACCOUNT)
        assert result["Tags"]["env"] == "old-env"

        # Update the Tags
        update_params = {"Tags": {"env": "new-env", "team": "backend"}}
        result = _update_stage(api_id, "prod", update_params, REGION, ACCOUNT)

        # Bug: Tags was silently ignored in _update_stage
        # After fix, the Tags should be updated
        assert (
            result["Tags"]["env"] == "new-env"
        ), f"Expected updated tag, got: {result.get('Tags')}"
        # Other tags should be preserved
        assert result["Tags"]["team"] == "backend"

    def test_update_stage_preserves_other_fields(self):
        """Updating AccessLogSettings should not affect other fields."""
        # Create an API first
        api_result = _create_api(
            {"Name": "test-api", "ProtocolType": "HTTP"}, REGION, ACCOUNT
        )
        api_id = api_result["ApiId"]

        # Create a stage with multiple fields
        create_params = {
            "StageName": "prod",
            "Description": "Original description",
            "StageVariables": {"key": "value"},
            "AccessLogSettings": {
                "DestinationArn": "arn:aws:logs:us-east-1:123456789012:log-group:old-group",
                "Format": "{\"requestId\": \"$context.requestId\"}",
            },
        }
        _create_stage(api_id, create_params, REGION, ACCOUNT)

        # Update only AccessLogSettings
        update_params = {
            "AccessLogSettings": {
                "DestinationArn": "arn:aws:logs:us-east-1:123456789012:log-group:new-group",
                "Format": "{\"updated\": true}",
            }
        }
        result = _update_stage(api_id, "prod", update_params, REGION, ACCOUNT)

        # Other fields should be preserved
        assert result["Description"] == "Original description"
        assert result["StageVariables"]["key"] == "value"
        # AccessLogSettings should be updated
        assert result["AccessLogSettings"]["DestinationArn"].endswith("new-group")

        # Cleanup
        _delete_api(api_id, REGION, ACCOUNT)
