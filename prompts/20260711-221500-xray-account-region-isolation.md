---
session: 20260711
slug: xray-account-region-isolation
type: fix
---

## Context

Reviewing the X-Ray provider for correctness bugs related to multi-account isolation.

## Root cause

The `_sampling_rules` and `_groups` dictionaries were global singletons, not scoped by account or region. This meant:

1. Sampling rules created in account A were visible in account B
2. Groups created in account A were visible in account B
3. There was no isolation between different accounts or regions

The data structures were:
```python
_sampling_rules: dict[str, dict[str, Any]] = {}  # rule_name -> record
_groups: dict[str, dict[str, Any]] = {}  # group_name -> group
```

But they should have been:
```python
_sampling_rules: dict[tuple[str, str], dict[str, Any]] = {}  # (account_id, region) -> {rule_name: record}
_groups: dict[tuple[str, str], dict[str, Any]] = {}  # (account_id, region) -> {group_name: group}
```

## Fix

Updated the data structures and all related functions to scope sampling rules and groups by (account_id, region):

- `_create_sampling_rule`: Now stores rules under `(account_id, region)` key
- `_get_sampling_rules`: Now returns only rules for the specific account/region
- `_delete_sampling_rule`: Now looks up rules by account/region
- `_update_sampling_rule`: Now looks up rules by account/region
- `_create_group`: Now stores groups under `(account_id, region)` key
- `_get_group`: Now looks up groups by account/region
- `_get_groups`: Now returns only groups for the specific account/region
- `_delete_group`: Now looks up groups by account/region
- `_update_group`: Now looks up groups by account/region

## Verification

- New unit test `test_sampling_rules_isolated_by_account` verifies rules are scoped by account
- New unit test `test_groups_isolated_by_account` verifies groups are scoped by account
- All existing X-Ray provider tests pass (15 tests)
- Full unit suite passes
