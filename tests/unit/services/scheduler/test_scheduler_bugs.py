"""Tests for correctness bugs in the EventBridge Scheduler provider."""

import json
from unittest.mock import MagicMock

import pytest

from robotocore.services.scheduler.provider import (
    _create_schedule,
    _get_schedules,
    _update_schedule,
)

REGION = "us-east-1"
ACCOUNT_ID = "123456789012"


def _make_request(method: str, path: str, body: dict | None = None, query: str = ""):
    scope = {
        "type": "http",
        "method": method.upper(),
        "path": path,
        "query_string": query.encode(),
        "headers": [],
    }
    body_bytes = json.dumps(body).encode() if body else b""

    async def receive():
        return {"type": "http.request", "body": body_bytes}

    return MagicMock(scope=scope, receive=receive)


@pytest.fixture(autouse=True)
def _clear_state():
    """Clear global state before each test."""
    from robotocore.services.scheduler import provider as sched_module

    sched_module._schedules.clear()
    sched_module._groups.clear()
    yield
    sched_module._schedules.clear()
    sched_module._groups.clear()


# ===================================================================
# Bug 1: UpdateSchedule ignores ScheduleExpressionTimezone, StartDate,
#        EndDate, and KmsKeyArn
# ===================================================================


class TestUpdateScheduleMissingFields:
    """UpdateSchedule should update all mutable fields, not just a subset."""

    def test_create_schedule_stores_all_fields(self):
        """Verify that _create_schedule stores all the fields we care about."""
        params = {
            "ScheduleExpression": "rate(1 hour)",
            "ScheduleExpressionTimezone": "America/New_York",
            "Target": {"Arn": "arn:aws:lambda:us-east-1:123:function:fn"},
            "FlexibleTimeWindow": {"Mode": "OFF"},
            "State": "ENABLED",
            "Description": "Test schedule",
            "StartDate": "2024-01-01T00:00:00Z",
            "EndDate": "2024-12-31T23:59:59Z",
            "KmsKeyArn": "arn:aws:kms:us-east-1:123:key/abc",
        }
        result = _create_schedule("test-sched", params, REGION, ACCOUNT_ID)
        assert "ScheduleArn" in result

        # Verify all fields were stored
        schedules = _get_schedules(REGION, ACCOUNT_ID)
        schedule = schedules["test-sched"]
        assert schedule["ScheduleExpressionTimezone"] == "America/New_York"
        assert schedule["StartDate"] == "2024-01-01T00:00:00Z"
        assert schedule["EndDate"] == "2024-12-31T23:59:59Z"
        assert schedule["KmsKeyArn"] == "arn:aws:kms:us-east-1:123:key/abc"

    def test_update_schedule_timezone_is_ignored_bug(self):
        """BUG: UpdateSchedule ignores ScheduleExpressionTimezone parameter.

        When updating a schedule, the ScheduleExpressionTimezone field
        should be updated if provided, but it's silently ignored.
        """
        # Create initial schedule
        create_params = {
            "ScheduleExpression": "rate(1 hour)",
            "ScheduleExpressionTimezone": "UTC",
            "Target": {"Arn": "arn:aws:lambda:us-east-1:123:function:fn"},
        }
        _create_schedule("test-sched", create_params, REGION, ACCOUNT_ID)

        # Update the timezone
        update_params = {"ScheduleExpressionTimezone": "America/Los_Angeles"}
        _update_schedule("test-sched", update_params, REGION, ACCOUNT_ID)

        # Verify the timezone was updated
        schedules = _get_schedules(REGION, ACCOUNT_ID)
        schedule = schedules["test-sched"]
        assert schedule["ScheduleExpressionTimezone"] == "America/Los_Angeles", (
            f"Expected timezone to be updated to 'America/Los_Angeles', "
            f"but got '{schedule['ScheduleExpressionTimezone']}'"
        )

    def test_update_schedule_start_date_is_ignored_bug(self):
        """BUG: UpdateSchedule ignores StartDate parameter."""
        # Create initial schedule
        create_params = {
            "ScheduleExpression": "rate(1 hour)",
            "Target": {"Arn": "arn:aws:lambda:us-east-1:123:function:fn"},
            "StartDate": "2024-01-01T00:00:00Z",
        }
        _create_schedule("test-sched", create_params, REGION, ACCOUNT_ID)

        # Update the start date
        update_params = {"StartDate": "2024-06-01T00:00:00Z"}
        _update_schedule("test-sched", update_params, REGION, ACCOUNT_ID)

        # Verify the start date was updated
        schedules = _get_schedules(REGION, ACCOUNT_ID)
        schedule = schedules["test-sched"]
        assert schedule["StartDate"] == "2024-06-01T00:00:00Z", (
            f"Expected StartDate to be updated to '2024-06-01T00:00:00Z', "
            f"but got '{schedule['StartDate']}'"
        )

    def test_update_schedule_end_date_is_ignored_bug(self):
        """BUG: UpdateSchedule ignores EndDate parameter."""
        # Create initial schedule
        create_params = {
            "ScheduleExpression": "rate(1 hour)",
            "Target": {"Arn": "arn:aws:lambda:us-east-1:123:function:fn"},
            "EndDate": "2024-12-31T23:59:59Z",
        }
        _create_schedule("test-sched", create_params, REGION, ACCOUNT_ID)

        # Update the end date
        update_params = {"EndDate": "2024-06-30T23:59:59Z"}
        _update_schedule("test-sched", update_params, REGION, ACCOUNT_ID)

        # Verify the end date was updated
        schedules = _get_schedules(REGION, ACCOUNT_ID)
        schedule = schedules["test-sched"]
        assert schedule["EndDate"] == "2024-06-30T23:59:59Z", (
            f"Expected EndDate to be updated to '2024-06-30T23:59:59Z', "
            f"but got '{schedule['EndDate']}'"
        )

    def test_update_schedule_kms_key_arn_is_ignored_bug(self):
        """BUG: UpdateSchedule ignores KmsKeyArn parameter."""
        # Create initial schedule
        create_params = {
            "ScheduleExpression": "rate(1 hour)",
            "Target": {"Arn": "arn:aws:lambda:us-east-1:123:function:fn"},
            "KmsKeyArn": "arn:aws:kms:us-east-1:123:key/old",
        }
        _create_schedule("test-sched", create_params, REGION, ACCOUNT_ID)

        # Update the KMS key
        update_params = {"KmsKeyArn": "arn:aws:kms:us-east-1:123:key/new"}
        _update_schedule("test-sched", update_params, REGION, ACCOUNT_ID)

        # Verify the KMS key was updated
        schedules = _get_schedules(REGION, ACCOUNT_ID)
        schedule = schedules["test-sched"]
        assert schedule["KmsKeyArn"] == "arn:aws:kms:us-east-1:123:key/new", (
            f"Expected KmsKeyArn to be updated to 'arn:aws:kms:us-east-1:123:key/new', "
            f"but got '{schedule['KmsKeyArn']}'"
        )
