"""Advanced tests for the tracing module: request ID generation,
TracingMiddleware behavior."""

import asyncio
from unittest.mock import patch

from robotocore.observability.tracing import TracingMiddleware, generate_request_id


def _scope(method="GET", path="/", headers=None):
    raw_headers = [(k.lower().encode(), v.encode()) for k, v in (headers or {}).items()]
    return {"type": "http", "method": method, "path": path, "headers": raw_headers}


def _receive(body=b""):
    state = {"sent": False}

    async def receive():
        if state["sent"]:
            return {"type": "http.disconnect"}
        state["sent"] = True
        return {"type": "http.request", "body": body, "more_body": False}

    return receive


def _send(messages):
    async def send(message):
        messages.append(message)

    return send


def _downstream_app(status=200, headers=None, body=b""):
    """A minimal ASGI app that echoes a canned response and records the scope
    it was actually called with (so tests can assert on `scope["state"]`)."""
    seen = {}

    async def app(scope, receive, send):
        seen["scope"] = scope
        await receive()  # must be able to read the replayed request body
        await send(
            {
                "type": "http.response.start",
                "status": status,
                "headers": [(k.encode(), v.encode()) for k, v in (headers or {}).items()],
            }
        )
        await send({"type": "http.response.body", "body": body})

    return app, seen


class TestGenerateRequestId:
    def test_returns_string(self):
        rid = generate_request_id()
        assert isinstance(rid, str)

    def test_unique_ids(self):
        ids = {generate_request_id() for _ in range(100)}
        assert len(ids) == 100

    def test_uuid_format(self):
        """Request IDs should be valid UUIDs (contain hyphens, 36 chars)."""
        rid = generate_request_id()
        assert len(rid) == 36
        assert rid.count("-") == 4


class TestTracingMiddleware:
    def test_adds_request_id_header(self):
        app, _seen = _downstream_app(status=200)
        middleware = TracingMiddleware(app)

        async def _run():
            messages = []
            with patch("robotocore.observability.tracing.log_request"):
                with patch("robotocore.observability.tracing.log_response"):
                    await middleware(_scope(method="POST"), _receive(b""), _send(messages))

            start = next(m for m in messages if m["type"] == "http.response.start")
            headers = dict(start["headers"])
            assert b"x-amz-request-id" in headers
            assert b"x-robotocore-request-id" in headers
            assert headers[b"x-amz-request-id"] == headers[b"x-robotocore-request-id"]

        asyncio.run(_run())

    def test_sets_scope_state(self):
        app, seen = _downstream_app(status=200)
        middleware = TracingMiddleware(app)

        async def _run():
            messages = []
            with patch("robotocore.observability.tracing.log_request"):
                with patch("robotocore.observability.tracing.log_response"):
                    await middleware(
                        _scope(method="GET", path="/test"), _receive(b""), _send(messages)
                    )

            state = seen["scope"]["state"]
            assert isinstance(state["request_id"], str)
            assert isinstance(state["start_time"], float)

        asyncio.run(_run())

    def test_calls_log_request_and_log_response(self):
        app, _seen = _downstream_app(status=200, headers={"content-length": "42"}, body=b"x" * 42)
        middleware = TracingMiddleware(app)

        async def _run():
            messages = []
            with patch("robotocore.observability.tracing.log_request") as mock_log_req:
                with patch("robotocore.observability.tracing.log_response") as mock_log_resp:
                    await middleware(
                        _scope(method="POST", headers={"content-type": "application/json"}),
                        _receive(b'{"key": "value"}'),
                        _send(messages),
                    )

            mock_log_req.assert_called_once()
            mock_log_resp.assert_called_once()
            call_kwargs = mock_log_resp.call_args
            assert call_kwargs.kwargs["status_code"] == 200
            assert call_kwargs.kwargs["body_size"] == 42

        asyncio.run(_run())

    def test_does_not_double_send_large_body(self):
        """The bug this middleware exists to avoid: BaseHTTPMiddleware could emit
        more body bytes than the Content-Length it captured. A pure ASGI middleware
        forwards send() messages unmodified, so the body sent downstream must equal
        the body the wrapped app produced, exactly once."""
        big_body = ("x" * 10_000).encode()
        app, _seen = _downstream_app(
            status=200, headers={"content-length": str(len(big_body))}, body=big_body
        )
        middleware = TracingMiddleware(app)

        async def _run():
            messages = []
            with patch("robotocore.observability.tracing.log_request"):
                with patch("robotocore.observability.tracing.log_response"):
                    await middleware(_scope(), _receive(b""), _send(messages))

            body_messages = [m for m in messages if m["type"] == "http.response.body"]
            total = b"".join(m["body"] for m in body_messages)
            assert len(total) == len(big_body)
            assert total == big_body

        asyncio.run(_run())

    def test_non_http_scope_passes_through_untouched(self):
        """Lifespan/websocket scopes must be forwarded as-is, no request tracing."""
        calls = []

        async def app(scope, receive, send):
            calls.append(scope["type"])

        middleware = TracingMiddleware(app)

        async def _run():
            await middleware({"type": "lifespan"}, _receive(), lambda m: None)

        asyncio.run(_run())
        assert calls == ["lifespan"]
