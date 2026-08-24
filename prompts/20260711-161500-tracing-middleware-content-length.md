---
session: 20260711
slug: tracing-middleware-content-length
type: fix
---

## Context

Found while validating `launchdarkly/terraform#25240` (a BedrockEngineer permission set + Bedrock usage dashboard) against a local robotocore instance: the server logged `h11._util.LocalProtocolError: Too much data for declared Content-Length` mid-response, leaving the client (curl, in that observation) hanging with a half-sent response.

## Root cause

`TracingMiddleware` extended `starlette.middleware.base.BaseHTTPMiddleware`. That base class re-streams the wrapped response body through its own internal memory-channel wrapper rather than forwarding `send()` untouched — a documented Starlette footgun (encode/starlette issues around `BaseHTTPMiddleware` + response streaming) that can emit more bytes than the Content-Length it captured from the wrapped response for larger bodies. h11 raises a protocol error partway through the write, and since the response has already started, the client is left waiting on a connection that will never complete or close cleanly.

## Fix

Rewrote `TracingMiddleware` as a pure ASGI middleware (matching `AWSRoutingMiddleware`'s existing style in the same file), which forwards `send()` messages unmodified — the class of bug is structurally impossible. Request-body reading (for size logging) now uses a cache-and-replay `receive()` wrapper so the downstream app still sees the full request body. `request.state.request_id`/`start_time` move to `scope["state"]`, which is what Starlette's `Request.state` reads from under the hood, so downstream handlers see no behavior change.

Rewrote the three `TestTracingMiddleware` unit tests that called the old `.dispatch()` method directly to instead drive the ASGI interface (`scope`/`receive`/`send`), and added a regression test that specifically proves a large response body is forwarded exactly once, unmodified.

## Note

Separately observed (not fixed here): `terraform apply` on an `aws_cloudwatch_dashboard` resource against robotocore can hang indefinitely with zero requests reaching the server's audit log — a different, client-side issue (likely in the terraform-provider-aws binary or its AWS SDK, not robotocore's HTTP handling). Time-boxed; left as an open finding rather than guessed at further.
