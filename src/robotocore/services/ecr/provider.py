"""Native ECR provider.

Intercepts operations that Moto doesn't implement or has bugs:
- BatchCheckLayerAvailability: Not implemented in Moto
- DescribeRepositories: maxResults pagination not enforced

Also routes OCI Registry v2 data plane requests (/v2/...).
"""

import json

from starlette.requests import Request
from starlette.responses import Response

from robotocore.providers.moto_bridge import forward_to_moto
from robotocore.services.ecr.registry import handle_registry_v2_request


async def handle_ecr_request(request: Request, region: str, account_id: str) -> Response:
    """Handle ECR requests, including OCI Registry v2 data plane."""
    # Check if this is a registry v2 request (path starts with /v2/)
    path = request.url.path
    if path.startswith("/v2/") or path == "/v2":
        return await handle_registry_v2_request(request, region, account_id)
    """Handle ECR requests, intercepting unimplemented operations."""
    target = request.headers.get("x-amz-target", "")
    action = target.split(".")[-1] if "." in target else ""

    body = await request.body()

    if action == "BatchCheckLayerAvailability":
        try:
            params = json.loads(body) if body else {}
        except json.JSONDecodeError as e:
            return Response(
                content=json.dumps(
                    {"__type": "InvalidParameterException", "message": f"Invalid JSON: {e}"}
                ),
                status_code=400,
                media_type="application/x-amz-json-1.1",
            )
        digests = params.get("layerDigests", [])
        repo_name = params.get("repositoryName", "")
        registry_id = params.get("registryId", account_id)
        layers = []
        for digest in digests:
            layers.append(
                {
                    "layerDigest": digest,
                    "layerAvailability": "UNAVAILABLE",
                    "repositoryName": repo_name,
                    "registryId": registry_id,
                }
            )
        return Response(
            content=json.dumps({"layers": layers, "failures": []}),
            status_code=200,
            media_type="application/x-amz-json-1.1",
        )

    if action == "DescribeRepositories":
        try:
            params = json.loads(body) if body else {}
        except json.JSONDecodeError as e:
            return Response(
                content=json.dumps(
                    {"__type": "InvalidParameterException", "message": f"Invalid JSON: {e}"}
                ),
                status_code=400,
                media_type="application/x-amz-json-1.1",
            )
        max_results = params.get("maxResults")
        next_token = params.get("nextToken")

        if max_results or next_token:
            # Forward to Moto, then handle pagination
            response = await forward_to_moto(request, "ecr", account_id=account_id)
            resp_body = json.loads(response.body)
            repos = resp_body.get("repositories", [])

            # Apply offset from nextToken
            start_idx = 0
            if next_token:
                try:
                    start_idx = int(next_token)
                except ValueError:
                    start_idx = 0
            repos = repos[start_idx:]

            # Apply maxResults limit
            if max_results and len(repos) > max_results:
                resp_body["repositories"] = repos[:max_results]
                # Set nextToken to the next position
                resp_body["nextToken"] = str(start_idx + max_results)
            else:
                resp_body["repositories"] = repos
                # Remove nextToken if there are no more results
                resp_body.pop("nextToken", None)

            return Response(
                content=json.dumps(resp_body),
                status_code=response.status_code,
                media_type="application/x-amz-json-1.1",
            )

    return await forward_to_moto(request, "ecr", account_id=account_id)
