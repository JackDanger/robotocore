"""ECR OCI Registry v2 data plane implementation.

Implements the OCI Distribution Specification for Docker/ECR-compatible
registry operations, including manifest and blob storage.

See: https://github.com/opencontainers/distribution-spec/blob/main/spec.md
"""

from __future__ import annotations

import hashlib
import json
import logging
import re
import uuid
from dataclasses import dataclass, field
from datetime import UTC, datetime
from typing import Any

from starlette.requests import Request
from starlette.responses import Response

logger = logging.getLogger(__name__)

# OCI media types
OCI_MANIFEST_MEDIA_TYPE = "application/vnd.oci.image.manifest.v1+json"
DOCKER_MANIFEST_MEDIA_TYPE = "application/vnd.docker.distribution.manifest.v2+json"
DOCKER_MANIFEST_LIST_MEDIA_TYPE = "application/vnd.docker.distribution.manifest.list.v2+json"
OCI_INDEX_MEDIA_TYPE = "application/vnd.oci.image.index.v1+json"

# Upload states (in-memory)
_upload_sessions: dict[str, dict[str, Any]] = {}

# Blob storage (in-memory, keyed by digest)
# Format: {digest: content_bytes}
_blob_store: dict[str, bytes] = {}


@dataclass
class RegistryImage:
    """Represents an image in the registry."""

    digest: str
    manifest: bytes
    media_type: str
    size: int
    tags: list[str] = field(default_factory=list)
    pushed_at: datetime = field(default_factory=lambda: datetime.now(UTC))
    scan_status: str = "PENDING"
    scan_findings: list[dict[str, Any]] = field(default_factory=list)

    def to_manifest_response(self) -> dict[str, str]:
        """Return manifest response headers."""
        return {
            "Content-Type": self.media_type,
            "Content-Length": str(self.size),
            "Docker-Content-Digest": self.digest,
        }


@dataclass
class RegistryBlob:
    """Represents a blob (layer) in the registry."""

    digest: str
    content: bytes
    size: int


def _compute_digest(content: bytes) -> str:
    """Compute sha256 digest for content."""
    return "sha256:" + hashlib.sha256(content).hexdigest()


def _parse_repository_name(path: str) -> str:
    """Extract repository name from /v2/{name}/... path."""
    # Path format: /v2/{name}/manifests/{reference} or /v2/{name}/blobs/{digest}
    parts = path.split("/")
    # parts[0] is empty (leading /), parts[1] is 'v2'
    # Repository name is everything between /v2/ and the next action (manifests/blobs/uploads)
    if len(parts) < 3:
        return ""

    # Find the action part (manifests, blobs, uploads)
    action_idx = -1
    for i, part in enumerate(parts[2:], start=2):
        if part in ("manifests", "blobs", "uploads"):
            action_idx = i
            break

    if action_idx == -1:
        # No action found, return everything after /v2/
        return "/".join(parts[2:])

    return "/".join(parts[2:action_idx])


def _extract_auth_token(request: Request) -> str | None:
    """Extract bearer token from Authorization header."""
    auth = request.headers.get("authorization", "")
    if auth.startswith("Bearer "):
        return auth[7:]
    if auth.startswith("Basic "):
        import base64

        try:
            decoded = base64.b64decode(auth[6:]).decode("utf-8")
            # ECR format: AWS:<token>
            if decoded.startswith("AWS:"):
                return decoded[4:]
            # Return the whole thing as token
            return decoded
        except Exception as e:
            logger.debug("Failed to decode basic auth: %s", e)
            return None
    return None


def _verify_auth(request: Request, account_id: str) -> bool:
    """Verify the request is authenticated for the given account."""
    # ECR GetAuthorizationToken returns base64(AWS:<token>)
    # The token should contain the account ID
    token = _extract_auth_token(request)
    if not token:
        return False

    # For now, accept any non-empty token that looks like it came from ECR
    # In real ECR, the token encodes the account and region
    return True


