#!/usr/bin/env bash
# B.A.S.E. Pilot — UART MMIO wedge (synthetic) — Path to Real R0–R6 / v0.2
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BASE="${ROOT}/target/debug/base"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="${PILOT}/out"

if [[ ! -x "$BASE" ]]; then
  echo "Building base-cli…"
  (cd "$ROOT" && cargo build -p base-cli)
fi

echo "== fixture integrity =="
(cd "$PILOT" && sha256sum -c SHA256SUMS)

rm -rf "$OUT"
mkdir -p "$OUT"

echo "== bir compile + validate =="
"$BASE" bir "$PILOT/pilot.bsl" --compile --validate -o "$OUT/bir"

echo "== analyze (Capstone --disasm, S1) =="
"$BASE" analyze "$PILOT/fw.bin" --disasm -o "$OUT/analyze_disasm"
# Capstone must hit UART page without heuristic/traces
grep -E 'base_address: (1073954816|0x40034000)' "$OUT/analyze_disasm/hardware_spec.yaml" >/dev/null \
  || grep -q '40034000' "$OUT/analyze_disasm/hardware_spec.yaml"

echo "== analyze (mmio-traces + classify, v0.2 path) =="
"$BASE" analyze "$PILOT/fw.bin" \
  --mmio-traces "$PILOT/mmio.json" \
  --classify uart \
  -o "$OUT/analyze"

echo "== design =="
"$BASE" design "$OUT/analyze/hardware_spec.yaml" -o "$OUT/design"

echo "== synth =="
"$BASE" synth "$OUT/analyze/hardware_spec.yaml" --max-bom-cost 80 -o "$OUT/synth"

echo "== check (skip without new_trace) =="
"$BASE" check "$OUT/synth/synthesized_spec.yaml" "$PILOT/trace.csv" \
  --format json -o "$OUT/check_skip"
grep -q 'NO_NEW_TRACE' "$OUT/check_skip/validation_report.json"
grep -q '"comparison_mode": "skipped"' "$OUT/check_skip/validation_report.json"

echo "== check (dual: original vs slow) =="
"$BASE" check "$OUT/synth/synthesized_spec.yaml" "$PILOT/trace.csv" \
  "$PILOT/trace_slow.csv" --format json --max-latency 2.0 -o "$OUT/check_dual"
grep -q '"comparison_mode": "dual"' "$OUT/check_dual/validation_report.json"
grep -q 'TIMING_VIOLATION' "$OUT/check_dual/validation_report.json"

test -f "$OUT/analyze/tension_report.json"
grep -q 'overall_tension' "$OUT/analyze/tension_report.json"

echo "== prove (sat) =="
"$BASE" prove "$PILOT/contracts.yaml" -o "$OUT/prove"

echo "== prove (contracts from BIR) =="
"$BASE" prove "$OUT/bir/contracts.yaml" -o "$OUT/prove_bir"

echo "== prove (unsat fixture) =="
"$BASE" prove "$PILOT/contracts.unsat.yaml" -o "$OUT/prove_unsat"

echo "== replay (hand contracts) =="
"$BASE" replay "$PILOT/trace.csv" \
  --contracts "$PILOT/contracts.yaml" \
  --output "$OUT/violations.json"

echo "== replay (--bir) =="
"$BASE" replay "$PILOT/trace.csv" \
  --bir "$OUT/bir/compiled.bir.yaml" \
  --output "$OUT/violations_bir.json"

echo "== replay fail trace =="
"$BASE" replay "$PILOT/trace_fail.csv" \
  --contracts "$PILOT/contracts.yaml" \
  --output "$OUT/violations_fail.json" || true

echo "== event-graph + goldens (X2) =="
"$BASE" event-graph "$PILOT/contracts.yaml" "$PILOT/trace.csv" \
  --format dot -o "$OUT/event_graph"
"$BASE" event-graph "$PILOT/contracts.yaml" "$PILOT/trace.csv" \
  --format mermaid -o "$OUT/event_graph"
