---
session: "20260712-123923"
slug: "update-field-persistence-integration-tests"
type: test
timestamp: "2026-07-12T12:39:23Z"
---

# Integration Tests for Update Field Persistence Bug Class

## Task
Write integration tests that lock in fixes for the "Update operation silently drops fields that Create stores" bug class across four services:

1. EventBridge Scheduler `UpdateSchedule` - fields: `ScheduleExpressionTimezone`, `StartDate`, `EndDate`, `KmsKeyArn`
2. AppSync `UpdateDataSource` - field: `httpConfig`
3. API Gateway v2 `UpdateStage` - field: `AccessLogSettings`
4. Cognito `CreateUserPoolClient` - fields: `LogoutURLs`, `DefaultRedirectURI`, `ReadAttributes`, `WriteAttributes`, `SupportedIdentityProviders`, `AllowedOAuthFlowsUserPoolClient`, `TokenValidityUnits`, `AccessTokenValidity`, `IdTokenValidity`, `RefreshTokenValidity`

## Implementation

Created `tests/integration/test_update_field_persistence.py` with 13 integration tests:

- 4 tests for EventBridge Scheduler (timezone, start date, end date, KMS key)
- 1 test for AppSync (httpConfig)
- 1 test for API Gateway v2 (AccessLogSettings)
- 7 tests for Cognito (all the create-time fields)

All tests follow the house style from existing integration tests:
- Use `make_boto_client` fixture for real boto3 clients
- Use `uuid.uuid4().hex[:8]` for unique resource names
- Clean up resources in `try`/`finally` blocks
- Clear assertion messages naming the bug they'd catch

## Bug Sweep Findings

During the grep sweep for additional instances of this bug class, found 2 potential new instances in API Gateway v2:

1. `_update_route` in `src/robotocore/services/apigatewayv2/provider.py` - may drop `ModelSelectionExpression`, `RequestModels`, `RequestParameters`, `RouteResponseSelectionExpression` (stored in create, not handled in update)

2. `_update_integration` in `src/robotocore/services/apigatewayv2/provider.py` - may drop `ResponseParameters`, `TemplateSelectionExpression`, `CredentialsArn` (stored in create, not handled in update)

These are noted for follow-up investigation but not fixed in this task (out of scope).

## Verification

All 13 tests pass:
```
uv run --no-sync python -m pytest tests/integration/test_update_field_persistence.py -v
============================== 13 passed in 7.10s ==============================
```

Full integration suite passes (86 passed, 4 skipped):
```
uv run --no-sync python -m pytest tests/integration/ -q
86 passed, 4 skipped in 27.76s
```

Linting clean:
```
uv run --no-sync ruff check tests/integration/test_update_field_persistence.py
uv run --no-sync ruff format --check tests/integration/test_update_field_persistence.py
uv run --no-sync mypy tests/integration/test_update_field_persistence.py
```
