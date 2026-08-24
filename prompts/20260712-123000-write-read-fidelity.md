---
session: 20260712
slug: write-read-fidelity-integration-tests
type: test
---

## Summary

Integration tests for the "write goes in, wrong (or no) value comes back out" bug
class: EC2 placement group `PartitionCount`, S3 notification config `Id`, ECR
`DescribeRepositories` pagination, Kinesis `AT_TIMESTAMP` filtering, SecretsManager
`RotateSecret` binary preservation, DynamoDB global-table replica stream inheritance.

Each test performs a real end-to-end round trip via boto3 against a running server —
no mocking of provider functions.

## Correction made during review

The agent's original Kinesis test only decoded the shard-iterator token and asserted
it contained a `timestamp` key — that only proves the value is stored, not that
`GetRecords` actually filters by it (the real behavior the fix targets, and the thing
a coupling-to-internal-token-format test can't catch a regression in). Rewrote it to
put an "early" and "late" record with a real time gap, request an `AT_TIMESTAMP`
iterator at the midpoint, call `GetRecords`, and assert only the late record comes
back.

The SecretsManager and DynamoDB tests looked risky on first read (both wrapped their
core assertions in `try/except: pytest.skip(...)`, which can silently mask a real
failure as an environment limitation) but both actually exercise the real path and
pass without skipping — SecretsManager spins up a real rotation Lambda end-to-end,
DynamoDB drives `create_global_table` for real. Left those as-is.

## Verification

- All 6 tests pass (re-verified after the Kinesis rewrite).
- Full integration suite: 79 passed, 4 skipped. Full unit suite: 8777 passed.
- `ruff`/`ruff format`/`mypy` clean.
