#!/usr/bin/env bash
# U1 — STM32F103 USART1 wedge (opt-in). NÃO substitui examples/pilot/run.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BASE="${ROOT}/target/debug/base"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="${PILOT}/out"
PREF="STMicroelectronics"

if [[ ! -x "$BASE" ]]; then
  echo "Building base-cli…"
  (cd "$ROOT" && cargo build -p base-cli)
fi

echo "== U1 STM32 fixture integrity =="
(cd "$PILOT" && sha256sum -c SHA256SUMS)

rm -rf "$OUT"
mkdir -p "$OUT"

echo "== bir =="
"$BASE" bir "$PILOT/pilot.bsl" --compile --validate -o "$OUT/bir"

echo "== analyze (Capstone --disasm, V1) =="
"$BASE" analyze "$PILOT/fw.bin" --disasm -o "$OUT/analyze_disasm"
# Capstone must hit USART1 regs (page 0x40013000 / addrs 0x40013800…) without traces
grep -E 'base_address: (1073819648|0x40013000)' "$OUT/analyze_disasm/hardware_spec.yaml" >/dev/null \
  || grep -qE '40013(800|000)' "$OUT/analyze_disasm/hardware_spec.yaml"

echo "== analyze (USART1 @ 0x40013800, traces) =="
"$BASE" analyze "$PILOT/fw.bin" \
  --mmio-traces "$PILOT/mmio.json" \
  --classify uart \
  -o "$OUT/analyze"
# Clustering 4K: USART1 @ 0x40013800 → page 0x40013000 (1073819648)
grep -E 'base_address: (1073819648|0x40013000)' "$OUT/analyze/hardware_spec.yaml" >/dev/null \
  || grep -q '40013000' "$OUT/analyze/hardware_spec.yaml"
grep -Eqi 'kind:[[:space:]]*(Uart|uart)' "$OUT/analyze/hardware_spec.yaml"

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

echo "== pcb draft (V2 USART labels, NOT FABRICABLE) =="
"$BASE" pcb "$OUT/synth/synthesized_spec.yaml" -o "$OUT/pcb"
SCH="$OUT/pcb/project.kicad_sch"
test -f "$SCH"
grep -q 'NOT FABRICABLE' "$SCH"
grep -Eq 'usart1_tx|uart0_tx' "$SCH"
grep -Eq 'usart1_rx|uart0_rx' "$SCH"
grep -Eq 'PA9|PA10' "$SCH"

echo "== prove =="
"$BASE" prove "$PILOT/contracts.yaml" -o "$OUT/prove"

echo "== replay =="
"$BASE" replay "$PILOT/trace.csv" \
  --contracts "$PILOT/contracts.yaml" \
  --output "$OUT/violations.json"

echo "== CASE_SUMMARY =="
python3 - "$OUT" <<'PY'
import pathlib, sys, re
out = pathlib.Path(sys.argv[1])
design = (out / "design" / "reference_design.yaml").read_text()
assert "STM32F103C8" in design
summary = out / "CASE_SUMMARY.md"
summary.write_text(
    "# U1 STM32 CASE SUMMARY\n\n"
    "- Wedge: STM32F103 USART1 @ 0x40013800\n"
    "- Capstone --disasm: synthetic AArch64 @ page 0x40013000 (V1; ≠ Thumb silicon)\n"
    "- Pins USART1: PA9/PA10 labels no draft PCB (V2; NOT FABRICABLE)\n"
    "- Prefer manufacturer: STMicroelectronics → STM32F103C8\n"
    "- Gate RP (`examples/pilot/run.sh`) intocado\n"
    f"- design bytes: {len(design)}\n"
    "- status: OK\n"
)
print(summary.read_text())
PY

echo "U1 STM32 smoke OK → $OUT"
