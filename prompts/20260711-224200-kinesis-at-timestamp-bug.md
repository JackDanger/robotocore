---
session: "20260711-001"
slug: "kinesis-at-timestamp-bug"
type: "bugfix"
---

## Context

While reviewing the native Kinesis provider for correctness bugs, I discovered that the `AT_TIMESTAMP` shard iterator type was not properly implemented. The `Timestamp` parameter was being silently ignored, causing `GetRecords` to return all records from the beginning of the shard instead of filtering by timestamp.

## Root Cause

In `src/robotocore/services/kinesis/provider.py`:

1. The `_encode_iterator` function did not accept or store a `timestamp` parameter
2. The `_get_shard_iterator` function ignored the `Timestamp` parameter when creating an `AT_TIMESTAMP` iterator
3. The `_get_records` function did not filter records by timestamp when the iterator type was `AT_TIMESTAMP`

## Fix

1. Added `timestamp: float | None = None` parameter to `_encode_iterator`
2. Store the timestamp in the iterator JSON payload
3. In `_get_shard_iterator`, pass the `Timestamp` parameter when creating an `AT_TIMESTAMP` iterator
4. In `_get_records`, filter records by timestamp when the iterator type is `AT_TIMESTAMP`

## Verification

Added regression test in `tests/unit/services/test_kinesis_bugs.py`:
- Creates a stream with two records at different timestamps
- Creates an `AT_TIMESTAMP` iterator for 30 minutes ago
- Verifies that only records at or after that timestamp are returned

Test passes after the fix, confirming the bug is resolved.

## Files Changed

- `src/robotocore/services/kinesis/provider.py` - Fixed AT_TIMESTAMP handling
- `tests/unit/services/test_kinesis_bugs.py` - Added regression test