def _get_ecr_backend(region: str, account_id: str) -> Any:
    """Get the Moto ECR backend for the given region/account."""
    from moto.backends import get_backend

    return get_backend("ecr")[account_id][region]


def _get_repository(backend: Any, repo_name: str, registry_id: str | None = None) -> Any | None:
    """Get a repository from the ECR backend."""
    try:
        return backend._get_repository(repo_name, registry_id)
    except Exception as e:
        logger.debug("Repository not found: %s", e)
        return None


def _image_to_oci_manifest(image: Any) -> dict[str, Any]:
    """Convert an ECR Image to OCI manifest format."""
    try:
        manifest = json.loads(image.image_manifest)
        return manifest
    except json.JSONDecodeError:
        # Return a minimal manifest
        return {
            "schemaVersion": 2,
            "mediaType": DOCKER_MANIFEST_MEDIA_TYPE,
            "config": {
                "mediaType": "application/vnd.docker.container.image.v1+json",
                "size": 0,
                "digest": "sha256:" + "0" * 64,
            },
            "layers": [],
        }


def _get_image_by_reference(repository: Any, reference: str) -> Any | None:
    """Get an image by tag or digest from a repository."""
    for image in repository.images:
        # Check by digest
        if image.get_image_digest() == reference:
            return image
        # Check by tag
        if reference in image.image_tags:
            return image
    return None


def _check_image_scan_status(repository: Any, image: Any) -> None:
    """Update image scan status based on repository scan configuration."""
    scan_config = getattr(repository, "image_scanning_configuration", {}) or {}
    if scan_config.get("scanOnPush", False):
        # Simulate scan completion
        if not hasattr(image, "scan_status") or image.scan_status == "PENDING":
            image.scan_status = "COMPLETE"
            image.scan_findings = []


