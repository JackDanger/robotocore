"""EC2 capacity profile compatibility tests.

Tests deterministic EC2 capacity profiles for:
- InsufficientInstanceCapacity errors
- Spot instance availability
- Unsupported instance type/AZ combinations
- Integration with chaos rules
- Integration with state snapshots
"""

import uuid

import botocore.exceptions
import pytest
import requests

from tests.compatibility.conftest import ENDPOINT_URL, make_client


def _unique(prefix: str) -> str:
    return f"{prefix}-{uuid.uuid4().hex[:8]}"


@pytest.fixture
def ec2():
    return make_client("ec2")


@pytest.fixture(autouse=True)
def reset_capacity_profiles():
    """Reset capacity profiles before each test."""
    requests.post(f"{ENDPOINT_URL}/_robotocore/ec2/capacity/reset", timeout=5)
    yield
    requests.post(f"{ENDPOINT_URL}/_robotocore/ec2/capacity/reset", timeout=5)


class TestCapacityProfileAdminEndpoints:
    """Tests for the capacity profile admin endpoints."""

    def test_set_capacity_profile(self):
        """Test setting a capacity profile via admin endpoint."""
        response = requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            json={
                "instance_type": "g5.xlarge",
                "availability_zone": "us-east-1a",
                "total_capacity": 5,
                "available_capacity": 5,
                "spot_available": True,
                "spot_price": 0.10,
            },
            timeout=5,
        )
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "created"
        assert data["profile"]["instance_type"] == "g5.xlarge"
        assert data["profile"]["total_capacity"] == 5

    def test_list_capacity_profiles(self):
        """Test listing capacity profiles."""
        # Set a profile first
        requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            json={
                "instance_type": "g5.xlarge",
                "availability_zone": "us-east-1a",
                "total_capacity": 5,
            },
            timeout=5,
        )

        response = requests.get(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            timeout=5,
        )
        assert response.status_code == 200
        data = response.json()
        assert "profiles" in data
        assert len(data["profiles"]) >= 1

    def test_delete_capacity_profile(self):
        """Test deleting a capacity profile."""
        # Set a profile first
        requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            json={
                "instance_type": "g5.xlarge",
                "availability_zone": "us-east-1a",
                "total_capacity": 5,
            },
            timeout=5,
        )

        # Delete it
        response = requests.delete(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            params={
                "instance_type": "g5.xlarge",
                "availability_zone": "us-east-1a",
            },
            timeout=5,
        )
        assert response.status_code == 200
        assert response.json()["status"] == "deleted"

    def test_reset_capacity_profiles(self):
        """Test resetting capacity profiles."""
        # Set a profile first
        requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            json={
                "instance_type": "g5.xlarge",
                "availability_zone": "us-east-1a",
                "total_capacity": 5,
            },
            timeout=5,
        )

        # Reset
        response = requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity/reset",
            timeout=5,
        )
        assert response.status_code == 200
        assert response.json()["status"] == "reset"


class TestInsufficientInstanceCapacity:
    """Tests for InsufficientInstanceCapacity error handling."""

    def test_run_instances_fails_when_capacity_exhausted(self, ec2):
        """Test RunInstances returns InsufficientInstanceCapacity when exhausted."""
        # Set capacity to 1
        requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            json={
                "instance_type": "t2.micro",
                "availability_zone": "us-east-1a",
                "total_capacity": 1,
                "available_capacity": 1,
            },
            timeout=5,
        )

        # Create VPC and subnet
        vpc = ec2.create_vpc(CidrBlock="10.0.0.0/16")
        vpc_id = vpc["Vpc"]["VpcId"]
        subnet = ec2.create_subnet(
            VpcId=vpc_id,
            CidrBlock="10.0.1.0/24",
            AvailabilityZone="us-east-1a",
        )
        subnet_id = subnet["Subnet"]["SubnetId"]

        # First instance should succeed
        response = ec2.run_instances(
            ImageId="ami-12345678",
            MinCount=1,
            MaxCount=1,
            InstanceType="t2.micro",
            SubnetId=subnet_id,
        )
        assert len(response["Instances"]) == 1

        # Second instance should fail with InsufficientInstanceCapacity
        with pytest.raises(botocore.exceptions.ClientError) as exc:
            ec2.run_instances(
                ImageId="ami-12345678",
                MinCount=1,
                MaxCount=1,
                InstanceType="t2.micro",
                SubnetId=subnet_id,
            )

        error = exc.value.response["Error"]
        assert error["Code"] == "InsufficientInstanceCapacity"
        assert "t2.micro" in error["Message"]
        assert "us-east-1a" in error["Message"]

        # Cleanup
        ec2.terminate_instances(InstanceIds=[response["Instances"][0]["InstanceId"]])
        ec2.delete_subnet(SubnetId=subnet_id)
        ec2.delete_vpc(VpcId=vpc_id)

    def test_run_instances_succeeds_with_sufficient_capacity(self, ec2):
        """Test that RunInstances succeeds when capacity is available."""
        # Set capacity to 5
        requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            json={
                "instance_type": "t2.micro",
                "availability_zone": "us-east-1a",
                "total_capacity": 5,
                "available_capacity": 5,
            },
            timeout=5,
        )

        # Create VPC and subnet
        vpc = ec2.create_vpc(CidrBlock="10.0.0.0/16")
        vpc_id = vpc["Vpc"]["VpcId"]
        subnet = ec2.create_subnet(
            VpcId=vpc_id,
            CidrBlock="10.0.1.0/24",
            AvailabilityZone="us-east-1a",
        )
        subnet_id = subnet["Subnet"]["SubnetId"]

        # Launch 3 instances should succeed
        response = ec2.run_instances(
            ImageId="ami-12345678",
            MinCount=1,
            MaxCount=3,
            InstanceType="t2.micro",
            SubnetId=subnet_id,
        )
        assert len(response["Instances"]) == 3

        # Cleanup
        instance_ids = [i["InstanceId"] for i in response["Instances"]]
        ec2.terminate_instances(InstanceIds=instance_ids)
        ec2.delete_subnet(SubnetId=subnet_id)
        ec2.delete_vpc(VpcId=vpc_id)