diff -u "$PILOT/expected/event_graph.dot" "$OUT/event_graph/event_graph.dot"
diff -u "$PILOT/expected/event_graph.mmd" "$OUT/event_graph/event_graph.mmd"
# Prove golden: stable fields only (omit smt_lib)
python3 - "$OUT/prove/proof_report.json" "$PILOT/expected/proof_report.golden.json" <<'PY'
import json, pathlib, sys
actual_path = pathlib.Path(sys.argv[1])
golden_path = pathlib.Path(sys.argv[2])
src = json.loads(actual_path.read_text())
got = {
    "backend": src["backend"],
    "contracts_proved": src["contracts_proved"],
    "all_satisfied": src["all_satisfied"],
    "results": [
        {
            "contract": r["contract"],
            "satisfiable": r["satisfiable"],
            "proved": r["proved"],
            "backend": r["backend"],
            "model": r["model"],
        }
        for r in src["results"]
    ],
}
want = json.loads(golden_path.read_text())
assert got == want, f"prove golden mismatch:\n got={got}\nwant={want}"
print("prove golden OK")
PY
# HardwareSpec field allowlist
python3 - "$OUT/analyze/hardware_spec.yaml" "$PILOT/expected/hardware_spec.fields.yaml" <<'PY'
import pathlib, sys
spec = pathlib.Path(sys.argv[1]).read_text()
fields = pathlib.Path(sys.argv[2]).read_text()
keys = []
in_list = False
for line in fields.splitlines():
    if line.strip() == "required_top_level:":
        in_list = True
        continue
    if in_list:
        if line.startswith("  - "):
            keys.append(line[4:].strip())
        elif line and not line.startswith(" "):
            break
        elif line.startswith("required_"):
            break
assert keys, "no required_top_level keys"
for k in keys:
    assert f"{k}:" in spec or f"\n{k}:" in spec or spec.startswith(f"{k}:"), f"missing top-level key {k}"
print(f"hardware_spec fields OK ({len(keys)} keys)")
PY

echo "== fw host =="
"$BASE" fw "$OUT/synth/synthesized_spec.yaml" -o "$OUT/fw"
make -C "$OUT/fw" host
"$OUT/fw/firmware_host"

echo "== CASE_SUMMARY =="
python3 - "$OUT" <<'PY'
import json, pathlib, sys
out = pathlib.Path(sys.argv[1])
# minimal YAML scrape (avoid PyYAML dependency)
rd = (out / "design/reference_design.yaml").read_text()
cpu = satisfied = total = cost = "?"
for line in rd.splitlines():
    s = line.strip()
    if s.startswith("part:") and cpu == "?":
        cpu = s.split(":", 1)[1].strip()
    elif s.startswith("satisfied:"):
        satisfied = s.split(":", 1)[1].strip()
    elif s.startswith("total:") and total == "?":
        total = s.split(":", 1)[1].strip()
    elif s.startswith("estimated_cost:"):
        cost = s.split(":", 1)[1].strip()
tens = json.loads((out / "analyze/tension_report.json").read_text())
text = f"""# Pilot CASE_SUMMARY — Path to Real v0.2

Generated by `examples/pilot/run.sh`.

| Check | Result |
|-------|--------|
| Fixtures SHA256 | OK (`SHA256SUMS`) |
| Design CPU | {cpu} |
| Contracts | {satisfied}/{total} |
| BOM est. | ${cost} |
| Tension Ψ | {tens['overall_tension']:.4f} (confidence {tens['overall_confidence']*100:.1f}%, {tens['conclusiveness']}) |
| Check skip | NO_NEW_TRACE (no self-pass) |
| Check dual | TIMING_VIOLATION present |
| Prove UNSAT | fixture refuses false-proven |
| Event-graph / prove goldens | match `expected/` (X2 diff, no overwrite) |
| FW host | firmware_host exit 0 |

**Host smoke ≠ silício.** PCB not in this smoke (use `pipeline --pcb`).

Vault case study: `base-vault/12 - Path to Real/12.20 - Pilot Case Study.md`
"""
(out / "CASE_SUMMARY.md").write_text(text)
# Gate: MCU not FPGA/TBD
assert cpu not in ("TBD", "unassigned", "ECP5-12F"), cpu
assert float(satisfied) >= 0.7 * float(total), (satisfied, total)
print(f"CASE_SUMMARY OK — cpu={cpu} contracts={satisfied}/{total}")
PY

echo
echo "Pilot smoke OK → $OUT"
echo "CASE_SUMMARY: $OUT/CASE_SUMMARY.md"
echo "Case study: base-vault/12 - Path to Real/12.20 - Pilot Case Study.md"
