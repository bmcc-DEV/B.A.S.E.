#!/usr/bin/env bash
# Empacota artefactos wedge → handoff_external (com phandles resolvidos + DTSI fundido).
# ≠ OS turnkey · ≠ flash · generates_os=false
set -euo pipefail
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT_REAL="$PILOT/out_real"
DEST="${HANDOFF_OUT:-$OUT_REAL/handoff_external}"
DTB="${WEDGE_DTB:-$PILOT/real_fw/vendor_boot.img}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"

need=(
  "$OUT_REAL/wedge_p0/board-ums9620-wedge-p0.dtsi"
  "$OUT_REAL/wedge_p0/cmdline_earlycon.txt"
  "$OUT_REAL/wedge_p0/hal_wedge_p0.h"
  "$OUT_REAL/wedge_p0/hal_wedge_p0.c"
  "$OUT_REAL/usb_cross/wedge_mmio_map.yaml"
  "$OUT_REAL/clocks_pinctrl/board-ums9620-wedge-clocks-pinctrl.dtsi"
  "$OUT_REAL/clocks_pinctrl/CLOCKS_PINCTRL.md"
)

missing=0
for f in "${need[@]}"; do
  if [[ ! -f "$f" ]]; then
    echo "MISSING: $f"
    missing=1
  fi
done
if [[ "$missing" -ne 0 ]]; then
  echo "Corre: ./run_path_a.sh   (gera o que falta e empacota)"
  exit 1
fi

rm -rf "$DEST"
mkdir -p "$DEST"/{dt,cmdline,hal,atlas,lab,docs,resolved}

cp -a "$OUT_REAL/wedge_p0/board-ums9620-wedge-p0.dtsi" "$DEST/dt/"
cp -a "$OUT_REAL/clocks_pinctrl/board-ums9620-wedge-clocks-pinctrl.dtsi" "$DEST/dt/"
cp -a "$OUT_REAL/wedge_p0/cmdline_earlycon.txt" "$DEST/cmdline/"
cp -a "$OUT_REAL/wedge_p0/hal_wedge_p0.h" "$OUT_REAL/wedge_p0/hal_wedge_p0.c" "$DEST/hal/"
cp -a "$OUT_REAL/usb_cross/wedge_mmio_map.yaml" "$DEST/atlas/"
cp -a "$OUT_REAL/clocks_pinctrl/CLOCKS_PINCTRL.md" "$OUT_REAL/clocks_pinctrl/clocks_pinctrl_hints.yaml" "$DEST/atlas/"
[[ -f "$OUT_REAL/usb_cross/BRINGUP_CHECKLIST.md" ]] && cp -a "$OUT_REAL/usb_cross/BRINGUP_CHECKLIST.md" "$DEST/docs/"
[[ -f "$OUT_REAL/wedge_hw/PHASE_C_CHECKLIST.md" ]] && cp -a "$OUT_REAL/wedge_hw/PHASE_C_CHECKLIST.md" "$DEST/lab/"
[[ -f "$PILOT/EXTERNAL_TREE.md" ]] && cp -a "$PILOT/EXTERNAL_TREE.md" "$DEST/EXTERNAL_TREE.md"
[[ -f "$PILOT/POSTMARKETOS.md" ]] && cp -a "$PILOT/POSTMARKETOS.md" "$DEST/POSTMARKETOS.md"
[[ -f "$PILOT/WEDGE_HANDOFF.md" ]] && cp -a "$PILOT/WEDGE_HANDOFF.md" "$DEST/docs/"

# Resolve phandles from vendor DTB
if [[ -f "$DTB" ]]; then
  python3 "$PILOT/resolve_dt_phandles.py" --dtb "$DTB" -o "$DEST/resolved" --uart0-base 0x20200000
  cp -a "$DEST/resolved/board-ums9620-wedge-merged.dtsi" "$DEST/dt/"
else
  echo "WARN: sem $DTB — skip phandle resolve"
fi

# Concatenate include-friendly board file
{
  echo "/* AUTO: include this single file in your board DTS */"
  echo "/* generates_os: false */"
  echo '#include "board-ums9620-wedge-p0.dtsi"'
  echo '#include "board-ums9620-wedge-clocks-pinctrl.dtsi"'
  if [[ -f "$DEST/dt/board-ums9620-wedge-merged.dtsi" ]]; then
    echo "/* Prefer resolved clocks from merged fragment for UART0: */"
    echo "/* #include \"board-ums9620-wedge-merged.dtsi\" */"
  fi
} > "$DEST/dt/INCLUDE_ME.dtsi"

