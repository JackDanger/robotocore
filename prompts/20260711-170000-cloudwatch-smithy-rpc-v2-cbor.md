---
session: 20260711
slug: cloudwatch-smithy-rpc-v2-cbor
type: fix
---

## Context

Follow-up to the previous `TracingMiddleware` fix (`fix/tracing-middleware-content-length`), which turned out not to be the actual cause of the reported symptom: `terraform apply` on `aws_cloudwatch_dashboard` hanging against robotocore for minutes, appearing as an unresolvable client-side issue (zero requests reaching the audit log).

Root-caused with `TF_LOG=trace`: the real issue was in my own test harness (a stale port in a hand-written `_override.tf`, pointing at a different, long-running robotocore instance instead of the one being iterated on) — masking the actual server-side bug underneath.

## Root cause

`terraform-provider-aws` 6.54.0 (`aws-sdk-go-v2`) sends `CloudWatch/PutDashboard` using AWS's newer **Smithy RPC v2 CBOR** protocol: a binary CBOR-encoded body, POSTed to `/service/{ServiceId}/operation/{OperationName}` with a `smithy-protocol: rpc-v2-cbor` header — no `X-Amz-Target`, no JSON, no query string.

`handle_cloudwatch_request` only recognized two protocols (JSON via `X-Amz-Target`, and classic query/form-encoded). Neither branch matched, so the request fell through with `action = ""`, matched no `_ACTION_MAP` entry, and was forwarded to Moto — whose query-protocol dispatcher tried to UTF-8-decode the raw CBOR bytes and raised `UnicodeDecodeError: 'utf-8' codec can't decode byte 0xa2 in position 0`.

That surfaced as an HTTP 500, which `aws-sdk-go-v2` treats as retryable — it retried with exponential backoff up to 25 attempts (~2 minutes), which is indistinguishable from a hang within any test window shorter than that.

## Fix

- Added `cbor2` as a dependency.
- `handle_cloudwatch_request` now detects `smithy-protocol: rpc-v2-cbor` before the JSON/query branches, decodes the body with `cbor2.loads`, and reads the operation name from the URL path (`/operation/{name}`) since there's no `X-Amz-Target` to parse.
- Added `_success_response`/`_error_body_response` helpers so every response site (6 call sites previously duplicated the `if use_json_protocol: ... else: ...` shape) picks JSON, CBOR, or XML consistently — CBOR responses get `media_type="application/cbor"` and the `smithy-protocol` header back, which the AWS SDK's CBOR deserializer requires.
- An unmapped CBOR action now fails closed with a real `501 NotImplemented` (still CBOR-encoded) instead of falling through to Moto, which has no CBOR support at all and would hit the identical UTF-8 crash.

## Verification

- Real `terraform apply`/`destroy` on the actual PR HCL (`aws_cloudwatch_dashboard` + 6 sibling resources from a Bedrock-usage-logging module) now completes in ~0s per resource instead of hanging.
- 3 new unit tests (`TestCloudWatchCborProtocol`): a full CBOR round-trip, a CBOR-encoded error response, and a negative test proving an unmapped CBOR action does NOT fall through to Moto.
- Full local suite: 8732 unit (3 new) + 140 integration/compat passed. `ruff`/`mypy`/`bandit` clean.
