"""Tests for correctness bugs found and fixed in AppSync provider.

Each test documents a specific bug that has been fixed.
"""

from robotocore.services.appsync.provider import (
    AppSyncStore,
    _create_data_source,
    _create_graphql_api,
    _update_data_source,
)

REGION = "us-east-1"
ACCOUNT = "123456789012"


# ===================================================================
# Bug 1: _update_data_source ignores httpConfig field
# ===================================================================


class TestUpdateDataSourceHttpConfig:
    """Test that httpConfig is properly updated in _update_data_source."""

    def test_update_data_source_http_config(self):
        """httpConfig field should be updated when provided in params."""
        store = AppSyncStore()

        # Create an API first
        api_result = _create_graphql_api(
            store, {"name": "test-api", "authenticationType": "API_KEY"}, REGION, ACCOUNT
        )
        api_id = api_result["graphqlApi"]["apiId"]

        # Create a data source with httpConfig
        create_params = {
            "name": "http-ds",
            "type": "HTTP",
            "httpConfig": {"endpoint": "https://example.com/api"},
        }
        result = _create_data_source(store, api_id, create_params, REGION, ACCOUNT)
        assert result["dataSource"]["httpConfig"]["endpoint"] == "https://example.com/api"

        # Update the httpConfig
        update_params = {"httpConfig": {"endpoint": "https://newdomain.com/api"}}
        result = _update_data_source(store, api_id, "http-ds", update_params, REGION, ACCOUNT)

        # Bug: httpConfig was silently ignored in _update_data_source
        # After fix, the httpConfig should be updated
        assert (
            result["dataSource"]["httpConfig"]["endpoint"] == "https://newdomain.com/api"
        ), f"Expected updated endpoint, got: {result['dataSource'].get('httpConfig')}"

    def test_update_data_source_preserves_other_fields(self):
        """Updating httpConfig should not affect other fields."""
        store = AppSyncStore()

        # Create an API first
        api_result = _create_graphql_api(
            store, {"name": "test-api", "authenticationType": "API_KEY"}, REGION, ACCOUNT
        )
        api_id = api_result["graphqlApi"]["apiId"]

        # Create a data source with multiple config fields
        create_params = {
            "name": "http-ds",
            "type": "HTTP",
            "description": "Original description",
            "httpConfig": {"endpoint": "https://example.com/api"},
        }
        _create_data_source(store, api_id, create_params, REGION, ACCOUNT)

        # Update only httpConfig
        update_params = {"httpConfig": {"endpoint": "https://newdomain.com/api"}}
        result = _update_data_source(store, api_id, "http-ds", update_params, REGION, ACCOUNT)

        # Other fields should be preserved
        assert result["dataSource"]["description"] == "Original description"
        assert result["dataSource"]["type"] == "HTTP"
        # httpConfig should be updated
        assert result["dataSource"]["httpConfig"]["endpoint"] == "https://newdomain.com/api"
