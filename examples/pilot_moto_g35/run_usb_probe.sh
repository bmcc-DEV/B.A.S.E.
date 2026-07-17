#!/usr/bin/env bash
# Moto G35 — USB live HW probe (ADB / fastboot / lsusb). Read-only. ≠ OS turnkey.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="${USB_PROBE_OUT:-$PILOT/out_real/usb_probe}"
mkdir -p "$OUT"

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
ARGS=()
if [[ -n "${ADB_SERIAL:-}" ]]; then
  ARGS+=(--serial "$ADB_SERIAL")
fi

"$BASE_BIN" port usb-probe "${ARGS[@]}" -o "$OUT"
echo "USB probe → $OUT (cruzar com out_real/platform_vendor_boot/)"
echo "generates_os=false"