async def handle_registry_v2_request(request: Request, region: str, account_id: str) -> Response:
    """Handle OCI Registry v2 API requests.

    Routes:
    - GET /v2/ - Check registry availability
    - GET /v2/{name}/manifests/{reference} - Get manifest by tag or digest
    - PUT /v2/{name}/manifests/{reference} - Push manifest
    - DELETE /v2/{name}/manifests/{reference} - Delete manifest
    - GET /v2/{name}/blobs/{digest} - Get blob
    - HEAD /v2/{name}/blobs/{digest} - Check blob exists
    - DELETE /v2/{name}/blobs/{digest} - Delete blob
    - POST /v2/{name}/blobs/uploads/ - Start blob upload
    - PATCH /v2/{name}/blobs/uploads/{uuid} - Upload chunk
    - PUT /v2/{name}/blobs/uploads/{uuid} - Complete upload
    - GET /v2/{name}/tags/list - List tags
    """
    path = request.url.path
    method = request.method.upper()

    logger.debug("Registry v2 request: %s %s", method, path)

    # Check authentication (except for /v2/ ping which can be unauthenticated)
    if path != "/v2/" and not _verify_auth(request, account_id):
        return _make_error_response(
            "UNAUTHORIZED",
            "authentication required",
            401,
            {"WWW-Authenticate": 'Bearer realm="ecr"'},
        )

    # Route to appropriate handler
    if path == "/v2/":
        return await _handle_v2_ping(request)

    # Parse repository name
    repo_name = _parse_repository_name(path)
    if not repo_name:
        return _make_error_response("NAME_INVALID", "invalid repository name", 400)

    # Get the ECR backend
    backend = _get_ecr_backend(region, account_id)

    # Match path patterns
    # Manifest operations: /v2/{name}/manifests/{reference}
    manifest_match = re.match(r"^/v2/(.+)/manifests/(.+)$", path)
    if manifest_match:
        repo_from_path = manifest_match.group(1)
        reference = manifest_match.group(2)
        # URL decode the reference (it might be URL encoded)
        from urllib.parse import unquote

        reference = unquote(reference)

        if method == "GET":
            return await _handle_get_manifest(
                request, backend, repo_from_path, reference, account_id
            )
        if method == "PUT":
            return await _handle_put_manifest(
                request, backend, repo_from_path, reference, account_id
            )
        if method == "DELETE":
            return await _handle_delete_manifest(
                request, backend, repo_from_path, reference, account_id
            )
        return _make_error_response("UNSUPPORTED", "method not allowed", 405)

    # Upload operations: /v2/{name}/blobs/uploads/ (POST to start)
    # Must come before blob operations to avoid matching 'uploads' as a digest
    uploads_match = re.match(r"^/v2/(.+)/blobs/uploads/?$", path)
    if uploads_match and method == "POST":
        repo_from_path = uploads_match.group(1)
        return await _handle_start_upload(request, backend, repo_from_path, account_id)

    # Upload chunk/completion: /v2/{name}/blobs/uploads/{uuid}
    upload_chunk_match = re.match(r"^/v2/(.+)/blobs/uploads/([^/]+)$", path)
    if upload_chunk_match:
        repo_from_path = upload_chunk_match.group(1)
        upload_uuid = upload_chunk_match.group(2)

        if method == "PATCH":
            return await _handle_upload_chunk(
                request, backend, repo_from_path, upload_uuid, account_id
            )
        if method == "PUT":
            return await _handle_complete_upload(
                request, backend, repo_from_path, upload_uuid, account_id
            )
        if method == "GET":
            return await _handle_get_upload_status(
                request, backend, repo_from_path, upload_uuid, account_id
            )
        if method == "DELETE":
            return await _handle_cancel_upload(
                request, backend, repo_from_path, upload_uuid, account_id
            )
        return _make_error_response("UNSUPPORTED", "method not allowed", 405)

    # Blob operations: /v2/{name}/blobs/{digest}
    # Exclude 'uploads' path which is handled above
    blob_match = re.match(r"^/v2/(.+)/blobs/(?!uploads/?$)(.+)$", path)
    if blob_match:
        repo_from_path = blob_match.group(1)
        digest = blob_match.group(2)
        from urllib.parse import unquote

        digest = unquote(digest)

        if method == "GET":
            return await _handle_get_blob(request, backend, repo_from_path, digest, account_id)
        if method == "HEAD":
            return await _handle_head_blob(request, backend, repo_from_path, digest, account_id)
        if method == "DELETE":
            return await _handle_delete_blob(request, backend, repo_from_path, digest, account_id)
        return _make_error_response("UNSUPPORTED", "method not allowed", 405)

    # Tags list: /v2/{name}/tags/list
    tags_match = re.match(r"^/v2/(.+)/tags/list/?$", path)
    if tags_match and method == "GET":
        repo_from_path = tags_match.group(1)
        return await _handle_list_tags(request, backend, repo_from_path, account_id)

    return _make_error_response("UNSUPPORTED", "unsupported operation", 404)


async def _handle_v2_ping(request: Request) -> Response:
    """Handle GET /v2/ - Registry availability check."""
    return Response(
        status_code=200,
        headers={
            "Content-Type": "application/json",
            "Docker-Distribution-API-Version": "registry/2.0",
        },
    )


async def _handle_get_manifest(
    request: Request, backend: Any, repo_name: str, reference: str, account_id: str
) -> Response:
    """Handle GET /v2/{name}/manifests/{reference}."""
    repository = _get_repository(backend, repo_name, account_id)
    if not repository:
        return _make_error_response("NAME_UNKNOWN", f"repository {repo_name} not found", 404)

    image = _get_image_by_reference(repository, reference)
    if not image:
        return _make_error_response("MANIFEST_UNKNOWN", f"manifest for {reference} not found", 404)

    # Update scan status
    _check_image_scan_status(repository, image)

    manifest_bytes = image.image_manifest.encode("utf-8")
    digest = image.get_image_digest()

    # Determine media type
    media_type = image.image_manifest_media_type or DOCKER_MANIFEST_MEDIA_TYPE

    return Response(
        content=manifest_bytes,
        status_code=200,
        headers={
            "Content-Type": media_type,
            "Docker-Content-Digest": digest,
            "Content-Length": str(len(manifest_bytes)),
            "Docker-Distribution-API-Version": "registry/2.0",
        },
    )