class TestUnsupportedInstanceType:
    """Tests for Unsupported error handling."""

    def test_run_instances_fails_for_disabled_offering(self, ec2):
        """Test that RunInstances returns Unsupported for disabled instance type/AZ."""
        # Set offering as disabled
        requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            json={
                "instance_type": "p4d.24xlarge",
                "availability_zone": "us-east-1a",
                "total_capacity": 0,
                "available_capacity": 0,
                "enabled": False,
            },
            timeout=5,
        )

        # Create VPC and subnet
        vpc = ec2.create_vpc(CidrBlock="10.0.0.0/16")
        vpc_id = vpc["Vpc"]["VpcId"]
        subnet = ec2.create_subnet(
            VpcId=vpc_id,
            CidrBlock="10.0.1.0/24",
            AvailabilityZone="us-east-1a",
        )
        subnet_id = subnet["Subnet"]["SubnetId"]

        # Launch should fail with Unsupported
        with pytest.raises(botocore.exceptions.ClientError) as exc:
            ec2.run_instances(
                ImageId="ami-12345678",
                MinCount=1,
                MaxCount=1,
                InstanceType="p4d.24xlarge",
                SubnetId=subnet_id,
            )

        error = exc.value.response["Error"]
        assert error["Code"] == "Unsupported"
        assert "not supported" in error["Message"].lower()

        # Cleanup
        ec2.delete_subnet(SubnetId=subnet_id)
        ec2.delete_vpc(VpcId=vpc_id)


class TestSpotInstanceCapacity:
    """Tests for spot instance capacity handling."""

    def test_request_spot_instances_fails_when_no_spot_capacity(self, ec2):
        """Test that RequestSpotInstances returns capacity-not-available when spot is disabled."""
        # Set spot as not available
        requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            json={
                "instance_type": "t2.micro",
                "availability_zone": "us-east-1a",
                "total_capacity": 10,
                "available_capacity": 10,
                "spot_available": False,
            },
            timeout=5,
        )

        # Request spot instance
        response = ec2.request_spot_instances(
            SpotPrice="0.05",
            InstanceCount=1,
            LaunchSpecification={
                "ImageId": "ami-12345678",
                "InstanceType": "t2.micro",
                "Placement": {"AvailabilityZone": "us-east-1a"},
            },
        )

        # Should return open state with capacity-not-available status
        requests_list = response["SpotInstanceRequests"]
        assert len(requests_list) == 1
        assert requests_list[0]["State"] == "open"
        assert requests_list[0]["Status"]["Code"] == "capacity-not-available"

    def test_request_spot_instances_succeeds_with_capacity(self, ec2):
        """Test that RequestSpotInstances succeeds when spot capacity is available."""
        # Set spot as available
        requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            json={
                "instance_type": "t2.micro",
                "availability_zone": "us-east-1a",
                "total_capacity": 10,
                "available_capacity": 10,
                "spot_available": True,
                "spot_price": 0.05,
            },
            timeout=5,
        )

        # Request spot instance
        response = ec2.request_spot_instances(
            SpotPrice="0.05",
            InstanceCount=1,
            LaunchSpecification={
                "ImageId": "ami-12345678",
                "InstanceType": "t2.micro",
                "Placement": {"AvailabilityZone": "us-east-1a"},
            },
        )

        # Should return active state with fulfilled status
        requests_list = response["SpotInstanceRequests"]
        assert len(requests_list) == 1
        assert requests_list[0]["State"] == "active"
        assert requests_list[0]["Status"]["Code"] == "fulfilled"
        assert "InstanceId" in requests_list[0]
        assert requests_list[0]["InstanceId"] is not None

        # Cleanup
        instance_id = requests_list[0]["InstanceId"]
        if instance_id:
            ec2.terminate_instances(InstanceIds=[instance_id])


