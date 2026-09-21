#!/usr/bin/env bash
# Post-deploy smoke test: hits <url>/health on a freshly deployed Worker and
# fails the CI job if it doesn't respond with a 2xx status.
#
# Usage: tools/smoke.sh <deployed-base-url>
set -euo pipefail

if [ $# -lt 1 ] || [ -z "${1:-}" ]; then
  echo "usage: tools/smoke.sh <deployed-base-url>" >&2
  exit 1
fi

base_url="${1%/}"
health_url="${base_url}/health"

status="$(curl -sS -o /dev/null -w '%{http_code}' "$health_url")"

if [ "$status" -lt 200 ] || [ "$status" -ge 300 ]; then
  echo "smoke test failed: GET ${health_url} returned HTTP ${status} (expected 2xx)" >&2
  exit 1
fi

echo "smoke test passed: GET ${health_url} returned HTTP ${status}"
