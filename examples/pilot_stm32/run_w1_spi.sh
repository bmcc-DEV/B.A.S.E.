#!/usr/bin/env bash
# W1 — USART1 + SPI2 no mesmo STM32 (opt-in). NÃO substitui run.sh (USART-only).
#
# SPI1 @ 0x40013000 partilha página 4K com USART1 @ 0x40013800 → dual usa SPI2 @ 0x40003800.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BASE="${ROOT}/target/debug/base"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="${PILOT}/out_w1_spi"
PREF="STMicroelectronics"
# Pages: USART1→0x40013000, SPI2→0x40003000
CLASSIFY="0x40013000=uart,0x40003000=spi"

if [[ ! -x "$BASE" ]]; then
  echo "Building base-cli…"
  (cd "$ROOT" && cargo build -p base-cli)
fi

echo "== W1 STM32 USART+SPI fixture integrity =="
(cd "$PILOT" && sha256sum -c SHA256SUMS.w1)

rm -rf "$OUT"
mkdir -p "$OUT"

echo "== bir SPI2 =="
"$BASE" bir "$PILOT/pilot_spi.bsl" --compile --validate -o "$OUT/bir_spi"

echo "== analyze dual (classify per page) =="
"$BASE" analyze "$PILOT/fw.bin" \
  --mmio-traces "$PILOT/mmio_usart_spi.json" \
  --classify "$CLASSIFY" \
  -o "$OUT/analyze"

grep -E 'base_address: (1073819648|0x40013000)' "$OUT/analyze/hardware_spec.yaml" >/dev/null
grep -E 'base_address: (1073754112|0x40003000)' "$OUT/analyze/hardware_spec.yaml" >/dev/null
grep -Eqi 'kind:[[:space:]]*(Uart|uart)' "$OUT/analyze/hardware_spec.yaml"
grep -Eqi 'kind:[[:space:]]*(Spi|spi)' "$OUT/analyze/hardware_spec.yaml"

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
grep -Eqi 'interface:[[:space:]]*spi|"spi"|spi' "$OUT/synth/synthesized_spec.yaml"

echo "== pcb draft (X1 SPI2 labels, NOT FABRICABLE) =="
"$BASE" pcb "$OUT/synth/synthesized_spec.yaml" -o "$OUT/pcb"
SCH="$OUT/pcb/project.kicad_sch"
test -f "$SCH"
grep -q 'NOT FABRICABLE' "$SCH"
grep -Eq 'usart1_tx|uart0_tx' "$SCH"
grep -Eq 'PA9|PA10' "$SCH"
grep -q 'spi2_sck' "$SCH"
grep -Eq 'spi2_miso|spi2_rx' "$SCH"
grep -Eq 'spi2_mosi|spi2_tx' "$SCH"
grep -Eq 'PB13|PB14|PB15' "$SCH"

echo "== prove SPI2 contracts =="
"$BASE" prove "$PILOT/contracts_spi.yaml" -o "$OUT/prove_spi"

echo "== replay SPI2 =="
"$BASE" replay "$PILOT/trace_spi.csv" \
  --contracts "$PILOT/contracts_spi.yaml" \
  --output "$OUT/violations_spi.json"

echo "== CASE_SUMMARY_W1 =="
python3 - "$OUT" <<'PY'
import pathlib, sys, re
out = pathlib.Path(sys.argv[1])
design = (out / "design" / "reference_design.yaml").read_text()
synth = (out / "synth" / "synthesized_spec.yaml").read_text()
assert "STM32F103C8" in design
assert re.search(r"(?i)uart", synth), "synth missing uart"
assert re.search(r"(?i)spi", synth), "synth missing spi"
summary = out / "CASE_SUMMARY_W1.md"
summary.write_text(
    "# W1 STM32 CASE SUMMARY\n\n"
    "- Dual wedge: USART1 @ 0x40013800 + SPI2 @ 0x40003800\n"
    "- Classify: `0x40013000=uart,0x40003000=spi`\n"
    "- SPI1 @ 0x40013000 omitted (4K page collision with USART1)\n"
    "- Pins SPI2: PB13/14/15 labels no draft PCB (X1; NOT FABRICABLE)\n"
    "- Prefer manufacturer: STMicroelectronics → STM32F103C8\n"
    "- Gate USART-only (`run.sh`) intocado\n"
    f"- design bytes: {len(design)}\n"
    "- status: OK\n"
)
print(summary.read_text())
PY

echo "W1 STM32 SPI smoke OK → $OUT"
