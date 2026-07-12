"""Tests for Config provider bug fixes.

Each test targets a specific bug that has been fixed.
"""

import copy
from unittest.mock import MagicMock, patch

import pytest

from robotocore.services.config.provider import (
    _delete_config_rule,
    _evaluations,
    _evaluation_statuses,
    _put_config_rule,
    _put_evaluations,
)


class TestDeleteConfigRuleCleanup:
    """Bug: DeleteConfigRule had incorrect cleanup code for evaluations.

    The _delete_config_rule function tried to clean up _evaluations using
    rule_name as the key, but evaluations are stored with a key of
    "{resource_type}:{resource_id}", not rule_name. This caused the cleanup
    to silently do nothing.

    Fix: Removed the incorrect cleanup code. Evaluations are stored per-resource
    and cannot be cleaned up by rule name without additional tracking.
    """

    @patch("robotocore.services.config.provider._get_config_backend")
    def test_delete_config_rule_cleans_up_evaluation_status(self, mock_backend_fn):
        """Deleting a rule should remove its evaluation status from the global store."""
        mock_backend = MagicMock()
        mock_backend.config_rules = {"test-rule": MagicMock()}
        mock_backend.put_config_rule.return_value = "arn:aws:config:us-east-1:123:config-rule/test-rule"
        mock_backend.delete_config_rule = MagicMock()
        mock_backend_fn.return_value = mock_backend

        # Clear any existing state
        _evaluation_statuses.clear()

        # Create a rule (this populates _evaluation_statuses)
        _put_config_rule(
            {
                "ConfigRule": {
                    "ConfigRuleName": "test-rule",
                    "Source": {
                        "Owner": "AWS",
                        "SourceIdentifier": "S3_BUCKET_VERSIONING_ENABLED",
                    },
                }
            },
            "us-east-1",
            "123456789012",
        )

        # Verify evaluation status was stored
        key = ("123456789012", "us-east-1")
        assert key in _evaluation_statuses
        assert "test-rule" in _evaluation_statuses[key]

        # Delete the rule
        _delete_config_rule(
            {"ConfigRuleName": "test-rule"}, "us-east-1", "123456789012"
        )

        # Evaluation status should be cleaned up
        assert "test-rule" not in _evaluation_statuses.get(key, {}), (
            "Evaluation status should be cleaned up when rule is deleted"
        )

    @patch("robotocore.services.config.provider._get_config_backend")
    def test_delete_config_rule_does_not_crash_on_evaluations(self, mock_backend_fn):
        """Deleting a rule should not crash even if evaluations exist."""
        mock_backend = MagicMock()
        mock_backend.config_rules = {"test-rule": MagicMock()}
        mock_backend.delete_config_rule = MagicMock()
        mock_backend_fn.return_value = mock_backend

        # Clear any existing state
        _evaluations.clear()
        _evaluation_statuses.clear()

        # Create an evaluation (stored by resource, not rule)
        _put_evaluations(
            {
                "Evaluations": [
                    {
                        "ComplianceResourceType": "AWS::S3::Bucket",
                        "ComplianceResourceId": "my-bucket",
                        "ComplianceType": "COMPLIANT",
                    }
                ],
                "ResultToken": "test-token",
            },
            "us-east-1",
            "123456789012",
        )

        # Verify evaluation was stored
        key = ("123456789012", "us-east-1")
        assert key in _evaluations

        # Delete the rule - should not crash
        # Bug was: this would try to pop using rule_name as key, which doesn't exist
        _delete_config_rule(
            {"ConfigRuleName": "test-rule"}, "us-east-1", "123456789012"
        )

        # Evaluations remain (they're per-resource, not per-rule)
        # The important thing is we didn't crash
        assert key in _evaluations
