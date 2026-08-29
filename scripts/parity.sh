#!/usr/bin/env bash
# Fidelity parity — run full or quick-check.
set -euo pipefail
PYV=/Users/jackdanger/www/robotocore/.venv/bin/python
cd /Users/jackdanger/www/robotocore-rust
case "${1:-full}" in
  full|run) exec "$PYV" scripts/harness/parity.py ;;
  next)    exec "$PYV" scripts/harness/parity.py --next ;;
  *)       exec "$PYV" scripts/harness/parity.py "$@" ;;
esac
