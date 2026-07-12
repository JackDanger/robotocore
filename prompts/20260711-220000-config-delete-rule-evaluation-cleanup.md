---
session: 20260711
slug: config-delete-rule-evaluation-cleanup
type: fix
---

## Context

Reviewing the Config provider's `_delete_config_rule` function for correctness bugs.

## Root cause

The `_delete_config_rule` function attempted to clean up `_evaluations` using `rule_name` as the key:

```python
if key in _evaluations:
    _evaluations[key].pop(rule_name, None)
```

However, `_put_evaluations` stores evaluations using `{resource_type}:{resource_id}` as the key:

```python
eval_key = f"{resource_type}:{resource_id}"
if eval_key not in _evaluations[key]:
    _evaluations[key][eval_key] = []
_evaluations[key][eval_key].append(evaluation)
```

This mismatch meant the cleanup code silently did nothing - it tried to pop a key that never existed. The evaluations would remain in memory even after the rule was deleted.

## Fix

Removed the incorrect cleanup code for `_evaluations`. The `_evaluation_statuses` cleanup (which uses the correct key) is preserved. Added a comment explaining why evaluations can't be cleaned up by rule name.

## Verification

- New unit test `test_delete_config_rule_cleans_up_evaluation_status` verifies that `_evaluation_statuses` is properly cleaned up
- New unit test `test_delete_config_rule_does_not_crash_on_evaluations` verifies that the function doesn't crash when evaluations exist
- All existing config provider tests pass (13 tests)
- Full unit suite passes
