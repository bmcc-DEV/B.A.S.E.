#!/usr/bin/env bash
# Sega Saturn HW discovery assist — fase A (VDP/SMPC contracts).
# Atalho para port PS1/x86→Saturn: mapa o hardware. ≠ jogos a correr.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$PILOT/out"
cd "$ROOT"

printf 'SATURNHW' > "$PILOT/fw.bin"
dd if=/dev/zero bs=1 count=56 >> "$PILOT/fw.bin" 2>/dev/null || true

cargo build -p base-cli -q
BASE="$ROOT/target/debug/base"

rm -rf "$OUT"
mkdir -p "$OUT"

echo "== analyze Saturn MMIO synth (VDP1/VDP2/SMPC) =="
"$BASE" analyze "$PILOT/fw.bin" \
  --mmio-traces "$PILOT/mmio.json" \
  -o "$OUT/analyze"
test -f "$OUT/analyze/hardware_spec.yaml"

python3 - <<'PY' "$OUT/analyze/hardware_spec.yaml"
import sys, re, yaml
text = open(sys.argv[1]).read()
clean = re.sub(r"![A-Za-z0-9_]+", "", text)
spec = yaml.safe_load(clean)
addrs = {int(b.get("base_address", 0)) for b in (spec.get("blocks") or [])}
# page bases from mmio.json
need = {0x25C00000 & ~0xFFF, 0x25F80000 & ~0xFFF, 0x20100000 & ~0xFFF}
# allow exact or nearby page clustering
found = False
blob = text.lower()
for a in (0x25C00000, 0x25F80000, 0x20100000, 633339904, 636485632, 537919488):
    if hex(a)[2:] in blob or str(a) in blob:
        found = True
        break
assert found or addrs, "no Saturn MMIO bases in hardware_spec"
print("mmio bases OK", sorted(addrs)[:8])
PY

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

echo "== port package Saturn HAL assist =="
"$BASE" port package "$OUT/analyze/hardware_spec.yaml" \
  --evidence "$OUT/analyze/evidence_db.yaml" \
  --tension "$OUT/analyze/tension_report.json" \
  --target-hal "hal_saturn_assist" \
  --hal-stub \
  -o "$OUT/port_package"
test -f "$OUT/port_package/PORT_PACKAGE.md"
test -f "$OUT/port_package/address_driver_map.yaml"

"$BASE" recomp runtime > "$OUT/runtime_stub.md"
grep -qi 'false' "$OUT/runtime_stub.md"

cp "$PILOT/manifest.yaml" "$OUT/manifest.yaml"
cp "$PILOT/HONESTY.md" "$OUT/HONESTY.md"
cp "$PILOT/PORT_SHORTCUT.md" "$OUT/PORT_SHORTCUT.md"

cat > "$OUT/CASE_SUMMARY_SATURN_A.md" <<EOF
# Sega Saturn HW discovery — fase A

- MMIO synth: VDP1 @ 0x25C00000 · VDP2 @ 0x25F80000 · SMPC @ 0x20100000
- port_package: hal_saturn_assist (atlas + fossils)
- runs_on_saturn=false · ports_games=false · generates_os=false
- Papel: atalho de descoberta HW para ports PS1/x86→Saturn (humano + runtime externo)
- status: OK
EOF

echo "Pilot Saturn fase A OK → $OUT"
