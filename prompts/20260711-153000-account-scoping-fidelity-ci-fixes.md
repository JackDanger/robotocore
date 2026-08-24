---
session: 20260711
slug: account-scoping-fidelity-ci-fixes
type: ci-fix
---

## Context

`fix/account-scoping-fidelity` (PR #286) had 5 failing CI checks: lint, mypy (via `lint`), a compat test regression, an empty-except-pass lint failure, and this prompt log itself.

## Changes

### Lint (`ruff`)

`ruff format` on `tests/integration/test_account_scoping_fidelity.py`, `src/robotocore/services/apigatewayv2/provider.py`, and `src/robotocore/services/events/provider.py` — wrapped lines over 100 chars introduced by the account-scoping diff.

### mypy — `src/robotocore/services/events/provider.py`

`_connections`, `_api_destinations`, and `_endpoints` are keyed by `(account_id, name)` tuples (the account-scoping fix), but their declared type stayed `dict[str, dict]`. Updated to `dict[tuple[str, str], dict]`.

### Compat regression — `tests/compatibility/test_events_compat.py::test_describe_endpoint`

This PR added a real `DescribeEndpoint` handler (previously fell through to Moto, which returned a fixture-shaped response regardless of whether the endpoint existed). The new handler correctly 404s on a nonexistent endpoint — more faithful to real AWS — which broke the test's assumption that `describe_endpoint(Name="test-endpoint")` succeeds with no prior `create_endpoint`. Fixed the test to create-then-describe, and added `test_describe_endpoint_nonexistent_raises` to lock in the new correct 404 behavior.

While fixing that test I found `_describe_endpoint` itself returned the raw internal dict (key `EndpointArn`) instead of the AWS response shape (key `Arn`, matching `create_endpoint`/`update_endpoint`). Fixed the response shaping.

### Empty except:pass — `tests/integration/test_account_scoping_fidelity.py`

Ran `scripts/fix_empty_except.py --write`, which added `# best-effort cleanup` comments to 9 bare `except: pass` blocks per this repo's lint convention.

### Unrelated, found while verifying against a real PR consuming this fix

While validating `launchdarkly/terraform#25240` (a `BedrockEngineer` SSO permission set) against a local robotocore instance, `terraform apply` never converged: `aws_ssoadmin_permission_set` calls `ListTagsForResource` right after create, and moto's SSO Admin backend (`vendor/moto`) doesn't implement `list_tags_for_resource`/`tag_resource`/`untag_resource` at all (HTTP 501). Added all three to `vendor/moto/moto/ssoadmin/{models,responses}.py`, mirroring the existing `create_permission_set`/`describe_permission_set` pattern, plus moto-level tests (`vendor/moto/tests/test_ssoadmin/test_ssoadmin_permission_sets.py`) and robotocore-level compat tests (`tests/compatibility/test_ssoadmin_compat.py::TestSSOAdminTags`).