async def _handle_put_manifest(
    request: Request, backend: Any, repo_name: str, reference: str, account_id: str
) -> Response:
    """Handle PUT /v2/{name}/manifests/{reference}.

    Pushes a manifest to the registry. If reference is a tag and the repository
    has imageTagMutability=IMMUTABLE, rejects pushes to existing tags.
    """
    repository = _get_repository(backend, repo_name, account_id)
    if not repository:
        return _make_error_response("NAME_UNKNOWN", f"repository {repo_name} not found", 404)

    body = await request.body()
    if not body:
        return _make_error_response("MANIFEST_INVALID", "empty manifest", 400)

    # Parse manifest to validate JSON
    try:
        manifest_json = json.loads(body)
    except json.JSONDecodeError as e:
        return _make_error_response("MANIFEST_INVALID", f"invalid JSON: {e}", 400)

    # Compute digest
    digest = _compute_digest(body)

    # Determine media type from request header or manifest
    content_type = request.headers.get("content-type", "")
    if content_type and content_type != "application/octet-stream":
        media_type = content_type
    else:
        media_type = manifest_json.get("mediaType", DOCKER_MANIFEST_MEDIA_TYPE)

    # Check if this is a tag reference (not a digest)
    is_tag = not reference.startswith("sha256:")

    if is_tag:
        # Check for existing image with this tag
        existing_image = _get_image_by_reference(repository, reference)
        if existing_image:
            # Check immutability
            if repository.is_tag_immutable():
                return _make_error_response(
                    "TAG_IMMUTABLE",
                    f"tag {reference} is immutable",
                    400,
                    error_code="ImageAlreadyExistsException",
                )

            # Remove tag from existing image (will be replaced)
            existing_image.remove_tag(reference)

        # Check for existing image with same digest (add tag to it)
        existing_by_digest = _get_image_by_reference(repository, digest)
        if existing_by_digest:
            # Just add the tag to the existing image
            if reference not in existing_by_digest.image_tags:
                existing_by_digest.update_tag(reference)
            return Response(
                status_code=201,
                headers={
                    "Docker-Content-Digest": digest,
                    "Location": f"/v2/{repo_name}/manifests/{digest}",
                    "Docker-Distribution-API-Version": "registry/2.0",
                },
            )

    # Create new image via ECR's put_image
    try:
        image = backend.put_image(
            repository_name=repo_name,
            image_manifest=body.decode("utf-8"),
            image_tag=reference if is_tag else None,
            image_manifest_mediatype=media_type,
            digest=digest,
        )

        # Trigger scan if scanOnPush is enabled
        _check_image_scan_status(repository, image)

        return Response(
            status_code=201,
            headers={
                "Docker-Content-Digest": digest,
                "Location": f"/v2/{repo_name}/manifests/{digest}",
                "Docker-Distribution-API-Version": "registry/2.0",
            },
        )
    except Exception as e:
        logger.exception("Failed to put image")
        if "ImageAlreadyExistsException" in str(e):
            return _make_error_response(
                "TAG_IMMUTABLE",
                f"image already exists: {e}",
                400,
                error_code="ImageAlreadyExistsException",
            )
        return _make_error_response("MANIFEST_INVALID", str(e), 400)


