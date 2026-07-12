---
type: test
date: 2025-07-12
summary: Integration tests for write-read fidelity bugs
description: |
  Created integration tests for 6 write-read fidelity bugs that were fixed:
  1. EC2 PlacementGroup PartitionCount not returned in DescribePlacementGroups
  2. S3 NotificationConfiguration Id not returned in GetBucketNotificationConfiguration
  3. ECR DescribeRepositories pagination nextToken was hardcoded
  4. Kinesis GetShardIterator AT_TIMESTAMP ignored Timestamp parameter
  5. SecretsManager RotateSecret didn't preserve SecretBinary in AWSPENDING version
  6. DynamoDB Global Table replica creation didn't inherit stream configuration

  Each test class performs a minimal end-to-end round trip using real boto3
  clients and HTTP round-trips to verify the fixes.
