"""Moto bridge sidecar - wraps Moto backends behind a minimal HTTP server.

This is the Python component of the Rust-Moto bridge. The Rust robotocore
server handles native services directly and proxies all other AWS service
requests to this sidecar, which dispatches them to Moto backends.

Run with:
    python scripts/moto_sidecar.py --port 4567

The Rust server talks to this over HTTP on localhost.
"""

import argparse
import json
import logging
import os

os.environ.setdefault("MOTO_ALLOW_NONEXISTENT_REGION", "true")

import moto.backends as moto_backends
import uvicorn
from moto.core.base_backend import BackendDict
from starlette.applications import Starlette
from starlette.requests import Request
from starlette.responses import Response
from starlette.routing import Route
from werkzeug.routing import Map, Rule
from werkzeug.routing.converters import BaseConverter
from werkzeug.test import EnvironBuilder
from werkzeug.wrappers import Request as WerkzeugRequest

DEFAULT_ACCOUNT_ID = "123456789012"

logger = logging.getLogger("moto_sidecar")


class WerkzeugRawBodyRequest(WerkzeugRequest):
    """Werkzeug request that preserves a raw body for Moto."""

    def __init__(self, environ: dict, body: bytes):
        super().__init__(environ)
        self.body = body

    @property
    def data(self) -> bytes:
        return self.body

    def get_data(self, *args, **kwargs) -> bytes:
        return self.body

    @property
    def form(self):
        raise AttributeError("form parsing disabled")

    @property
    def files(self):
        raise AttributeError("file parsing disabled")


class _RegexConverter(BaseConverter):
    part_isolating = False

    def __init__(self, map, *args, **kwargs):
        super().__init__(map, *args, **kwargs)
        self.regex = args[0] if args else ".*"


_routing_cache: dict[str, Map] = {}


def _get_routing_table(service: str) -> Map:
    """Build and cache a Werkzeug URL Map from a Moto backend's flask_paths."""
    if service in _routing_cache:
        return _routing_cache[service]

    backend_dict = moto_backends.get_backend(service)
    if isinstance(backend_dict, BackendDict):
        if DEFAULT_ACCOUNT_ID not in backend_dict:
            backend_dict[DEFAULT_ACCOUNT_ID] = {}
        if "us-east-1" in backend_dict[DEFAULT_ACCOUNT_ID]:
            backend = backend_dict[DEFAULT_ACCOUNT_ID]["us-east-1"]
        else:
            backend = backend_dict[DEFAULT_ACCOUNT_ID]["global"]
    else:
        backend = backend_dict["global"]

    url_map = Map()
    url_map.converters["regex"] = _RegexConverter

    for url_path, handler in backend.flask_paths.items():
        if url_path in ("", "/"):
            url_map.add(Rule("/", endpoint=handler, strict_slashes=False))
            continue
        if url_path in ("/.*", "/.+"):
            url_map.add(Rule("/<path:__catch_all>", endpoint=handler, strict_slashes=False))
            url_map.add(Rule("/", endpoint=handler, strict_slashes=False))
            continue
        url_map.add(Rule(url_path, endpoint=handler, strict_slashes=False))

    _routing_cache[service] = url_map
    return url_map


def _extract_service(request: Request) -> str:
    """Extract the AWS service name from the request."""
    # 1. X-Amz-Target header
    target = request.headers.get("x-amz-target", "")
    if target:
        prefix = target.split(".")[0]
        # Map known prefixes
        prefix_map = {
            "DynamoDB": "dynamodb",
            "DynamoDBStreams": "dynamodbstreams",
            "SecretsManager": "secretsmanager",
            "secretsmanager": "secretsmanager",
            "AmazonSSM": "ssm",
            "TrentService": "kms",
            "Kinesis_20131202": "kinesis",
            "Firehose_20150804": "firehose",
            "Logs_20140328": "logs",
            "AWSEvents": "events",
            "CloudWatchEvents": "events",
            "monitoring": "cloudwatch",
            "AmazonEC2ContainerRegistry": "ecr",
            "AmazonEC2ContainerServiceV20141113": "ecs",
            "AWSStepFunctions": "stepfunctions",
            "CertificateManager": "acm",
            "RekognitionService": "rekognition",
            "StarlingDoveService": "config",
            "OvertureService": "support",
            "AWSSupport": "support",
        }
        if prefix in prefix_map:
            return prefix_map[prefix]
        # Fallback: use the prefix as-is (lowercase)
        return prefix.lower()

    # 2. Authorization header (credential scope)
    auth = request.headers.get("authorization", "")
    if "Credential=" in auth:
        try:
            cred_part = auth.split("Credential=")[1].split("/")[0]
            # Format: ACCESS_KEY_DATE_REGION_SERVICE
            parts = cred_part.split("_")
            if len(parts) >= 4:
                return parts[3]
        except (IndexError, AttributeError):
            pass

    # 3. Host header
    host = request.headers.get("host", "")
    if ".amazonaws.com" in host or ".amazonaws.com.cn" in host:
        service = host.split(".")[0]
        if service and service != "s3":
            return service

    # 4. Path-based detection
    path = request.url.path
    if path.startswith("/2015-03-31/"):
        return "lambda"
    if path.startswith("/2018-01-01/"):
        return "lambda"
    if path.startswith("/2016-11-15/"):
        return "apigatewayv2"

    # 5. Body-based (Action= parameter for query protocol)
    try:
        body = (
            request.url.query.decode()
            if hasattr(request.url.query, "decode")
            else str(request.url.query)
        )
        if "Action=" in body:
            pass  # need auth header to map action to service
    except Exception:
        pass

    return ""


