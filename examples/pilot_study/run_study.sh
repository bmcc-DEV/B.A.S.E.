#!/usr/bin/env bash
# Specter VM study smoke (Path to v1.1) — ≠ auto-fix
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
OUT="$ROOT/examples/pilot_study/out"
PILOT="$ROOT/examples/pilot"
rm -rf "$OUT"
mkdir -p "$OUT"

cargo build -p base-cli -q
BASE="$ROOT/target/debug/base"

echo "== analyze seed (RP UART) =="
"$BASE" analyze "$PILOT/fw.bin" \
  --mmio-traces "$PILOT/mmio.json" \
  --classify uart \
  -o "$OUT/analyze"

echo "== specter study (Forth + Lua) =="
"$BASE" study "$OUT/analyze/hardware_spec.yaml" \
  --policy "$ROOT/examples/pilot_study/policy.lua" \
  --program "$ROOT/examples/pilot_study/study.base" \
  -o "$OUT/study"

REPORT="$OUT/study/study_report.json"
test -f "$REPORT"
python3 - <<'PY' "$REPORT"
import json, sys
r = json.load(open(sys.argv[1]))
assert r.get("auto_fix_complete") is False, r
assert "stop_reason" in r, r
assert r["stop_reason"] in ("converged", "stagnated", "max_iterations"), r
assert r.get("total_steps", 0) >= 1, r
print("study_report OK:", r["stop_reason"], "steps=", r["total_steps"])
PY

test -f "$OUT/study/hardware_spec_refined.yaml"

echo "== reconstruct parity (REAL*) =="
"$BASE" reconstruct "$OUT/analyze/hardware_spec.yaml" \
  --threshold 0.99 --max-iterations 16 \
  -o "$OUT/reconstruct"
test -f "$OUT/reconstruct/convergence_report.json"
python3 - <<'PY' "$OUT/reconstruct/convergence_report.json"
import json, sys
r = json.load(open(sys.argv[1]))
assert r.get("auto_fix_complete") is False
assert "stop_reason" in r
print("reconstruct OK:", r["stop_reason"])
PY

echo "== evolve metrics (REAL*) =="
"$BASE" synth "$OUT/analyze/hardware_spec.yaml" -o "$OUT/synth"
"$BASE" evolve "$OUT/synth/synthesized_spec.yaml" --format yaml -o "$OUT/evolve"
test -f "$OUT/evolve/evolution_plan.yaml"
# golden: plan must mention Evidence or Evolution title
grep -E "Evidence gap|Evolution:|Classification:" "$OUT/evolve/evolution_plan.yaml" \
  || grep -q "title:" "$OUT/evolve/evolution_plan.yaml"

echo "== hil host (REAL* host / not production) =="
"$BASE" hil enumerate -o "$OUT/hil"
"$BASE" hil flash /dev/null --mock-flash -o "$OUT/hil"
test -f "$OUT/hil/hil_enumerate.json"
python3 - <<'PY' "$OUT/hil/hil_enumerate.json"
import json, sys
r = json.load(open(sys.argv[1]))
assert r.get("production") is False
print("hil enumerate OK production=false")
PY

echo "== pcb draft banner (REAL* draft) =="
"$BASE" design "$OUT/analyze/hardware_spec.yaml" --pcb -o "$OUT/design"
SCH="$OUT/design/pcb/reference.kicad_sch"
test -f "$SCH"
grep -q "NOT FABRICABLE" "$SCH"
# golden marker check
diff -q <(grep -o "NOT FABRICABLE" "$SCH" | head -1) \
  "$ROOT/examples/pilot/expected_pcb/NOT_FABRICABLE.marker"

cat > "$OUT/CASE_SUMMARY_STUDY.md" <<EOF
# Specter study CASE SUMMARY

- Policy: examples/pilot_study/policy.lua
- Program: examples/pilot_study/study.base
- stop_reason + auto_fix_complete=false verified
- reconstruct / evolve / hil host / pcb draft REAL* checks
- status: OK
EOF

echo "Pilot study smoke OK → $OUT"