async def _handle_delete_manifest(
    request: Request, backend: Any, repo_name: str, reference: str, account_id: str
) -> Response:
    """Handle DELETE /v2/{name}/manifests/{reference}."""
    repository = _get_repository(backend, repo_name, account_id)
    if not repository:
        return _make_error_response("NAME_UNKNOWN", f"repository {repo_name} not found", 404)

    image = _get_image_by_reference(repository, reference)
    if not image:
        return _make_error_response("MANIFEST_UNKNOWN", f"manifest for {reference} not found", 404)

    # Delete via batch_delete_image
    image_id = {"imageDigest": image.get_image_digest()}
    if reference in image.image_tags:
        image_id["imageTag"] = reference

    try:
        backend.batch_delete_image(
            repository_name=repo_name,
            registry_id=account_id,
            image_ids=[image_id],
        )
        return Response(
            status_code=202,
            headers={"Docker-Distribution-API-Version": "registry/2.0"},
        )
    except Exception as e:
        logger.exception("Failed to delete manifest")
        return _make_error_response("MANIFEST_INVALID", str(e), 400)


async def _handle_get_blob(
    request: Request, backend: Any, repo_name: str, digest: str, account_id: str
) -> Response:
    """Handle GET /v2/{name}/blobs/{digest}."""
    repository = _get_repository(backend, repo_name, account_id)
    if not repository:
        return _make_error_response("NAME_UNKNOWN", f"repository {repo_name} not found", 404)

    # Check if blob exists in the blob store
    global _blob_store
    blob_content = _blob_store.get(digest)
    if blob_content is not None:
        return Response(
            content=blob_content,
            status_code=200,
            headers={
                "Content-Type": "application/octet-stream",
                "Docker-Content-Digest": digest,
                "Content-Length": str(len(blob_content)),
                "Docker-Distribution-API-Version": "registry/2.0",
            },
        )

    # Also check if blob is referenced as a layer in any manifest
    for image in repository.images:
        try:
            manifest = json.loads(image.image_manifest)
            layers = manifest.get("layers", [])
            for layer in layers:
                if layer.get("digest") == digest:
                    # Return empty content for layers (we don't have the actual layer data)
                    # In a real implementation, this would return the actual layer content
                    return Response(
                        content=b"",
                        status_code=200,
                        headers={
                            "Content-Type": layer.get("mediaType", "application/octet-stream"),
                            "Docker-Content-Digest": digest,
                            "Content-Length": str(layer.get("size", 0)),
                            "Docker-Distribution-API-Version": "registry/2.0",
                        },
                    )
        except json.JSONDecodeError:
            continue

    return _make_error_response("BLOB_UNKNOWN", f"blob {digest} not found", 404)


async def _handle_head_blob(
    request: Request, backend: Any, repo_name: str, digest: str, account_id: str
) -> Response:
    """Handle HEAD /v2/{name}/blobs/{digest}."""
    repository = _get_repository(backend, repo_name, account_id)
    if not repository:
        return _make_error_response("NAME_UNKNOWN", f"repository {repo_name} not found", 404)

    # Check if blob exists in the blob store
    global _blob_store
    blob_content = _blob_store.get(digest)
    if blob_content is not None:
        return Response(
            status_code=200,
            headers={
                "Content-Type": "application/octet-stream",
                "Docker-Content-Digest": digest,
                "Content-Length": str(len(blob_content)),
                "Docker-Distribution-API-Version": "registry/2.0",
            },
        )

    # Check if any image has this blob as a layer
    for image in repository.images:
        try:
            manifest = json.loads(image.image_manifest)
            layers = manifest.get("layers", [])
            for layer in layers:
                if layer.get("digest") == digest:
                    return Response(
                        status_code=200,
                        headers={
                            "Content-Type": layer.get("mediaType", "application/octet-stream"),
                            "Docker-Content-Digest": digest,
                            "Content-Length": str(layer.get("size", 0)),
                            "Docker-Distribution-API-Version": "registry/2.0",
                        },
                    )
        except json.JSONDecodeError:
            continue

    return _make_error_response("BLOB_UNKNOWN", f"blob {digest} not found", 404)


