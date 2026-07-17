#!/usr/bin/env bash
# Pipeline completo do wedge P0 (assist). ≠ OS turnkey · ≠ flash.
set -euo pipefail
PILOT="$(cd "$(dirname "$0")" && pwd)"
cd "$PILOT"

echo "=== 1/7 USB probe ==="
./run_usb_probe.sh || echo "WARN: usb-probe skipped/failed (sem ADB?) — continua se out_real existir"

echo "=== 2/7 USB×DTB cross + atlas ==="
./run_usb_cross.sh

echo "=== 3/7 board stub ==="
./run_wedge_p0.sh

echo "=== 4/7 clocks/pinctrl hints ==="
./run_clocks_pinctrl.sh

echo "=== 5/7 Specter live (twin + QMP) ==="
./run_wedge_specter_live.sh

echo "=== 6/7 Specter + QEMU smoke (legacy) ==="
./run_wedge_qemu_smoke.sh

echo "=== 7/8 fase C assist (sem flash) ==="
./run_wedge_hw_assist.sh

echo "=== 8/8 pack handoff externo (phandles + validate) ==="
./pack_external_handoff.sh

echo ""
echo "Wedge pipeline OK (assist)."
echo "Path A one-shot: ./run_path_a.sh"
echo "Handoff → $PILOT/out_real/handoff_external/"
echo "generates_os=false · flashed=false"
