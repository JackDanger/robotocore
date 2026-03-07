"""ACM compatibility tests."""

import uuid

import pytest
from botocore.exceptions import ClientError

from tests.compatibility.conftest import make_client


def _uid():
    return uuid.uuid4().hex[:8]


@pytest.fixture
def acm():
    return make_client("acm")


class TestACMOperations:
    def test_request_certificate(self, acm):
        response = acm.request_certificate(DomainName="example.com")
        assert "CertificateArn" in response

    def test_list_certificates(self, acm):
        acm.request_certificate(DomainName="list-test.example.com")
        response = acm.list_certificates()
        assert len(response["CertificateSummaryList"]) >= 1

    def test_describe_certificate(self, acm):
        arn = acm.request_certificate(DomainName="describe.example.com")["CertificateArn"]
        response = acm.describe_certificate(CertificateArn=arn)
        assert response["Certificate"]["DomainName"] == "describe.example.com"

    def test_describe_certificate_fields(self, acm):
        arn = acm.request_certificate(DomainName="fields.example.com")["CertificateArn"]
        response = acm.describe_certificate(CertificateArn=arn)
        cert = response["Certificate"]
        assert cert["CertificateArn"] == arn
        assert "Status" in cert
        assert "Type" in cert
        assert cert["DomainName"] == "fields.example.com"

    def test_list_tags_for_certificate(self, acm):
        arn = acm.request_certificate(DomainName="tags.example.com")["CertificateArn"]
        acm.add_tags_to_certificate(
            CertificateArn=arn,
            Tags=[
                {"Key": "Environment", "Value": "test"},
                {"Key": "Project", "Value": "robotocore"},
            ],
        )
        response = acm.list_tags_for_certificate(CertificateArn=arn)
        tags = {t["Key"]: t["Value"] for t in response["Tags"]}
        assert tags["Environment"] == "test"
        assert tags["Project"] == "robotocore"

    def test_remove_tags_from_certificate(self, acm):
        arn = acm.request_certificate(DomainName="rmtags.example.com")["CertificateArn"]
        acm.add_tags_to_certificate(
            CertificateArn=arn,
            Tags=[{"Key": "ToRemove", "Value": "yes"}],
        )
        acm.remove_tags_from_certificate(
            CertificateArn=arn,
            Tags=[{"Key": "ToRemove", "Value": "yes"}],
        )
        response = acm.list_tags_for_certificate(CertificateArn=arn)
        keys = [t["Key"] for t in response.get("Tags", [])]
        assert "ToRemove" not in keys

    def test_delete_certificate(self, acm):
        arn = acm.request_certificate(DomainName=f"del-{_uid()}.example.com")["CertificateArn"]
        response = acm.delete_certificate(CertificateArn=arn)
        assert response["ResponseMetadata"]["HTTPStatusCode"] == 200

    def test_request_certificate_with_san(self, acm):
        domain = f"san-{_uid()}.example.com"
        response = acm.request_certificate(
            DomainName=domain,
            SubjectAlternativeNames=[domain, f"*.{domain}"],
        )
        assert "CertificateArn" in response
        arn = response["CertificateArn"]
        cert = acm.describe_certificate(CertificateArn=arn)["Certificate"]
        assert domain in cert.get("SubjectAlternativeNames", [])
        acm.delete_certificate(CertificateArn=arn)

    def test_list_certificates_empty(self, acm):
        response = acm.list_certificates()
        assert "CertificateSummaryList" in response

    def test_request_certificate_with_tags(self, acm):
        domain = f"tagged-{_uid()}.example.com"
        arn = acm.request_certificate(
            DomainName=domain,
            Tags=[{"Key": "env", "Value": "prod"}, {"Key": "team", "Value": "infra"}],
        )["CertificateArn"]
        tags = acm.list_tags_for_certificate(CertificateArn=arn)["Tags"]
        tag_map = {t["Key"]: t["Value"] for t in tags}
        assert tag_map["env"] == "prod"
        assert tag_map["team"] == "infra"
        acm.delete_certificate(CertificateArn=arn)

    def test_describe_nonexistent_certificate(self, acm):
        fake_arn = (
            "arn:aws:acm:us-east-1:123456789012:certificate/"
            "00000000-0000-0000-0000-000000000000"
        )
        with pytest.raises(ClientError) as exc:
            acm.describe_certificate(CertificateArn=fake_arn)
        assert exc.value.response["Error"]["Code"] == "ResourceNotFoundException"