async def _handle_delete_blob(
    request: Request, backend: Any, repo_name: str, digest: str, account_id: str
) -> Response:
    """Handle DELETE /v2/{name}/blobs/{digest}."""
    # ECR doesn't support deleting individual blobs
    return _make_error_response("UNSUPPORTED", "blob deletion not supported", 405)


async def _handle_start_upload(
    request: Request, backend: Any, repo_name: str, account_id: str
) -> Response:
    """Handle POST /v2/{name}/blobs/uploads/ - Start blob upload.

    Returns upload URL with UUID for subsequent PATCH/PUT operations.
    """
    repository = _get_repository(backend, repo_name, account_id)
    if not repository:
        return _make_error_response("NAME_UNKNOWN", f"repository {repo_name} not found", 404)

    # Check for mount parameter (cross-repository blob mount)
    mount = request.query_params.get("mount")
    from_repo = request.query_params.get("from")

    if mount and from_repo:
        # Try to mount existing blob from another repository
        source_repo = _get_repository(backend, from_repo, account_id)
        if source_repo:
            # Check if blob exists in source
            for image in source_repo.images:
                try:
                    manifest = json.loads(image.image_manifest)
                    for layer in manifest.get("layers", []):
                        if layer.get("digest") == mount:
                            # Blob exists, mount it
                            return Response(
                                status_code=201,
                                headers={
                                    "Docker-Content-Digest": mount,
                                    "Location": f"/v2/{repo_name}/blobs/{mount}",
                                    "Docker-Distribution-API-Version": "registry/2.0",
                                },
                            )
                except json.JSONDecodeError:
                    continue

    # Create new upload session
    upload_id = str(uuid.uuid4())
    session_key = f"{account_id}:{repo_name}:{upload_id}"

    _upload_sessions[session_key] = {
        "uuid": upload_id,
        "repository": repo_name,
        "account_id": account_id,
        "started_at": datetime.now(UTC).isoformat(),
        "chunks": [],
    }

    return Response(
        status_code=202,
        headers={
            "Location": f"/v2/{repo_name}/blobs/uploads/{upload_id}",
            "Docker-Upload-UUID": upload_id,
            "Range": "0-0",
            "Docker-Distribution-API-Version": "registry/2.0",
        },
    )


async def _handle_upload_chunk(
    request: Request, backend: Any, repo_name: str, upload_uuid: str, account_id: str
) -> Response:
    """Handle PATCH /v2/{name}/blobs/uploads/{uuid} - Upload chunk."""
    session_key = f"{account_id}:{repo_name}:{upload_uuid}"
    session = _upload_sessions.get(session_key)

    if not session:
        return _make_error_response("BLOB_UPLOAD_UNKNOWN", "upload session not found", 404)

    body = await request.body()
    if body:
        session["chunks"].append(body)

    # Calculate current offset
    current_size = sum(len(chunk) for chunk in session["chunks"])

    return Response(
        status_code=202,
        headers={
            "Location": f"/v2/{repo_name}/blobs/uploads/{upload_uuid}",
            "Docker-Upload-UUID": upload_uuid,
            "Range": f"0-{current_size - 1}" if current_size > 0 else "0-0",
            "Docker-Distribution-API-Version": "registry/2.0",
        },
    )


