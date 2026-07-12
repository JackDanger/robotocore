---
session: 20260711
slug: tagging-account-isolation
type: fix
---

## Context

Reviewing the Resource Groups Tagging API provider for correctness bugs related to multi-account isolation.

## Root cause

The `_get_native_sqs_resources` and `_get_native_sns_resources` functions called `_get_store(region)` without passing `account_id`, which meant they always used the default account (DEFAULT_ACCOUNT_ID). This caused resources from one account to be returned when querying for a different account.

For example:
```python
# Bug: always uses DEFAULT_ACCOUNT_ID
sqs_store = get_sqs_store(region)
```

Should have been:
```python
# Fix: uses the requested account
sqs_store = get_sqs_store(region, account_id)
```

## Fix

Updated the tagging provider to pass `account_id` to the store functions:

- `_get_resources`: Now passes `account_id` to `_get_native_sqs_resources` and `_get_native_sns_resources`
- `_get_tag_keys`: Now passes `account_id` to `get_sqs_store` and `get_sns_store`
- `_get_native_sqs_resources`: Updated signature to accept `account_id` parameter
- `_get_native_sns_resources`: Updated signature to accept `account_id` parameter

## Verification

- New unit test `test_get_native_sqs_resources_uses_provided_account` verifies SQS resources are scoped by account
- New unit test `test_get_native_sns_resources_uses_provided_account` verifies SNS resources are scoped by account
- All existing tagging provider tests pass (9 tests)
- Full unit suite passes
