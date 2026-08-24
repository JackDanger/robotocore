---
session: 20260711
slug: elasticache-action-param
type: fix
---

## Context

Part of a broad Bedrock-agent sweep for silent-correctness bugs across robotocore's native providers.

## Root cause

`handle_elasticache_request` chose ONE source for request parameters based on content-type: if `x-www-form-urlencoded`, it parsed only the body; otherwise, only the query string. A request with `Action` in the query string but a form-urlencoded body (even a mostly-empty one) would silently lose the query-string parameters entirely, including `Action` itself.

## Fix

Always parse the query string first, then merge in body parameters (form-urlencoded only) on top, with body values taking precedence on overlap.

## Verification

- New test: `test_action_in_query_string_with_form_body`.
- Full local suite: 8745 unit passed. `ruff`/`ruff format`/`mypy` clean.

## Confidence note

Lower confidence than the S3/ECR fixes in this same PR: I couldn't confirm a real AWS SDK actually sends a query-protocol request with parameters split across query string and form body this way (the typical pattern is all-query-string OR all-body, not a mix). The fix is a safe superset — normal single-source requests behave identically — so it's low-risk to include even if the exact trigger scenario turns out to be rare or theoretical.
