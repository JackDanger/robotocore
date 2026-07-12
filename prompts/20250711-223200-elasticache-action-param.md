---
session: "elasticache-action-param"
slug: "elasticache-action-param-bug"
type: "bugfix"
---

## Context

While reviewing the ElastiCache native provider, I found that the `Action` parameter was not being correctly parsed when it was in the query string but the request body was form-urlencoded. The AWS ElastiCache API uses the query protocol, which allows parameters to be in either the query string or the body (or both).

## Root Cause

In `handle_elasticache_request()`, the code checked if the content-type was `x-www-form-urlencoded` and if so, only parsed the body. If the content-type was not form-urlencoded, it only parsed the query string. This meant that if the `Action` parameter was in the query string but other parameters were in the body (with form-urlencoded content-type), the `Action` would be ignored.

## Fix

Modified `src/robotocore/services/elasticache/provider.py`:

1. Changed the parsing logic to always parse the query string first.

2. If the content-type is form-urlencoded, also parse the body and merge the parameters (body parameters override query string parameters).

3. This ensures that the `Action` parameter is found regardless of whether it's in the query string or body.

## Verification

Added tests in `tests/unit/services/test_elasticache_bugs.py`:
- `test_action_in_query_string_with_form_body`: Verifies that Action in query string is recognized even when body is form-urlencoded

All tests pass, and the existing ElastiCache test suite (196 tests) continues to pass.
