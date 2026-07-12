"""Tests for correctness bugs in the EC2 native provider."""

from robotocore.services.ec2.provider import (
    _create_placement_group,
    _describe_placement_groups,
)

REGION = "us-east-1"
ACCOUNT = "123456789012"


def _params(**kwargs) -> dict:
    return {k: [v] for k, v in kwargs.items()}


class TestPlacementGroupPartitionCountInResponse:
    """CreatePlacementGroup and DescribePlacementGroups stored partitionCount
    but never rendered it into the XML response.
    """

    def test_create_placement_group_includes_partition_count(self):
        resp = _create_placement_group(
            _params(GroupName="pg-1", Strategy="partition", PartitionCount="3"),
            REGION,
            ACCOUNT,
        )
        body = resp.body.decode()
        assert "<partitionCount>3</partitionCount>" in body

    def test_describe_placement_groups_includes_partition_count(self):
        _create_placement_group(
            _params(GroupName="pg-2", Strategy="partition", PartitionCount="5"),
            REGION,
            ACCOUNT,
        )
        resp = _describe_placement_groups(_params(**{"GroupName.1": "pg-2"}), REGION, ACCOUNT)
        body = resp.body.decode()
        assert "<partitionCount>5</partitionCount>" in body

    def test_partition_count_defaults_to_7_for_partition_strategy(self):
        resp = _create_placement_group(
            _params(GroupName="pg-3", Strategy="partition"), REGION, ACCOUNT
        )
        body = resp.body.decode()
        assert "<partitionCount>7</partitionCount>" in body

    def test_partition_count_absent_for_non_partition_strategy(self):
        resp = _create_placement_group(
            _params(GroupName="pg-4", Strategy="cluster"), REGION, ACCOUNT
        )
        body = resp.body.decode()
        assert "<partitionCount>" not in body
