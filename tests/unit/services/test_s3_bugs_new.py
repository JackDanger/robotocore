"""Tests for S3 provider bugs found during review.

Bug 1: Notification configuration Id element is silently ignored.
When a user PUTs a notification configuration with an <Id> element,
the Id is parsed but not stored, and is missing from the GET response.
"""

import pytest

from robotocore.services.s3.notifications import NotificationConfig
from robotocore.services.s3.provider import (
    _notification_config_to_xml,
    _parse_notification_config_xml,
)


class TestNotificationConfigId:
    """Bug 1: Notification configuration Id element is silently ignored."""

    def test_queue_configuration_id_is_preserved(self):
        """PUT with <Id> should return the same Id on GET."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<NotificationConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <QueueConfiguration>
    <Id>my-queue-config-id</Id>
    <Queue>arn:aws:sqs:us-east-1:123456789012:my-queue</Queue>
    <Event>s3:ObjectCreated:Put</Event>
  </QueueConfiguration>
</NotificationConfiguration>"""
        config = _parse_notification_config_xml(xml)
        # The Id should be stored in the config
        assert "Id" in config.queue_configs[0], "Id element should be parsed and stored"
        assert config.queue_configs[0]["Id"] == "my-queue-config-id"

    def test_topic_configuration_id_is_preserved(self):
        """PUT with <Id> should return the same Id on GET."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<NotificationConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <TopicConfiguration>
    <Id>my-topic-config-id</Id>
    <Topic>arn:aws:sns:us-east-1:123456789012:my-topic</Topic>
    <Event>s3:ObjectCreated:Put</Event>
  </TopicConfiguration>
</NotificationConfiguration>"""
        config = _parse_notification_config_xml(xml)
        assert "Id" in config.topic_configs[0]
        assert config.topic_configs[0]["Id"] == "my-topic-config-id"

    def test_lambda_configuration_id_is_preserved(self):
        """PUT with <Id> should return the same Id on GET."""
        xml = """<?xml version="1.0" encoding="UTF-8"?>
<NotificationConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <LambdaFunctionConfiguration>
    <Id>my-lambda-config-id</Id>
    <LambdaFunctionArn>arn:aws:lambda:us-east-1:123456789012:function:my-func</LambdaFunctionArn>
    <Event>s3:ObjectCreated:Put</Event>
  </LambdaFunctionConfiguration>
</NotificationConfiguration>"""
        config = _parse_notification_config_xml(xml)
        assert "Id" in config.lambda_configs[0]
        assert config.lambda_configs[0]["Id"] == "my-lambda-config-id"

    def test_id_is_included_in_xml_output(self):
        """The Id should be included when serializing to XML."""
        config = NotificationConfig()
        config.queue_configs.append({
            "QueueArn": "arn:aws:sqs:us-east-1:123456789012:my-queue",
            "Events": ["s3:ObjectCreated:Put"],
            "Id": "my-config-id",
        })
        xml = _notification_config_to_xml(config)
        assert "<Id>my-config-id</Id>" in xml
