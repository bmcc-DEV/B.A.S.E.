#!/usr/bin/env bash
# HIL Lab Gate A smoke — software checks (≠ production)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
OUT="$ROOT/examples/hil_lab/out"
rm -rf "$OUT"
mkdir -p "$OUT"

cargo build -p base-cli -q
BASE="$ROOT/target/debug/base"
SOP="$ROOT/examples/hil_lab/SOP.md"

echo "== Gate A lab-status (expect not lab-ready without Detected/programmer) =="
"$BASE" hil lab-status --sop "$SOP" -o "$OUT"
test -f "$OUT/hil_lab_gate.json"
python3 - <<'PY' "$OUT/hil_lab_gate.json"
import json, sys
r = json.load(open(sys.argv[1]))
assert r.get("production") is False
assert r.get("claim") == "A"
checks = {c["id"]: c for c in r["checks"]}
assert checks["A3"]["green"] is True, "SOP must satisfy A3"
assert checks["A4"]["green"] is True
assert checks["A1"]["green"] is False, "CI default must keep A1 blocked"
assert checks["A2"]["green"] is False, "CI default must keep A2 blocked"
assert r["lab_assist_ready"] is False, "CI default must not be lab-ready without Detected+programmer"
print("lab_gate OK production=false A3/A4 green A1/A2 blocked lab_assist_ready=false")
PY

echo "== mock flash still ≠ production =="
"$BASE" hil flash /dev/null --mock-flash -o "$OUT"
python3 - <<'PY' "$OUT/hil_flash_receipt.json"
import json, sys
r = json.load(open(sys.argv[1]))
assert r.get("production") is False
assert r.get("mode") == "mock_dry_run"
print("mock flash OK")
PY

cat > "$OUT/CASE_SUMMARY_HIL_LAB.md" <<EOF
# HIL Lab Gate A CASE SUMMARY

- lab-status report: hil_lab_gate.json
- SOP: examples/hil_lab/SOP.md (A3)
- production: false
- status: OK (software gate default)
EOF

echo "HIL lab smoke OK → $OUT"

# Phase 2: close A1/A2 in controlled lab rehearsal (still ≠ production / A5)
chmod +x "$ROOT/examples/hil_lab/run_hil_lab_assist.sh"
"$ROOT/examples/hil_lab/run_hil_lab_assist.sh"
