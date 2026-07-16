#!/usr/bin/env bash
# Y3 — USART1 + SPI2 + I2C1 no mesmo STM32 (opt-in). NÃO substitui run.sh / run_w1_spi.sh / run_x3_i2c.sh.
#
# Páginas 4K distintas: USART1→0x40013000, SPI2→0x40003000, I2C1→0x40005000.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BASE="${ROOT}/target/debug/base"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="${PILOT}/out_y3_triple"
PREF="STMicroelectronics"
CLASSIFY="0x40013000=uart,0x40003000=spi,0x40005000=i2c"

if [[ ! -x "$BASE" ]]; then
  echo "Building base-cli…"
  (cd "$ROOT" && cargo build -p base-cli)
fi

echo "== Y3 STM32 triple fixture integrity =="
(cd "$PILOT" && sha256sum -c SHA256SUMS.y3)

rm -rf "$OUT"
mkdir -p "$OUT"

echo "== analyze triple (classify per page) =="
"$BASE" analyze "$PILOT/fw.bin" \
  --mmio-traces "$PILOT/mmio_usart_spi_i2c.json" \
  --classify "$CLASSIFY" \
  -o "$OUT/analyze"

grep -E 'base_address: (1073819648|0x40013000)' "$OUT/analyze/hardware_spec.yaml" >/dev/null
grep -E 'base_address: (1073754112|0x40003000)' "$OUT/analyze/hardware_spec.yaml" >/dev/null
grep -E 'base_address: (1073762304|0x40005000)' "$OUT/analyze/hardware_spec.yaml" >/dev/null
grep -Eqi 'kind:[[:space:]]*(Uart|uart)' "$OUT/analyze/hardware_spec.yaml"
grep -Eqi 'kind:[[:space:]]*(Spi|spi)' "$OUT/analyze/hardware_spec.yaml"
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
grep -Eqi 'interface:[[:space:]]*spi|"spi"|spi' "$OUT/synth/synthesized_spec.yaml"
grep -Eqi 'interface:[[:space:]]*i2c|"i2c"|i2c' "$OUT/synth/synthesized_spec.yaml"

echo "== pcb draft (triple labels, NOT FABRICABLE) =="
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
grep -Eq 'i2c1_scl|i2c_scl' "$SCH"
grep -Eq 'i2c1_sda|i2c_sda' "$SCH"
grep -Eq 'PB6|PB7' "$SCH"

echo "== CASE_SUMMARY_Y3 =="
python3 - "$OUT" <<'PY'
import pathlib, sys, re
out = pathlib.Path(sys.argv[1])
design = (out / "design" / "reference_design.yaml").read_text()
synth = (out / "synth" / "synthesized_spec.yaml").read_text()
assert "STM32F103C8" in design
assert re.search(r"(?i)uart", synth), "synth missing uart"
assert re.search(r"(?i)spi", synth), "synth missing spi"
assert re.search(r"(?i)i2c", synth), "synth missing i2c"
summary = out / "CASE_SUMMARY_Y3.md"
summary.write_text(
    "# Y3 STM32 CASE SUMMARY\n\n"
    "- Triple wedge: USART1 @ 0x40013800 + SPI2 @ 0x40003800 + I2C1 @ 0x40005400\n"
    "- Classify: `0x40013000=uart,0x40003000=spi,0x40005000=i2c`\n"
    "- Pins: PA9/10 + PB13/14/15 + PB6/7 (NOT FABRICABLE)\n"
    "- Prefer manufacturer: STMicroelectronics → STM32F103C8\n"
    "- Gates USART / SPI / I2C isolados intocados\n"
    f"- design bytes: {len(design)}\n"
    "- status: OK\n"
)
print(summary.read_text())
PY

echo "Y3 STM32 triple smoke OK → $OUT"
