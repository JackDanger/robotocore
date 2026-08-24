"""Tests for Resource Groups Tagging API provider bug fixes.

Each test targets a specific bug that has been fixed.
"""

from unittest.mock import MagicMock, patch

from robotocore.services.tagging.provider import (
    _get_native_sns_resources,
    _get_native_sqs_resources,
)


class TestTaggingAccountIsolation:
    """Bug: Native resource lookups didn't filter by account_id.

    The _get_native_sqs_resources and _get_native_sns_resources functions
    called _get_store(region) without passing account_id, which meant they
    always used the default account. This caused resources from one account
    to be returned when querying for a different account.
    """

    @patch("robotocore.services.sqs.provider._get_store")
    def test_get_native_sqs_resources_uses_provided_account(self, mock_get_store):
        """SQS resources should be looked up using the provided account."""
        # Create a mock store with a queue
        mock_store = MagicMock()
        mock_queue = MagicMock()
        mock_queue.arn = "arn:aws:sqs:us-east-1:222222222222:test-queue"
        mock_queue.tags = {"env": "prod"}
        mock_store.list_queues.return_value = [mock_queue]
        mock_get_store.return_value = mock_store

        # Call _get_native_sqs_resources with account 222222222222
        results = _get_native_sqs_resources("us-east-1", "222222222222", [])

        # Verify _get_store is called with both region and account_id
        mock_get_store.assert_called_once_with("us-east-1", "222222222222")

        # Results should be from the requested account
        assert len(results) == 1
        assert results[0]["ResourceARN"] == "arn:aws:sqs:us-east-1:222222222222:test-queue"

    @patch("robotocore.services.sns.provider._get_store")
    def test_get_native_sns_resources_uses_provided_account(self, mock_get_store):
        """SNS resources should be looked up using the provided account."""
        # Create a mock store with a topic
        mock_store = MagicMock()
        mock_topic = MagicMock()
        mock_topic.arn = "arn:aws:sns:us-east-1:222222222222:test-topic"
        mock_topic.tags = {"env": "prod"}
        mock_store.topics.values.return_value = [mock_topic]
        mock_get_store.return_value = mock_store

        # Call _get_native_sns_resources with account 222222222222
        results = _get_native_sns_resources("us-east-1", "222222222222", [])

        # Verify _get_store is called with both region and account_id
        mock_get_store.assert_called_once_with("us-east-1", "222222222222")

        assert len(results) == 1
        assert results[0]["ResourceARN"] == "arn:aws:sns:us-east-1:222222222222:test-topic"
