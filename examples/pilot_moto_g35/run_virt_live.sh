#!/usr/bin/env bash
# Moto G35 — Specter Live (opt-in). NDJSON → Ψ; QEMU opcional.
# ≠ OS turnkey · ≠ HIL production
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$PILOT/out_virt"
mkdir -p "$OUT"

BASE_BIN="${BASE_BIN:-$ROOT/target/release/base}"
if [[ ! -x "$BASE_BIN" ]]; then
  BASE_BIN="$ROOT/target/debug/base"
fi
if [[ ! -x "$BASE_BIN" ]]; then
  echo "Building base…"
  (cd "$ROOT" && cargo build -p base-cli --bin base)
  BASE_BIN="$ROOT/target/debug/base"
fi

TRACE="${VIRT_TRACE:-$PILOT/virt/sample_mmio.ndjson}"
SPEC="${VIRT_SPEC:-}"
if [[ -z "$SPEC" ]]; then
  for c in \
    "$PILOT/out_real/analyze_boot/hardware_spec.yaml" \
    "$PILOT/out/analyze/hardware_spec.yaml" \
    "$PILOT/virt/hardware_spec_uart_stub.yaml" \
    "$ROOT/examples/pilot/out/analyze/hardware_spec.yaml"
  do
    if [[ -f "$c" ]]; then SPEC="$c"; break; fi
  done
fi
if [[ -z "$SPEC" || ! -f "$SPEC" ]]; then
  echo "ERROR: set VIRT_SPEC=path/to/hardware_spec.yaml"
  exit 1
fi

"$BASE_BIN" virt ingest "$TRACE" -o "$OUT/ingest"
"$BASE_BIN" virt score --spec "$SPEC" --evidence "$OUT/ingest/evidence_db.yaml" \
  --window-size 4 --max-windows 16 -o "$OUT/score"

KERNEL="${HIL_FW_IMAGE:-}"
NO_QEMU=()
PLUGIN_ARGS=()
PLUGIN_SO="${VIRT_PLUGIN:-$ROOT/base-virt/plugin/libbase_virt_ndjson.so}"
if [[ -f "$PLUGIN_SO" ]]; then
  PLUGIN_ARGS=(--plugin "$PLUGIN_SO" --qmp --probe-qmp --plugin-arg io_only=0)
  echo "NOTE: using TCG plugin $PLUGIN_SO + QMP probe (io_only=0 for stub kernel)"
elif [[ "${VIRT_BUILD_PLUGIN:-0}" == "1" ]]; then
  make -C "$ROOT/base-virt/plugin"
  PLUGIN_ARGS=(--plugin "$PLUGIN_SO" --qmp --probe-qmp --plugin-arg io_only=0)
fi

if [[ -z "$KERNEL" ]]; then
  if [[ -f "$PILOT/kernel.bin" ]]; then
    KERNEL="$PILOT/kernel.bin"
  else
    NO_QEMU=(--no-qemu)
    PLUGIN_ARGS=()
    echo "NOTE: no kernel — trace-only Specter Live (set HIL_FW_IMAGE for QEMU)"
  fi
fi

if [[ ${#NO_QEMU[@]} -eq 0 ]]; then
  "$BASE_BIN" virt run --spec "$SPEC" --trace "$TRACE" --kernel "$KERNEL" \
    --timeout-sec "${QEMU_TIMEOUT_SEC:-8}" \
    --window-size 4 --max-windows 16 \
    "${PLUGIN_ARGS[@]}" \
    -o "$OUT/run"
else
  "$BASE_BIN" virt run --spec "$SPEC" --trace "$TRACE" --no-qemu \
    --window-size 4 --max-windows 16 \
    -o "$OUT/run"
fi

cp -f "$OUT/run/CASE_SUMMARY_VIRT.md" "$OUT/CASE_SUMMARY_G35_VIRT.md" 2>/dev/null || true
echo "Specter Live OK → $OUT (generates_os=false)"
