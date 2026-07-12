---
session: 20260711
slug: apigatewayv2-stage-update
type: fix
---

## Context

Reviewing the API Gateway v2 provider for correctness bugs in update operations. The bug class being hunted is: request/response fields that are parsed but then IGNORED, dimensions/attributes that are accepted but never attached to what's returned.

## Root cause

In `src/robotocore/services/apigatewayv2/provider.py`, the `_update_stage` function was missing the `AccessLogSettings` and `Tags` fields in its update loop. These fields were correctly stored in `_create_stage` but silently ignored during updates.

The function only updated these fields:
- AutoDeploy
- Description
- StageVariables
- DeploymentId
- DefaultRouteSettings
- RouteSettings

But `AccessLogSettings` and `Tags` were missing, even though they are valid stage configuration fields that were stored at creation time.

## Fix

Added `AccessLogSettings` and `Tags` to the list of fields that get updated in `_update_stage`:

```python
for key in (
    "AutoDeploy",
    "Description",
    "StageVariables",
    "DeploymentId",
    "DefaultRouteSettings",
    "RouteSettings",
    "AccessLogSettings",  # Added
    "Tags",               # Added
):
    if key in params:
        stage[key] = params[key]
```

## Verification

- 3 new unit tests in `tests/unit/services/test_apigatewayv2_bugs.py`:
  - `test_update_stage_access_log_settings`: Verifies AccessLogSettings is updated
  - `test_update_stage_tags`: Verifies Tags are updated
  - `test_update_stage_preserves_other_fields`: Verifies other fields are preserved when updating AccessLogSettings
- Full unit suite: 8742 passed (3 new), 23 skipped
- Linting: ruff clean