# Artifact hashes for receipt
HASH_P0=$(sha256sum "$DEST/dt/board-ums9620-wedge-p0.dtsi" | awk '{print $1}')
HASH_CLK=$(sha256sum "$DEST/dt/board-ums9620-wedge-clocks-pinctrl.dtsi" | awk '{print $1}')
HASH_MERGED="null"
[[ -f "$DEST/dt/board-ums9620-wedge-merged.dtsi" ]] && \
  HASH_MERGED="\"$(sha256sum "$DEST/dt/board-ums9620-wedge-merged.dtsi" | awk '{print $1}')\""

UART0_CLOCKS="null"
if [[ -f "$DEST/resolved/clocks_resolved.json" ]]; then
  UART0_CLOCKS=$(python3 -c "import json; r=json.load(open('$DEST/resolved/clocks_resolved.json')); u=next((x for x in r['uart_bindings'] if x['path'].endswith('serial@0')), None); print(json.dumps(u.get('clocks_dts') if u else None))")
fi

cat > "$DEST/lab/hw_boot_receipt.json" <<EOF
{
  "phase": "C",
  "wedge": "linux_wedge_uart_ufs_g35",
  "device": "moto_g35_5g",
  "product_model": "moto g35 5G",
  "operator": "_______________",
  "date": "$(date -u +%Y-%m-%d)",
  "packed_at": "$STAMP",
  "image_sha256": "_______________",
  "method": "not_run",
  "result": "not_run",
  "console_log_path": null,
  "earlycon_candidate": "earlycon=uart8250,mmio32,0x20200000,115200n8",
  "uart0_clocks_dts": $UART0_CLOCKS,
  "artifact_sha256": {
    "board_p0_dtsi": "$HASH_P0",
    "board_clocks_dtsi": "$HASH_CLK",
    "board_merged_dtsi": $HASH_MERGED
  },
  "wedge_bases": {
    "uart": "0x20200000",
    "gicd": "0x12000000",
    "gicr": "0x12040000",
    "ufs": "0x22000000",
    "ap_clk": "0x20010000",
    "pinctrl": "0x642e0000"
  },
  "production": false,
  "flashed": false,
  "generates_os": false,
  "auto_flash_complete": false,
  "note": "fill after manual lab — use ./lab_watch_assist.sh after flash"
}
EOF

# Validate package
python3 - <<PY
import json, pathlib, sys
d=pathlib.Path("$DEST")
req=["dt/board-ums9620-wedge-p0.dtsi","cmdline/cmdline_earlycon.txt","lab/hw_boot_receipt.json","EXTERNAL_TREE.md"]
bad=[r for r in req if not (d/r).is_file()]
merged=(d/"dt/board-ums9620-wedge-merged.dtsi").is_file()
receipt=json.loads((d/"lab/hw_boot_receipt.json").read_text())
report={
  "ok": not bad,
  "missing": bad,
  "merged_dtsi": merged,
  "uart0_clocks_dts": receipt.get("uart0_clocks_dts"),
  "generates_os": False,
  "auto_flash_complete": False,
}
(d/"VALIDATE.json").write_text(json.dumps(report, indent=2)+"\n")
print("validate:", report)
sys.exit(0 if report["ok"] else 1)
PY

cat > "$DEST/MANIFEST.txt" <<EOF
B.A.S.E. wedge handoff — ums9620 / moto g35
packed_at: $STAMP
tag_hint: v1.6.3-rc+
generates_os: false
auto_flash_complete: false

START: POSTMARKETOS.md  (alvo: postmarketOS / motorola-manila)
ALSO:  EXTERNAL_TREE.md
DT prefer: dt/board-ums9620-wedge-merged.dtsi  (UART0 clocks resolvidos)
resolved/: CLOCKS_RESOLVED.md · clocks_resolved.json
lab/:      hw_boot_receipt.json · lab_watch_assist.sh (após flash manual)
EOF

# Convenience copy of lab watcher into package
cp -a "$PILOT/lab_watch_assist.sh" "$DEST/lab/" 2>/dev/null || true

echo "Handoff externo → $DEST"
echo "UART0 clocks: $UART0_CLOCKS"
echo "generates_os=false · flashed=false"
