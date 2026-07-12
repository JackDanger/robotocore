"""Tests for correctness bugs in the DynamoDB native provider.

Each test targets a specific bug that has been fixed.
"""

from unittest.mock import MagicMock, patch

from robotocore.services.dynamodb.replication import create_replica_table

REGION = "us-east-1"
ACCOUNT = "123456789012"


# ===================================================================
# Bug 1: Global table replica doesn't inherit stream configuration
# ===================================================================


class TestGlobalTableReplicaStreamConfig:
    """When creating a global table replica, the replica table should inherit
    the stream configuration from the source table.

    Previously, create_replica_table passed streams=None, which meant
    replica tables never had streams enabled even when the source
    table had streaming enabled.
    """

    def test_replica_inherits_stream_config(self):
        """Replica tables should inherit stream configuration from source."""
        # Mock the source table with streams enabled
        mock_source_table = MagicMock()
        mock_source_table.hash_key_attr = "id"
        mock_source_table.hash_key_type = "S"
        mock_source_table.range_key_attr = None
        mock_source_table.range_key_type = None
        mock_source_table.latest_stream_label = "2024-01-01T00:00:00"
        mock_source_table.stream_specification = {
            "StreamEnabled": True,
            "StreamViewType": "NEW_AND_OLD_IMAGES",
        }

        # Track what parameters are passed to create_table
        create_table_calls = []

        def mock_create_table(*args, **kwargs):
            create_table_calls.append((args, kwargs))
            return MagicMock()

        # Mock the backends
        with patch("moto.backends.get_backend") as mock_get_backend:
            mock_source_backend = MagicMock()
            mock_source_backend.get_table.return_value = mock_source_table

            mock_target_backend = MagicMock()
            mock_target_backend.get_table.return_value = None
            mock_target_backend.create_table = mock_create_table

            # Set up the backend dictionary
            backend_dict = {
                ACCOUNT: {
                    "us-east-1": mock_source_backend,
                    "us-west-2": mock_target_backend,
                }
            }
            mock_get_backend.return_value = backend_dict

            # Call create_replica_table
            result = create_replica_table(
                table_name="test-table",
                source_region="us-east-1",
                target_region="us-west-2",
                account_id=ACCOUNT,
            )

            assert result is True
            assert len(create_table_calls) == 1
            args, kwargs = create_table_calls[0]
            # The streams parameter should be inherited from source
            assert kwargs.get("streams") == {
                "StreamEnabled": True,
                "StreamViewType": "NEW_AND_OLD_IMAGES",
            }

    def test_replica_with_no_source_stream(self):
        """Replica tables should have streams=None when source has no streams."""
        # Mock the source table WITHOUT streams
        mock_source_table = MagicMock()
        mock_source_table.hash_key_attr = "id"
        mock_source_table.hash_key_type = "S"
        mock_source_table.range_key_attr = None
        mock_source_table.range_key_type = None
        mock_source_table.latest_stream_label = None  # No stream
        mock_source_table.stream_specification = {}

        create_table_calls = []

        def mock_create_table(*args, **kwargs):
            create_table_calls.append((args, kwargs))
            return MagicMock()

        with patch("moto.backends.get_backend") as mock_get_backend:
            mock_source_backend = MagicMock()
            mock_source_backend.get_table.return_value = mock_source_table

            mock_target_backend = MagicMock()
            mock_target_backend.get_table.return_value = None
            mock_target_backend.create_table = mock_create_table

            backend_dict = {
                ACCOUNT: {
                    "us-east-1": mock_source_backend,
                    "us-west-2": mock_target_backend,
                }
            }
            mock_get_backend.return_value = backend_dict

            result = create_replica_table(
                table_name="test-table",
                source_region="us-east-1",
                target_region="us-west-2",
                account_id=ACCOUNT,
            )

            assert result is True
            assert len(create_table_calls) == 1
            args, kwargs = create_table_calls[0]
            # The streams parameter should be None when source has no stream
            assert kwargs.get("streams") is None
