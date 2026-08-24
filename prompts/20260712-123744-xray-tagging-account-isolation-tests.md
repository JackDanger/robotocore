---
session: 20260712
slug: xray-tagging-account-isolation-tests
type: test
---

## Summary

Added two new test classes to `tests/integration/test_account_scoping_fidelity.py` to verify cross-account isolation for X-Ray and ResourceGroupsTaggingAPI services.

## Changes Made

### 1. Updated module docstring
Added two new entries to the AWS scoping reference list:
- X-Ray sampling rules — per-account + per-region
- X-Ray groups — per-account + per-region
- Resource Groups Tagging API — per-account + per-region

### 2. Added `TestXRayAccountIsolation` class
Two test methods:
- `test_sampling_rules_isolated_by_account`: Creates same-named sampling rules in both accounts and verifies:
  - Each account only sees its own rules
  - ARNs are account-specific
  - No cross-account leakage via GetSamplingRules
  
- `test_groups_isolated_by_account`: Creates same-named groups in both accounts and verifies:
  - Each account only sees its own groups
  - ARNs are account-specific
  - GetGroup returns only the account's own group

### 3. Added `TestTaggingApiAccountIsolation` class
Two test methods:
- `test_sqs_resources_isolated_by_account`: Creates tagged SQS queues in both accounts and verifies:
  - Each account's tagging API only sees its own queues
  - Tag values are correct per account
  - No cross-account leakage
  
- `test_sns_resources_isolated_by_account`: Creates tagged SNS topics in both accounts and verifies:
  - Each account's tagging API only sees its own topics
  - Tag values are correct per account
  - No cross-account leakage

## Verification

All 11 tests in the file pass:
```
tests/integration/test_account_scoping_fidelity.py::TestApiGatewayV2AccountIsolation::test_apis_isolated_by_account PASSED
tests/integration/test_account_scoping_fidelity.py::TestCloudWatchCompositeAlarmIsolation::test_composite_alarms_isolated_by_account PASSED
tests/integration/test_account_scoping_fidelity.py::TestCloudWatchDashboardIsolation::test_dashboards_isolated_by_account PASSED
tests/integration/test_account_scoping_fidelity.py::TestCloudWatchMetricStreamIsolation::test_metric_streams_isolated_by_account PASSED
tests/integration/test_account_scoping_fidelity.py::TestEventBridgeConnectionIsolation::test_connections_isolated_by_account PASSED
tests/integration/test_account_scoping_fidelity.py::TestEventBridgeApiDestinationIsolation::test_api_destinations_isolated_by_account PASSED
tests/integration/test_account_scoping_fidelity.py::TestEventBridgeEndpointIsolation::test_endpoints_isolated_by_account PASSED
tests/integration/test_account_scoping_fidelity.py::TestXRayAccountIsolation::test_sampling_rules_isolated_by_account PASSED
tests/integration/test_account_scoping_fidelity.py::TestXRayAccountIsolation::test_groups_isolated_by_account PASSED
tests/integration/test_account_scoping_fidelity.py::TestTaggingApiAccountIsolation::test_sqs_resources_isolated_by_account PASSED
tests/integration/test_account_scoping_fidelity.py::TestTaggingApiAccountIsolation::test_sns_resources_isolated_by_account PASSED
```

All linting checks pass:
- ruff check: passed
- ruff format: passed
- mypy: passed

Full integration test suite: 77 passed, 4 skipped

## Bug Class Hunt Findings

During the grep sweep for similar patterns, found the following potential issues (NOT fixed - test-only task):

### Confirmed Bugs (no account scoping):

1. **X-Ray `_resource_policies`** (`src/robotocore/services/xray/provider.py:328`)
   - Global dict keyed only by `policy_name`
   - `_list_resource_policies` returns all policies from all accounts
   - Cross-account leak: Any account can see/delete another account's resource policies

### Low Risk / Design Decisions:

2. **X-Ray `_encryption_config`** (`src/robotocore/services/xray/provider.py:27`)
   - Region-keyed only: `dict[str, dict[str, Any]]`  # region -> config
   - All accounts share the same encryption config per region
   - May be intentional (X-Ray encryption is regional)

3. **Lambda `_esm_store`** (`src/robotocore/services/lambda_/provider.py:40`)
   - UUID-keyed: `dict[str, dict]`  # uuid -> mapping config
   - Stores `_account_id` inside config but list operations don't filter
   - Low risk: UUIDs are hard to guess

4. **Rekognition `_video_jobs`** (`src/robotocore/services/rekognition/provider.py:25`)
   - UUID-keyed: `dict[str, dict]`  # job_id -> job
   - No account filtering on lookups
   - Low risk: UUIDs are hard to guess

5. **Rekognition `_liveness_sessions`** (`src/robotocore/services/rekognition/provider.py:31`)
   - UUID-keyed: `dict[str, dict]`  # session_id -> session
   - No account filtering on lookups
   - Low risk: UUIDs are hard to guess

### Not Bugs (properly scoped):
- API Gateway v2: Uses `_acct_region(account_id, region)` as key
- CloudWatch: Uses `(account_id, region)` tuple as key
- Step Functions: Uses full ARN as key (includes account)
- S3: Bucket names are globally unique by design
- Batch: Uses `f"{account_id}:{region}"` as key
- SSM: Uses `f"{account_id}:{region}"` as key
- EC2 placement groups: Uses `account_id` then `region` as nested keys
- Pipes: Uses `f"{account_id}:{region}:{name}"` as key
- DynamoDBStreams: Stores account_id in iterator state
- Support: Case IDs are unique; Moto backend filters by account
