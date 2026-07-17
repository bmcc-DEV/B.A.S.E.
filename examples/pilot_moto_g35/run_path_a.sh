#!/usr/bin/env bash
# Caminho A one-shot: gera assists → empacota handoff_external.
# ≠ flash · ≠ OS turnkey · generates_os=false
set -euo pipefail
PILOT="$(cd "$(dirname "$0")" && pwd)"
cd "$PILOT"

SKIP_USB="${SKIP_USB:-0}"
SKIP_SPECTER="${SKIP_SPECTER:-1}"   # default skip — path A foca handoff externo
SKIP_QEMU="${SKIP_QEMU:-1}"

echo "=== Path A — assist → handoff externo ==="

if [[ "$SKIP_USB" != "1" ]]; then
  echo "--- USB probe ---"
  ./run_usb_probe.sh || echo "WARN: usb-probe skipped (sem ADB?) — usa out_real existente"
fi

if [[ ! -f out_real/usb_cross/wedge_mmio_map.yaml ]]; then
  echo "--- USB×DTB cross ---"
  ./run_usb_cross.sh
fi

if [[ ! -f out_real/wedge_p0/board-ums9620-wedge-p0.dtsi ]]; then
  echo "--- wedge-p0 ---"
  ./run_wedge_p0.sh
fi

if [[ ! -f out_real/clocks_pinctrl/CLOCKS_PINCTRL.md ]]; then
  echo "--- clocks/pinctrl ---"
  ./run_clocks_pinctrl.sh
fi

# Always refresh clocks + resolve (cheap, needs vendor_boot)
./run_clocks_pinctrl.sh
# Ensure wedge-p0 exists after cross
[[ -f out_real/wedge_p0/board-ums9620-wedge-p0.dtsi ]] || ./run_wedge_p0.sh

if [[ "$SKIP_SPECTER" != "1" ]]; then
  ./run_wedge_specter_live.sh || true
fi
if [[ "$SKIP_QEMU" != "1" ]]; then
  ./run_wedge_qemu_smoke.sh || true
fi

# Fase C draft if missing
[[ -f out_real/wedge_hw/PHASE_C_CHECKLIST.md ]] || ./run_wedge_hw_assist.sh || true

echo "--- pack + phandle resolve ---"
./pack_external_handoff.sh

DEST="$PILOT/out_real/handoff_external"
echo ""
echo "Path A OK → $DEST"
if [[ -f "$DEST/VALIDATE.json" ]]; then
  python3 -c "import json;print(json.load(open('$DEST/VALIDATE.json')))"
fi
echo ""
echo "Próximo (manual):"
echo "  1. Copia $DEST para o tree externo"
echo "  2. Inclui dt/board-ums9620-wedge-merged.dtsi (ou p0+clocks)"
echo "  3. Build + flash manual"
echo "  4. $DEST/lab/lab_watch_assist.sh   # monitor read-only"
echo "generates_os=false · auto_flash_complete=false"