async def moto_handler(request: Request) -> Response:
    """Main handler - dispatch to Moto backend."""
    body = await request.body()

    # Get the service from the X-Robotocore-Service header (set by Rust server)
    # or extract it from the request
    service = request.headers.get("x-robotocore-service", "")
    if not service:
        service = _extract_service(request)

    if not service:
        return Response(
            content=json.dumps(
                {"__type": "UnknownService", "message": "Could not determine service"}
            ),
            status_code=400,
            media_type="application/json",
        )

    # Find the Moto backend
    try:
        routing_map = _get_routing_table(service)
    except Exception as e:
        return Response(
            content=json.dumps(
                {
                    "__type": "ServiceNotAvailable",
                    "message": f"Moto backend not found for {service}: {e}",
                }
            ),
            status_code=501,
            media_type="application/json",
        )

    # Build the Werkzeug environ
    method = request.method
    path = request.url.path
    query = (
        request.url.query.decode()
        if hasattr(request.url.query, "decode")
        else str(request.url.query)
    )

    # Build headers
    headers = {}
    for key, value in request.headers.items():
        headers[key] = value

    environ = EnvironBuilder(
        path=f"{path}?{query}" if query else path,
        method=method,
        data=body,
        headers=headers,
    ).get_environ()

    # Route to the correct Moto handler
    try:
        binding = routing_map.bind_to_environ(environ)
        endpoint, args = binding.match()
    except Exception:
        # Try a catch-all
        try:
            binding = routing_map.bind_to_environ(environ)
            endpoint, args = binding.match()
        except Exception:
            return Response(
                content=json.dumps(
                    {
                        "__type": "NotFound",
                        "message": f"No Moto handler for {service} {method} {path}",
                    }
                ),
                status_code=404,
                media_type="application/json",
            )

    # Create the Werkzeug request
    werkzeug_request = WerkzeugRawBodyRequest(environ, body)

    # Call the Moto handler
    try:
        response = endpoint(werkzeug_request, **args)
        status_code = getattr(response, "status_code", 200)
        if isinstance(status_code, str):
            status_code = int(status_code.split()[0])
        resp_body = response.get_data()
        content_type = response.headers.get("Content-Type", "application/json")

        # Build Starlette response
        resp_headers = {}
        for key, value in response.headers.items():
            if key.lower() not in ("content-length", "content-type"):
                resp_headers[key] = value

        return Response(
            content=resp_body,
            status_code=status_code,
            media_type=content_type,
            headers=resp_headers,
        )
    except Exception as e:
        import traceback

        traceback.print_exc()
        return Response(
            content=json.dumps({"__type": "InternalError", "message": str(e)}),
            status_code=500,
            media_type="application/json",
        )


async def health(request: Request) -> Response:
    """Health check endpoint."""
    services = sorted(moto_backends.backends.keys()) if hasattr(moto_backends, "backends") else []
    return Response(
        content=json.dumps(
            {
                "status": "ok",
                "service": "moto-sidecar",
                "moto_version": __import__("moto").__version__,
                "services": len(services),
            }
        ),
        media_type="application/json",
    )


def create_app() -> Starlette:
    """Create the Starlette ASGI application."""
    return Starlette(
        routes=[
            Route("/_sidecar/health", health),
            Route("/{service:path}", moto_handler),
        ],
    )


def main():
    parser = argparse.ArgumentParser(description="Moto bridge sidecar")
    parser.add_argument("--port", type=int, default=4567)
    parser.add_argument("--host", type=str, default="127.0.0.1")
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO)
    logger.info(f"Starting Moto sidecar on {args.host}:{args.port}")

    app = create_app()
    uvicorn.run(app, host=args.host, port=args.port, log_level="warning")


if __name__ == "__main__":
    main()