class TestChaosIntegration:
    """Tests for chaos rule integration with capacity profiles."""

    def test_chaos_override_insufficient_capacity(self, ec2):
        """Test that chaos override can force InsufficientInstanceCapacity."""
        # Set capacity to plenty
        requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity",
            json={
                "instance_type": "t2.micro",
                "availability_zone": "us-east-1a",
                "total_capacity": 100,
                "available_capacity": 100,
            },
            timeout=5,
        )

        # Set chaos override
        requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity/chaos",
            json={"error_code": "InsufficientInstanceCapacity"},
            timeout=5,
        )

        # Create VPC and subnet
        vpc = ec2.create_vpc(CidrBlock="10.0.0.0/16")
        vpc_id = vpc["Vpc"]["VpcId"]
        subnet = ec2.create_subnet(
            VpcId=vpc_id,
            CidrBlock="10.0.1.0/24",
            AvailabilityZone="us-east-1a",
        )
        subnet_id = subnet["Subnet"]["SubnetId"]

        try:
            # Launch should fail due to chaos override
            with pytest.raises(botocore.exceptions.ClientError) as exc:
                ec2.run_instances(
                    ImageId="ami-12345678",
                    MinCount=1,
                    MaxCount=1,
                    InstanceType="t2.micro",
                    SubnetId=subnet_id,
                )

            error = exc.value.response["Error"]
            assert error["Code"] == "InsufficientInstanceCapacity"
        finally:
            # Clear chaos override
            requests.post(
                f"{ENDPOINT_URL}/_robotocore/ec2/capacity/chaos",
                json={"clear": True},
                timeout=5,
            )
            # Cleanup
            ec2.delete_subnet(SubnetId=subnet_id)
            ec2.delete_vpc(VpcId=vpc_id)

    def test_chaos_override_unsupported(self, ec2):
        """Test that chaos override can force Unsupported error."""
        # Set chaos override
        requests.post(
            f"{ENDPOINT_URL}/_robotocore/ec2/capacity/chaos",
            json={"error_code": "Unsupported"},
            timeout=5,
        )

        # Create VPC and subnet
        vpc = ec2.create_vpc(CidrBlock="10.0.0.0/16")
        vpc_id = vpc["Vpc"]["VpcId"]
        subnet = ec2.create_subnet(
            VpcId=vpc_id,
            CidrBlock="10.0.1.0/24",
            AvailabilityZone="us-east-1a",
        )
        subnet_id = subnet["Subnet"]["SubnetId"]

        try:
            # Launch should fail due to chaos override
            with pytest.raises(botocore.exceptions.ClientError) as exc:
                ec2.run_instances(
                    ImageId="ami-12345678",
                    MinCount=1,
                    MaxCount=1,
                    InstanceType="t2.micro",
                    SubnetId=subnet_id,
                )

            error = exc.value.response["Error"]
            assert error["Code"] == "Unsupported"
        finally:
            # Clear chaos override
            requests.post(
                f"{ENDPOINT_URL}/_robotocore/ec2/capacity/chaos",
                json={"clear": True},
                timeout=5,
            )
            # Cleanup
            ec2.delete_subnet(SubnetId=subnet_id)
            ec2.delete_vpc(VpcId=vpc_id)


class TestDescribeInstanceTypeOfferings:
    """Tests for DescribeInstanceTypeOfferings with capacity profiles."""

    def test_describe_instance_type_offerings_returns_offerings(self, ec2):
        """Test that DescribeInstanceTypeOfferings returns configured offerings."""
        response = ec2.describe_instance_type_offerings(
            LocationType="availability-zone",
            Filters=[{"Name": "location", "Values": ["us-east-1a"]}],
        )
        assert "InstanceTypeOfferings" in response
        assert len(response["InstanceTypeOfferings"]) > 0

        # Check that offerings have the expected structure
        offering = response["InstanceTypeOfferings"][0]
        assert "InstanceType" in offering
        assert "LocationType" in offering
        assert "Location" in offering