async def _handle_complete_upload(
    request: Request, backend: Any, repo_name: str, upload_uuid: str, account_id: str
) -> Response:
    """Handle PUT /v2/{name}/blobs/uploads/{uuid} - Complete upload.

    Combines all chunks and verifies digest.
    """
    session_key = f"{account_id}:{repo_name}:{upload_uuid}"
    session = _upload_sessions.get(session_key)

    if not session:
        return _make_error_response("BLOB_UPLOAD_UNKNOWN", "upload session not found", 404)

    # Get final chunk from request body
    body = await request.body()

    # Get digest from query param
    digest = request.query_params.get("digest")
    if not digest:
        return _make_error_response("DIGEST_INVALID", "digest query parameter required", 400)

    # Combine all chunks
    all_chunks = b"".join(session["chunks"])
    if body:
        all_chunks += body

    # Verify digest
    computed_digest = _compute_digest(all_chunks)
    if computed_digest != digest:
        return _make_error_response(
            "DIGEST_INVALID",
            f"digest mismatch: expected {digest}, got {computed_digest}",
            400,
        )

    # Store blob in memory
    global _blob_store
    _blob_store[digest] = all_chunks

    # Clean up session
    del _upload_sessions[session_key]

    return Response(
        status_code=201,
        headers={
            "Docker-Content-Digest": digest,
            "Location": f"/v2/{repo_name}/blobs/{digest}",
            "Docker-Distribution-API-Version": "registry/2.0",
        },
    )


async def _handle_get_upload_status(
    request: Request, backend: Any, repo_name: str, upload_uuid: str, account_id: str
) -> Response:
    """Handle GET /v2/{name}/blobs/uploads/{uuid} - Get upload status."""
    session_key = f"{account_id}:{repo_name}:{upload_uuid}"
    session = _upload_sessions.get(session_key)

    if not session:
        return _make_error_response("BLOB_UPLOAD_UNKNOWN", "upload session not found", 404)

    current_size = sum(len(chunk) for chunk in session["chunks"])

    return Response(
        status_code=204,
        headers={
            "Docker-Upload-UUID": upload_uuid,
            "Range": f"0-{current_size - 1}" if current_size > 0 else "0-0",
            "Docker-Distribution-API-Version": "registry/2.0",
        },
    )


async def _handle_cancel_upload(
    request: Request, backend: Any, repo_name: str, upload_uuid: str, account_id: str
) -> Response:
    """Handle DELETE /v2/{name}/blobs/uploads/{uuid} - Cancel upload."""
    session_key = f"{account_id}:{repo_name}:{upload_uuid}"

    if session_key not in _upload_sessions:
        return _make_error_response("BLOB_UPLOAD_UNKNOWN", "upload session not found", 404)

    del _upload_sessions[session_key]

    return Response(
        status_code=204,
        headers={"Docker-Distribution-API-Version": "registry/2.0"},
    )


async def _handle_list_tags(
    request: Request, backend: Any, repo_name: str, account_id: str
) -> Response:
    """Handle GET /v2/{name}/tags/list."""
    repository = _get_repository(backend, repo_name, account_id)
    if not repository:
        return _make_error_response("NAME_UNKNOWN", f"repository {repo_name} not found", 404)

    # Collect all tags from all images
    tags = []
    for image in repository.images:
        tags.extend(image.image_tags)

    # Remove duplicates and sort
    tags = sorted(set(tags))

    # Handle pagination (n query param)
    n = request.query_params.get("n")
    if n:
        try:
            n = int(n)
            tags = tags[:n]
        except ValueError:
            logger.debug("Invalid n parameter for tag list: %s", n)

    response_body = json.dumps(
        {
            "name": repo_name,
            "tags": tags,
        }
    )

    return Response(
        content=response_body,
        status_code=200,
        headers={
            "Content-Type": "application/json",
            "Docker-Distribution-API-Version": "registry/2.0",
        },
    )


def _make_error_response(
    code: str,
    message: str,
    status: int,
    headers: dict[str, str] | None = None,
    error_code: str | None = None,
) -> Response:
    """Create a registry error response."""
    body = json.dumps(
        {
            "errors": [{"code": code, "message": message}],
        }
    )

    all_headers = {
        "Content-Type": "application/json",
        "Docker-Distribution-API-Version": "registry/2.0",
    }
    if headers:
        all_headers.update(headers)

    # For AWS-compatible errors
    if error_code:
        all_headers["x-amzn-errortype"] = error_code

    return Response(content=body, status_code=status, headers=all_headers)
