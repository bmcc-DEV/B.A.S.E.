#!/usr/bin/env bash
# iMac G3 OS-port assist — fase A (OF/MacIO contracts). Sem Capstone PPC.
# Dual target: ReactOS + Haiku (externos). ≠ OS bootável in-tree.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$PILOT/out"
cd "$ROOT"

# Placeholder FW (analyze usa --mmio-traces; bytes só para path de ficheiro)
printf 'OFIMACG3' > "$PILOT/fw.bin"
dd if=/dev/zero bs=1 count=56 >> "$PILOT/fw.bin" 2>/dev/null || true

cargo build -p base-cli -q
BASE="$ROOT/target/debug/base"

rm -rf "$OUT"
mkdir -p "$OUT"

echo "== analyze MacIO/OF mmio-traces (no Capstone PPC) =="
"$BASE" analyze "$PILOT/fw.bin" \
  --mmio-traces "$PILOT/mmio.json" \
  -o "$OUT/analyze"
test -f "$OUT/analyze/hardware_spec.yaml"
grep -E 'base_address: (2147483648|0x80000000)' "$OUT/analyze/hardware_spec.yaml" >/dev/null \
  || grep -qi '80000000' "$OUT/analyze/hardware_spec.yaml"

echo "== fields allowlist =="
python3 - <<'PY' "$OUT/analyze/hardware_spec.yaml" "$PILOT/expected/hardware_spec.fields.yaml"
import sys, re, yaml
text = open(sys.argv[1]).read()
clean = re.sub(r"![A-Za-z0-9_]+", "", text)
spec = yaml.safe_load(clean)
exp = yaml.safe_load(open(sys.argv[2]))
for k in exp["required_top_level"]:
    assert k in spec, f"missing {k}"
assert spec.get("blocks"), "blocks empty"
print("fields OK")
PY

echo "== prove contracts =="
"$BASE" prove "$PILOT/contracts.yaml" -o "$OUT/prove"
test -f "$OUT/prove/proof_report.json"

echo "== reconstruct (≠ auto-fix) =="
"$BASE" reconstruct "$OUT/analyze/hardware_spec.yaml" \
  --threshold 0.99 --max-iterations 16 \
  -o "$OUT/reconstruct"
python3 - <<'PY' "$OUT/reconstruct/convergence_report.json"
import json, sys
r = json.load(open(sys.argv[1]))
assert r.get("auto_fix_complete") is False
print("reconstruct OK", r["stop_reason"])
PY

echo "== port package ReactOS (OF/MacIO map + fossils) =="
"$BASE" port package "$OUT/analyze/hardware_spec.yaml" \
  --evidence "$OUT/analyze/evidence_db.yaml" \
  --tension "$OUT/analyze/tension_report.json" \
  --target-hal "hal_reactos_ppc_assist" \
  --hal-stub \
  -o "$OUT/port_package_reactos"
test -f "$OUT/port_package_reactos/PORT_PACKAGE.md"
test -f "$OUT/port_package_reactos/address_driver_map.yaml"

echo "== port package Haiku (OF/MacIO map + fossils) =="
"$BASE" port package "$OUT/analyze/hardware_spec.yaml" \
  --evidence "$OUT/analyze/evidence_db.yaml" \
  --tension "$OUT/analyze/tension_report.json" \
  --target-hal "hal_haiku_ppc_assist" \
  --hal-stub \
  -o "$OUT/port_package_haiku"
test -f "$OUT/port_package_haiku/PORT_PACKAGE.md"
test -f "$OUT/port_package_haiku/address_driver_map.yaml"

cp "$PILOT/manifest.yaml" "$OUT/manifest.yaml"
cp "$PILOT/REACTOS_EXTERNAL.md" "$OUT/REACTOS_EXTERNAL.md"
cp "$PILOT/HAIKU_EXTERNAL.md" "$OUT/HAIKU_EXTERNAL.md"
if [[ -f "$PILOT/EXTERNAL_PORT_ROADMAP.md" ]]; then
  cp "$PILOT/EXTERNAL_PORT_ROADMAP.md" "$OUT/EXTERNAL_PORT_ROADMAP.md"
fi
cat > "$OUT/CASE_SUMMARY_IMAC_A.md" <<EOF
# iMac G3 OS-port assist — fase A (dual target)

- MacIO/OF contracts @ 0x80000000 (synth)
- port_package_reactos (hal_reactos_ppc_assist)
- port_package_haiku (hal_haiku_ppc_assist)
- Capstone PPC: not in this path
- ReactOS: external only (PPC retired upstream)
- Haiku: external only (PPC experimental)
- auto_fix_complete=false · generates_os=false
- generates_reactos=false · generates_haiku=false
- status: OK
EOF

echo "Pilot iMac G3 fase A OK → $OUT"
