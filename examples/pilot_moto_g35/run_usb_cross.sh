#!/usr/bin/env bash
# G35 — USB probe + cruzamento com platform_vendor_boot → bring-up checklist.
# ≠ OS turnkey · generates_os=false
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT_USB="${USB_PROBE_OUT:-$PILOT/out_real/usb_probe}"
OUT_CROSS="${USB_CROSS_OUT:-$PILOT/out_real/usb_cross}"
PLAT="${PLATFORM_YAML:-$PILOT/out_real/platform_vendor_boot/platform_inventory.yaml}"

BASE_BIN="${BASE_BIN:-$ROOT/target/release/base}"
if [[ ! -x "$BASE_BIN" ]]; then
  BASE_BIN="$ROOT/target/debug/base"
fi
if [[ ! -x "$BASE_BIN" ]]; then
  (cd "$ROOT" && cargo build -p base-cli --bin base -q)
  BASE_BIN="$ROOT/target/debug/base"
fi

cd "$ROOT"
ARGS=()
[[ -n "${ADB_SERIAL:-}" ]] && ARGS+=(--serial "$ADB_SERIAL")

"$BASE_BIN" port usb-probe "${ARGS[@]}" -o "$OUT_USB"

if [[ ! -f "$PLAT" ]]; then
  echo "WARN: missing $PLAT — run run_real_fw.sh first for DTB inventory"
  exit 0
fi

"$BASE_BIN" port usb-cross \
  --usb "$OUT_USB/usb_hw_inventory.yaml" \
  --platform "$PLAT" \
  -o "$OUT_CROSS"

echo "Bring-up → $OUT_CROSS/BRINGUP_CHECKLIST.md"
echo "generates_os=false"
