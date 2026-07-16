#!/usr/bin/env bash
# T1 B2 — UART + SPI no mesmo RP (opt-in). NÃO substitui run.sh (gate UART).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BASE="${ROOT}/target/debug/base"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="${PILOT}/out_t1_b2"
CLASSIFY="0x40034000=uart,0x4003c000=spi"

if [[ ! -x "$BASE" ]]; then
  echo "Building base-cli…"
  (cd "$ROOT" && cargo build -p base-cli)
fi

echo "== T1 B2 fixture integrity =="
(cd "$PILOT" && sha256sum -c SHA256SUMS.b2)

rm -rf "$OUT"
mkdir -p "$OUT"

echo "== bir SPI =="
"$BASE" bir "$PILOT/pilot_spi.bsl" --compile --validate -o "$OUT/bir_spi"

echo "== analyze dual (classify per page) =="
"$BASE" analyze "$PILOT/fw.bin" \
  --mmio-traces "$PILOT/mmio_uart_spi.json" \
  --classify "$CLASSIFY" \
  -o "$OUT/analyze"

grep -E 'base_address: (1073954816|0x40034000|1073987584|0x4003c000)' \
  "$OUT/analyze/hardware_spec.yaml" >/dev/null
grep -Eqi 'kind:[[:space:]]*(Uart|uart)' "$OUT/analyze/hardware_spec.yaml"
grep -Eqi 'kind:[[:space:]]*(Spi|spi)' "$OUT/analyze/hardware_spec.yaml"

echo "== design =="
"$BASE" design "$OUT/analyze/hardware_spec.yaml" -o "$OUT/design"

echo "== synth =="
"$BASE" synth "$OUT/analyze/hardware_spec.yaml" --max-bom-cost 80 -o "$OUT/synth"
grep -Eqi 'interface:[[:space:]]*uart|"uart"|uart' "$OUT/synth/synthesized_spec.yaml"
grep -Eqi 'interface:[[:space:]]*spi|"spi"|spi' "$OUT/synth/synthesized_spec.yaml"

echo "== prove SPI contracts =="
"$BASE" prove "$PILOT/contracts_spi.yaml" -o "$OUT/prove_spi"

echo "== replay SPI =="
"$BASE" replay "$PILOT/trace_spi.csv" \
  --contracts "$PILOT/contracts_spi.yaml" \
  --output "$OUT/violations_spi.json"

echo "== CASE_SUMMARY_T1 =="
python3 - "$OUT" <<'PY'
import pathlib, sys, re
out = pathlib.Path(sys.argv[1])
design = (out / "design" / "reference_design.yaml").read_text()
synth = (out / "synth" / "synthesized_spec.yaml").read_text()
assert re.search(r"(?i)uart", synth), "synth missing uart"
assert re.search(r"(?i)spi", synth), "synth missing spi"
summary = out / "CASE_SUMMARY_T1.md"
summary.write_text(
    "# T1 B2 CASE SUMMARY\n\n"
    "- Dual wedge: UART @ 0x40034000 + SPI0 @ 0x4003c000\n"
    "- Classify: `0x40034000=uart,0x4003c000=spi`\n"
    "- Gate UART (`run.sh`) intocado\n"
    f"- design bytes: {len(design)}\n"
    "- status: OK\n"
)
print(summary.read_text())
PY

echo "T1 B2 smoke OK → $OUT"
