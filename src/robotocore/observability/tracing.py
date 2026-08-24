"""Request tracing middleware for Robotocore.

Assigns a unique request ID (X-Amz-Request-Id) to every request,
tracks request timing, and optionally exports traces via OpenTelemetry.
"""

import logging
import os
import time
import uuid

from starlette.requests import Request

from robotocore.observability.logging import log_request, log_response

logger = logging.getLogger(__name__)

# Optional OpenTelemetry tracer
_tracer = None


def _get_tracer():
    """Lazily initialize OpenTelemetry tracer if OTEL endpoint is configured."""
    global _tracer
    if _tracer is not None:
        return _tracer

    endpoint = os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT")
    if not endpoint:
        return None

    try:
        from opentelemetry import trace
        from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import (
            OTLPSpanExporter,
        )
        from opentelemetry.sdk.resources import Resource
        from opentelemetry.sdk.trace import TracerProvider
        from opentelemetry.sdk.trace.export import BatchSpanProcessor

        resource = Resource.create({"service.name": "robotocore"})
        provider = TracerProvider(resource=resource)
        exporter = OTLPSpanExporter(endpoint=endpoint)
        provider.add_span_processor(BatchSpanProcessor(exporter))
        trace.set_tracer_provider(provider)
        _tracer = trace.get_tracer("robotocore")
        logger.info("OpenTelemetry tracing enabled, exporting to %s", endpoint)
        return _tracer
    except ImportError:
        logger.debug("OpenTelemetry packages not installed; tracing disabled")
        return None


def generate_request_id() -> str:
    """Generate a unique request ID in AWS format."""
    return str(uuid.uuid4())


class TracingMiddleware:
    """Adds request tracing and timing.

    Pure ASGI, not `starlette.middleware.base.BaseHTTPMiddleware`: that base class
    re-streams the wrapped response body through its own memory-channel, and for
    response bodies past a certain size can emit more bytes than the Content-Length
    it captured — h11 raises `LocalProtocolError: Too much data for declared
    Content-Length` mid-write, which leaves the client (e.g. terraform's AWS
    provider) hanging forever with no response and no closed connection, since the
    error happens after the response has already started. A pure ASGI middleware
    forwards `send()` unmodified, so it can't double the body.
    """

    def __init__(self, app):
        self.app = app

    async def __call__(self, scope, receive, send) -> None:
        if scope["type"] != "http":
            await self.app(scope, receive, send)
            return

        request_id = generate_request_id()
        start_time = time.monotonic()
        scope.setdefault("state", {})
        scope["state"]["request_id"] = request_id
        scope["state"]["start_time"] = start_time

        # Drain the body once (to log its size) and cache the ASGI messages so the
        # downstream app — which builds its own Request from `receive` — sees the
        # same messages replayed, rather than an already-exhausted stream.
        cached_messages = []
        while True:
            message = await receive()
            cached_messages.append(message)
            if message["type"] != "http.request" or not message.get("more_body", False):
                break
        body = b"".join(m.get("body", b"") for m in cached_messages if m["type"] == "http.request")

        async def replay_receive():
            if cached_messages:
                return cached_messages.pop(0)
            return await receive()

        request = Request(scope)
        log_request(
            logger,
            method=request.method,
            path=request.url.path,
            headers=dict(request.headers),
            body_size=len(body),
            request_id=request_id,
        )

        response_state = {"status_code": 0, "content_length": 0}

        async def send_wrapper(message):
            if message["type"] == "http.response.start":
                response_state["status_code"] = message["status"]
                headers = list(message.get("headers", []))
                headers.append((b"x-amz-request-id", request_id.encode()))
                headers.append((b"x-robotocore-request-id", request_id.encode()))
                for key, value in headers:
                    if key.lower() == b"content-length":
                        response_state["content_length"] = int(value)
                message = {**message, "headers": headers}
            await send(message)

        tracer = _get_tracer()
        if tracer:
            with tracer.start_as_current_span(
                f"{request.method} {request.url.path}",
                attributes={
                    "http.method": request.method,
                    "http.url": str(request.url),
                    "robotocore.request_id": request_id,
                },
            ):
                await self.app(scope, replay_receive, send_wrapper)
        else:
            await self.app(scope, replay_receive, send_wrapper)

        duration_ms = (time.monotonic() - start_time) * 1000
        log_response(
            logger,
            status_code=response_state["status_code"],
            body_size=response_state["content_length"],
            duration_ms=duration_ms,
            request_id=request_id,
        )
