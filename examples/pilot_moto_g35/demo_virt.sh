#!/usr/bin/env bash
# Demo curto Specter Live / Twin↔guest (piloto G35). ≠ OS turnkey.
# Uso: ./examples/pilot_moto_g35/demo_virt.sh [watch|twin|qmp|all]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET="${1:-all}"

BASE_BIN="${BASE_BIN:-$ROOT/target/release/base}"
if [[ ! -x "$BASE_BIN" ]]; then
  BASE_BIN="$ROOT/target/debug/base"
fi
if [[ ! -x "$BASE_BIN" ]]; then
  echo "Building base…"
  (cd "$ROOT" && cargo build -p base-cli --bin base -q)
  BASE_BIN="$ROOT/target/debug/base"
fi

cd "$ROOT"
exec "$BASE_BIN" virt demo "$TARGET" -o /tmp/base_virt_demo
