#!/usr/bin/env bash
# Smoke-tests a deployed (or locally running) venture backend.
#
# Usage: ./smoke.sh [BASE_URL]
#   BASE_URL defaults to http://localhost:8787 (wrangler dev's default).
set -euo pipefail

BASE_URL="${1:-http://localhost:8787}"
FAILED=0

check_status() {
    local description="$1"
    local expected="$2"
    local actual="$3"

    if [[ ",${expected}," == *",${actual},"* ]]; then
        echo "PASS: ${description} (got ${actual})"
    else
        echo "FAIL: ${description} (expected one of [${expected}], got ${actual})"
        FAILED=1
    fi
}

echo "Smoke-testing ${BASE_URL}"

health_status="$(curl -s -o /dev/null -w '%{http_code}' "${BASE_URL}/__health")"
check_status "GET /__health" "200" "${health_status}"

ready_status="$(curl -s -o /dev/null -w '%{http_code}' "${BASE_URL}/__ready")"
check_status "GET /__ready" "200" "${ready_status}"

waitlist_status="$(curl -s -o /dev/null -w '%{http_code}' \
    -X POST "${BASE_URL}/v1/waitlist" \
    -H 'Content-Type: application/json' \
    -d '{"email":"smoke-test@example.com","product":"venture","captchaToken":"smoke-test-token"}')"
check_status "POST /v1/waitlist" "202,503" "${waitlist_status}"

if [[ "${FAILED}" -ne 0 ]]; then
    echo "smoke.sh: one or more checks failed"
    exit 1
fi

echo "smoke.sh: all checks passed"
