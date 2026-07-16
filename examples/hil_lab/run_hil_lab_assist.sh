#!/usr/bin/env bash
# HIL Lab Gate A — fechar A1/A2 (Detected + programmer) em lab rehearsal.
# ≠ production · ≠ USB real obrigatório (usa --mock-detected) · A5 continua contrato.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
OUT="$ROOT/examples/hil_lab/out_assist"
rm -rf "$OUT"
mkdir -p "$OUT"

echo "== Build CLI with hil_programmer (A2 feature) =="
cargo build -p base-cli --features hil_programmer -q
BASE="$ROOT/target/debug/base"
SOP="$ROOT/examples/hil_lab/SOP.md"
FW="$OUT/lab_fw.bin"
printf 'BASE-HIL-LAB' > "$FW"

echo "== A1+A2 lab-status (mock Detected + ALLOW_FLASH + CMD; A5 still open) =="
export BASE_HIL_ALLOW_FLASH=1
export BASE_HIL_PROGRAMMER_CMD='test -f {image}'
"$BASE" hil lab-status --sop "$SOP" --mock-detected -o "$OUT"
test -f "$OUT/hil_lab_gate.json"
python3 - <<'PY' "$OUT/hil_lab_gate.json"
import json, sys
r = json.load(open(sys.argv[1]))
assert r.get("production") is False
checks = {c["id"]: c for c in r["checks"]}
assert checks["A1"]["green"] is True, checks["A1"]
assert checks["A2"]["green"] is True, checks["A2"]
assert checks["A3"]["green"] is True
assert checks["A4"]["green"] is True
assert checks["A5"]["green"] is False, "CI must not claim SOW signed"
assert r["lab_assist_ready"] is False, "A5 open → not full lab-assist"
print("A1/A2 GREEN production=false A5 open lab_assist_ready=false")
PY

echo "== experimental flash under Detected + programmer (≠ production) =="
"$BASE" hil flash "$FW" --mock-detected -o "$OUT"
python3 - <<'PY' "$OUT/hil_flash_receipt.json"
import json, sys
r = json.load(open(sys.argv[1]))
assert r.get("production") is False
assert r.get("mode") == "experimental_external_cmd", r
assert r.get("mode") != "production"
print("experimental_external_cmd OK")
PY

# unset so parent shells / follow-on CI steps stay clean if sourced (we don't source)
unset BASE_HIL_ALLOW_FLASH BASE_HIL_PROGRAMMER_CMD

cat > "$OUT/CASE_SUMMARY_HIL_LAB_ASSIST.md" <<EOF
# HIL Lab A1/A2 CASE SUMMARY

- A1: Detected via \`--mock-detected\` (rehearsal; USB real = \`hil_usb\` + probe)
- A2: \`--features hil_programmer\` + ALLOW_FLASH + PROGRAMMER_CMD
- A5: not signed in CI
- production: false
- receipt mode: experimental_external_cmd
EOF

echo "HIL lab A1/A2 assist OK → $OUT"
