#!/usr/bin/env bash
# X3 — USART1 + I2C1 no mesmo STM32 (opt-in). NÃO substitui run.sh / run_w1_spi.sh.
#
# I2C1 @ 0x40005400 (APB1) → page 0x40005000 (≠ USART1 / SPI2).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BASE="${ROOT}/target/debug/base"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="${PILOT}/out_x3_i2c"
PREF="STMicroelectronics"
CLASSIFY="0x40013000=uart,0x40005000=i2c"

if [[ ! -x "$BASE" ]]; then
  echo "Building base-cli…"
  (cd "$ROOT" && cargo build -p base-cli)
fi

echo "== X3 STM32 USART+I2C fixture integrity =="
(cd "$PILOT" && sha256sum -c SHA256SUMS.x3)

rm -rf "$OUT"
mkdir -p "$OUT"

echo "== bir I2C1 =="
"$BASE" bir "$PILOT/pilot_i2c.bsl" --compile --validate -o "$OUT/bir_i2c"

echo "== analyze dual (classify per page) =="
"$BASE" analyze "$PILOT/fw.bin" \
  --mmio-traces "$PILOT/mmio_usart_i2c.json" \
  --classify "$CLASSIFY" \
  -o "$OUT/analyze"

grep -E 'base_address: (1073819648|0x40013000)' "$OUT/analyze/hardware_spec.yaml" >/dev/null
grep -E 'base_address: (1073762304|0x40005000)' "$OUT/analyze/hardware_spec.yaml" >/dev/null
grep -Eqi 'kind:[[:space:]]*(Uart|uart)' "$OUT/analyze/hardware_spec.yaml"
grep -Eqi 'kind:[[:space:]]*(I2c|i2c)' "$OUT/analyze/hardware_spec.yaml"

echo "== design (prefer ST) =="
"$BASE" design "$OUT/analyze/hardware_spec.yaml" \
  --preferred-manufacturer "$PREF" \
  --max-bom-cost 80 \
  -o "$OUT/design"
grep -q 'STM32F103C8' "$OUT/design/reference_design.yaml"

echo "== synth (prefer ST) =="
"$BASE" synth "$OUT/analyze/hardware_spec.yaml" \
  --preferred-manufacturer "$PREF" \
  --max-bom-cost 80 \
  -o "$OUT/synth"
grep -q 'STM32F103C8' "$OUT/synth/synthesized_spec.yaml"
grep -Eqi 'interface:[[:space:]]*uart|"uart"|uart' "$OUT/synth/synthesized_spec.yaml"
grep -Eqi 'interface:[[:space:]]*i2c|"i2c"|i2c' "$OUT/synth/synthesized_spec.yaml"

echo "== pcb draft (Y1 I2C1 labels, NOT FABRICABLE) =="
"$BASE" pcb "$OUT/synth/synthesized_spec.yaml" -o "$OUT/pcb"
SCH="$OUT/pcb/project.kicad_sch"
test -f "$SCH"
grep -q 'NOT FABRICABLE' "$SCH"
grep -Eq 'usart1_tx|uart0_tx' "$SCH"
grep -Eq 'PA9|PA10' "$SCH"
grep -Eq 'i2c1_scl|i2c_scl' "$SCH"
grep -Eq 'i2c1_sda|i2c_sda' "$SCH"
grep -Eq 'PB6|PB7' "$SCH"

echo "== prove I2C1 contracts =="
"$BASE" prove "$PILOT/contracts_i2c.yaml" -o "$OUT/prove_i2c"

echo "== event-graph + goldens I2C (Y2) =="
"$BASE" event-graph "$PILOT/contracts_i2c.yaml" "$PILOT/trace_i2c.csv" \
  --format dot -o "$OUT/event_graph_i2c"
"$BASE" event-graph "$PILOT/contracts_i2c.yaml" "$PILOT/trace_i2c.csv" \
  --format mermaid -o "$OUT/event_graph_i2c"
diff -u "$PILOT/expected_i2c/event_graph.dot" "$OUT/event_graph_i2c/event_graph.dot"
diff -u "$PILOT/expected_i2c/event_graph.mmd" "$OUT/event_graph_i2c/event_graph.mmd"
python3 - "$OUT/prove_i2c/proof_report.json" "$PILOT/expected_i2c/proof_report.golden.json" <<'PY'
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
print("prove I2C golden OK")
PY

echo "== replay I2C1 =="
"$BASE" replay "$PILOT/trace_i2c.csv" \
  --contracts "$PILOT/contracts_i2c.yaml" \
  --output "$OUT/violations_i2c.json"

echo "== CASE_SUMMARY_X3 =="
python3 - "$OUT" <<'PY'
import pathlib, sys, re
out = pathlib.Path(sys.argv[1])
design = (out / "design" / "reference_design.yaml").read_text()
synth = (out / "synth" / "synthesized_spec.yaml").read_text()
assert "STM32F103C8" in design
assert re.search(r"(?i)uart", synth), "synth missing uart"
assert re.search(r"(?i)i2c", synth), "synth missing i2c"
summary = out / "CASE_SUMMARY_X3.md"
summary.write_text(
    "# X3 STM32 CASE SUMMARY\n\n"
    "- Dual wedge: USART1 @ 0x40013800 + I2C1 @ 0x40005400\n"
    "- Classify: `0x40013000=uart,0x40005000=i2c`\n"
    "- Pins I2C1: PB6/PB7 labels no draft PCB (Y1; NOT FABRICABLE)\n"
    "- Goldens Y2: event-graph + prove vs expected_i2c/ (diff, não overwrite)\n"
    "- Prefer manufacturer: STMicroelectronics → STM32F103C8\n"
    "- Gates USART (`run.sh`) e SPI (`run_w1_spi.sh`) intocados\n"
    f"- design bytes: {len(design)}\n"
    "- status: OK\n"
)
print(summary.read_text())
PY

echo "X3 STM32 I2C smoke OK → $OUT"
