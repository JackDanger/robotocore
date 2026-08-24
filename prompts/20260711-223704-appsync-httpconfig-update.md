---
session: 20260711
slug: appsync-httpconfig-update
type: fix
---

## Context

Reviewing the AppSync provider for correctness bugs in update operations. The bug class being hunted is: request/response fields that are parsed but then IGNORED, dimensions/attributes that are accepted but never attached to what's returned.

## Root cause

In `src/robotocore/services/appsync/provider.py`, the `_update_data_source` function was missing the `httpConfig` field in its update loop. The field was correctly stored in `_create_data_source` but silently ignored during updates.

The function only updated these fields:
- type
- description
- serviceRoleArn
- dynamodbConfig
- lambdaConfig

But `httpConfig` was missing, even though it's a valid data source configuration field (for HTTP data sources) that was stored at creation time.

## Fix

Added `httpConfig` to the list of fields that get updated in `_update_data_source`:

```python
if "httpConfig" in params:
    ds["httpConfig"] = params["httpConfig"]
```

## Verification

- 2 new unit tests in `tests/unit/services/test_appsync_bugs.py`:
  - `test_update_data_source_http_config`: Verifies httpConfig is updated
  - `test_update_data_source_preserves_other_fields`: Verifies other fields are preserved when updating httpConfig
- Full unit suite: 8742 passed (2 new), 23 skipped
- Linting: ruff clean
