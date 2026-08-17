#!/usr/bin/env bash
# Smoke Identify API — Saturn synth MMIO
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

KEY="${BASE_API_KEY:-sk-base-dev-local}"
URL="${BASE_API_URL:-http://127.0.0.1:8787}"

if ! curl -sf "$URL/health" >/dev/null; then
  echo "start API first: cargo run -p base-api"
  exit 1
fi

FW=$(printf 'SATURNHW' | base64 -w0 2>/dev/null || printf 'SATURNHW' | base64)
CONTRACTS=$(cat examples/pilot_saturn/contracts.yaml | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')

curl -sS "$URL/v1/identify" \
  -H "Authorization: Bearer $KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"label\": \"saturn_api_smoke\",
    \"firmware_b64\": \"$FW\",
    \"mmio\": $(cat examples/pilot_saturn/mmio.json),
    \"contracts_yaml\": $CONTRACTS
  }" | python3 -m json.tool

echo
curl -sS "$URL/v1/usage" -H "Authorization: Bearer $KEY" | python3 -m json.tool
