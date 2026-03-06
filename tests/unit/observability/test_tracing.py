"""Tests for request tracing."""

from robotocore.observability.tracing import generate_request_id


class TestGenerateRequestId:
    def test_returns_string(self):
        rid = generate_request_id()
        assert isinstance(rid, str)

    def test_unique_ids(self):
        ids = {generate_request_id() for _ in range(100)}
        assert len(ids) == 100

    def test_uuid_format(self):
        rid = generate_request_id()
        parts = rid.split("-")
        assert len(parts) == 5
