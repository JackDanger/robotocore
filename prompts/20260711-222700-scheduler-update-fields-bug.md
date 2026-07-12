---
session: "scheduler-bug-fix"
timestamp: "2026-07-11T22:27:00Z"
slug: "scheduler-update-fields-bug"
type: "bug-fix"
---

## Context

Found a correctness bug in the EventBridge Scheduler native provider where the `UpdateSchedule` operation silently ignores several fields that are accepted during `CreateSchedule`:

- `ScheduleExpressionTimezone`
- `StartDate`
- `EndDate`
- `KmsKeyArn`

## Root cause

The `_update_schedule` function in `src/robotocore/services/scheduler/provider.py` only updates a subset of the fields that `_create_schedule` stores:

```python
def _update_schedule(name: str, params: dict, region: str, account_id: str) -> dict:
    # ...
    if "ScheduleExpression" in params:
        schedule["ScheduleExpression"] = params["ScheduleExpression"]
    if "Target" in params:
        schedule["Target"] = params["Target"]
    if "FlexibleTimeWindow" in params:
        schedule["FlexibleTimeWindow"] = params["FlexibleTimeWindow"]
    if "State" in params:
        schedule["State"] = params["State"]
    if "Description" in params:
        schedule["Description"] = params["Description"]
    # Missing: ScheduleExpressionTimezone, StartDate, EndDate, KmsKeyArn
    schedule["LastModificationDate"] = time.time()
```

## Fix

Added the missing field updates to `_update_schedule`:

```python
if "ScheduleExpressionTimezone" in params:
    schedule["ScheduleExpressionTimezone"] = params["ScheduleExpressionTimezone"]
if "StartDate" in params:
    schedule["StartDate"] = params["StartDate"]
if "EndDate" in params:
    schedule["EndDate"] = params["EndDate"]
if "KmsKeyArn" in params:
    schedule["KmsKeyArn"] = params["KmsKeyArn"]
```

## Verification

Added regression tests in `tests/unit/services/scheduler/test_scheduler_bugs.py` that verify:
1. All fields are stored during CreateSchedule
2. Each missing field can be updated via UpdateSchedule

All scheduler tests pass (37 tests).
