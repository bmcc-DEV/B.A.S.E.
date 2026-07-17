#!/usr/bin/env bash
# G35 fase C assist — receipt + checklist a partir do wedge P0.
# READ-ONLY: sem flash. ≠ production · ≠ earlycon verificado.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="${WEDGE_HW_OUT:-$PILOT/out_real/wedge_hw}"
WEDGE_P0="${WEDGE_P0_DIR:-$PILOT/out_real/wedge_p0}"
MAP="${WEDGE_MAP:-$PILOT/out_real/usb_cross/wedge_mmio_map.yaml}"
mkdir -p "$OUT"

DATE_UTC=$(date -u +%Y-%m-%dT%H:%M:%SZ)
DATE_DAY=$(date -u +%Y-%m-%d)

# Ensure wedge artefacts exist
if [[ ! -f "$MAP" ]] || [[ ! -d "$WEDGE_P0" ]]; then
  echo "NOTE: regenerating wedge atlas/stub…"
  "$PILOT/run_usb_cross.sh" || true
  "$PILOT/run_wedge_p0.sh" || true
fi

UART="0x20200000"
GIC="0x12000000"
UFS="0x22000000"
if [[ -f "$MAP" ]]; then
  eval "$(python3 - <<PY
import yaml
m=yaml.safe_load(open("$MAP"))
for e in m.get("entries") or []:
  c=e.get("class"); h=e.get("absolute_base_hex")
  if c=="uart" and h: print(f"UART={h!r}")
  if c=="gic" and h: print(f"GIC={h!r}")
  if c=="storage_emmc_ufs" and h: print(f"UFS={h!r}")
PY
)"
fi

ADB_STATE="none"
FB_STATE="none"
PRODUCT=""
if command -v adb >/dev/null 2>&1; then
  if adb devices 2>/dev/null | grep -q $'\tdevice$'; then
    ADB_STATE="device"
    PRODUCT=$(adb shell getprop ro.product.model 2>/dev/null | tr -d '\r' || true)
  elif adb devices 2>/dev/null | grep -q $'\trecovery$'; then
    ADB_STATE="recovery"
  fi
fi
if command -v fastboot >/dev/null 2>&1; then
  if fastboot devices 2>/dev/null | grep -q .; then
    FB_STATE="present"
  fi
fi

# Copy earlycon hints if present
if [[ -f "$WEDGE_P0/cmdline_earlycon.txt" ]]; then
  cp -f "$WEDGE_P0/cmdline_earlycon.txt" "$OUT/cmdline_earlycon.txt"
fi
if [[ -f "$WEDGE_P0/board-ums9620-wedge-p0.dtsi" ]]; then
  cp -f "$WEDGE_P0/board-ums9620-wedge-p0.dtsi" "$OUT/board-ums9620-wedge-p0.dtsi"
fi

# Draft receipt (operator fills result after lab test)
RECEIPT="$OUT/hw_boot_receipt.draft.json"
python3 - <<PY
import json
r = {
  "phase": "C",
  "wedge": "linux_wedge_uart_ufs_g35",
  "device": "moto_g35_5g",
  "product_model": """$PRODUCT""" or None,
  "operator": "_______________",
  "date": "$DATE_DAY",
  "generated_at": "$DATE_UTC",
  "image_sha256": "_______________",
  "method": "not_run",
  "result": "not_run",
  "console_log_path": None,
  "earlycon_candidate": None,
  "wedge_bases": {
    "uart": "$UART",
    "gic": "$GIC",
    "ufs": "$UFS",
  },
  "lab_presence": {
    "adb": "$ADB_STATE",
    "fastboot": "$FB_STATE",
  },
  "sow_signed": False,
  "production": False,
  "flashed": False,
  "generates_os": False,
  "note": "draft assist — fill result after lab; NEVER claim production; flash is manual/out-of-band",
}
hints = []
try:
  hints = open("$OUT/cmdline_earlycon.txt").read().strip().splitlines()
except Exception:
  pass
if hints:
  r["earlycon_candidate"] = hints[0]
open("$RECEIPT","w").write(json.dumps(r, indent=2)+"\n")
print("draft receipt → $RECEIPT")
PY

cat > "$OUT/PHASE_C_CHECKLIST.md" <<EOF
# Fase C — Wedge P0 HW assist (G35)

≠ OS turnkey · ≠ flash automático · \`production: false\` · \`flashed: false\`

Generated: \`$DATE_UTC\`

## Lab presence (read-only)

- adb: **$ADB_STATE**
- fastboot: **$FB_STATE**
- product: \`${PRODUCT:-unknown}\`

## Wedge bases (atlas)

| Class | Absolute |
|-------|----------|
| UART | \`$UART\` |
| GIC  | \`$GIC\` |
| UFS  | \`$UFS\` |

## Pré-checks

- [ ] SOW OS-port assinado
- [ ] Build **externo** (boot.img / Image) disponível
- [ ] Unlock / lab policy OK
- [ ] Backup / brick recovery conhecido

## Passos manuais (lab)

1. Copiar earlycon de \`cmdline_earlycon.txt\` para o cmdline do teu Image
2. Integrar \`board-ums9620-wedge-p0.dtsi\` no tree externo (ou só cmdline)
3. Flash **manual** (fastboot/EDL) — **este script NÃO faz flash**
4. Capturar consola (UART físico se existir, ou ramo log)
5. Preencher \`hw_boot_receipt.draft.json\` → \`hw_boot_receipt.json\`:
   - \`result\`: boot_ok | panic | hang | earlycon_seen | not_run
   - \`image_sha256\`, \`operator\`, \`console_log_path\`
6. Manter \`production: false\`

## Proibido

- Flash no CI
- \`mode=production\` / \`generates_os: true\`
- Claim “port validado” sem receipt + SOW

Ref: SOP.md · vault 24.30 · 24.44
EOF

cat > "$OUT/CASE_SUMMARY_WEDGE_HW.md" <<EOF
# Wedge P0 — fase C assist

- draft receipt: \`$RECEIPT\`
- checklist: \`$OUT/PHASE_C_CHECKLIST.md\`
- adb=$ADB_STATE fastboot=$FB_STATE
- flashed: **false** (script read-only)
- generates_os: false
EOF

echo "Fase C assist → $OUT"
echo "  PHASE_C_CHECKLIST.md · hw_boot_receipt.draft.json"
echo "READ-ONLY: nenhum flash executado · generates_os=false"
