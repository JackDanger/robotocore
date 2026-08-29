#!/usr/bin/env bash
# Start the Rust server + moto sidecar as daemons for parity testing.
set -euo pipefail

PYV=/Users/jackdanger/www/robotocore/.venv/bin/python
RUST_REPO=/Users/jackdanger/www/robotocore-rust
SIDECAR_PORT=4568
RUST_PORT=4567

# Kill existing
pkill -9 -f moto_sidecar 2>/dev/null || true
pkill -f robotocore-rust 2>/dev/null || true
sleep 2

# Start sidecar (detached)
nohup $PYV $RUST_REPO/scripts/moto_sidecar.py --port $SIDECAR_PORT \
    > /tmp/parity_sidecar.log 2>&1 &
SIDECAR_PID=$!
echo "Sidecar PID: $SIDECAR_PID"

# Wait for sidecar
for i in $(seq 1 20); do
    if curl -sf --max-time 3 http://127.0.0.1:$SIDECAR_PORT/_sidecar/health > /dev/null 2>&1; then
        echo "Sidecar up"
        break
    fi
    sleep 1
done

# Build Rust if needed
cd $RUST_REPO
cargo build --release 2>&1 | tail -1

# Start Rust server (detached)
nohup $RUST_REPO/target/release/robotocore-rust \
    --port $RUST_PORT \
    --moto-url http://127.0.0.1:$SIDECAR_PORT \
    > /tmp/parity_rust.log 2>&1 &
RUST_PID=$!
echo "Rust PID: $RUST_PID"

# Wait for Rust
for i in $(seq 1 20); do
    if curl -sf --max-time 3 http://127.0.0.1:$RUST_PORT/_robotocore/health > /dev/null 2>&1; then
        echo "Rust up"
        break
    fi
    sleep 1
done

# Verify bridge
for i in $(seq 1 30); do
    RESP=$(curl -s -X POST http://127.0.0.1:$RUST_PORT/ --max-time 10 \
        -H 'Content-Type: application/x-www-form-urlencoded' \
        -H 'Authorization: AWS4-HMAC-SHA256 Credential=123456789012/20240101/us-east-1/ec2/aws4_request' \
        -H 'X-Amz-Date: 20240101T000000Z' \
        -d 'Action=DescribeInstances&Version=2016-11-15' 2>/dev/null || echo "")
    if echo "$RESP" | grep -q "DescribeInstancesResponse"; then
        echo "Bridge WORKING"
        exit 0
    fi
    sleep 2
done
echo "Bridge NOT working"
exit 1
