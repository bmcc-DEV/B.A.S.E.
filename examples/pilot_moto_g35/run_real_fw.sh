#!/usr/bin/env bash
# Analyze real Moto G35 Firmware.zip → port package + platform inventory (≠ TaurOS rewrite).
# Requires Firmware.zip at repo root (gitignored).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PILOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"
ZIP="$ROOT/Firmware.zip"
DEST="$PILOT/real_fw"
OUT="$PILOT/out_real"

if [[ ! -f "$ZIP" ]]; then
  echo "ERROR: $ZIP missing — place Motorola Moto G35 firmware zip at repo root" >&2
  exit 2
fi

rm -rf "$DEST" "$OUT"
mkdir -p "$DEST" "$OUT"

echo "== extract key images (not super.img) =="
unzip -jo "$ZIP" \
  '*/Firmware/boot-gki.img' \
  '*/Firmware/init_boot.img' \
  '*/Firmware/vendor_boot.img' \
  '*/Firmware/EXEC_KERNEL_IMAGE.bin' \
  '*/Firmware/lk-sign.bin' \
  '*/Firmware/dtbo.img' \
  '*/Firmware/sw-version.txt' \
  '*/Firmware/flash.cfg' \
  -d "$DEST"

cargo build -p base-cli -q
BASE="$ROOT/target/debug/base"

echo "== analyze lk-sign.bin --disasm (best Capstone MMIO) =="
"$BASE" analyze "$DEST/lk-sign.bin" --disasm -o "$OUT/analyze_lk"

echo "== analyze boot-gki.img --disasm =="
"$BASE" analyze "$DEST/boot-gki.img" --disasm -o "$OUT/analyze_boot"

echo "== analyze EXEC_KERNEL_IMAGE.bin --disasm =="
"$BASE" analyze "$DEST/EXEC_KERNEL_IMAGE.bin" --disasm -o "$OUT/analyze_kernel"

echo "== port package LK (primary) =="
"$BASE" port package "$OUT/analyze_lk/hardware_spec.yaml" \
  --evidence "$OUT/analyze_lk/evidence_db.yaml" \
  --tension "$OUT/analyze_lk/tension_report.json" \
  --target-hal "hal_tauros_aarch64_g35" \
  --hal-stub \
  --dtb "$DEST/dtbo.img" \
  --flash-cfg "$DEST/flash.cfg" \
  -o "$OUT/port_package_lk"

echo "== port package boot =="
"$BASE" port package "$OUT/analyze_boot/hardware_spec.yaml" \
  --evidence "$OUT/analyze_boot/evidence_db.yaml" \
  --tension "$OUT/analyze_boot/tension_report.json" \
  --target-hal "hal_tauros_aarch64_g35" \
  --hal-stub \
  -o "$OUT/port_package_boot"

echo "== port package EXEC_KERNEL =="
"$BASE" port package "$OUT/analyze_kernel/hardware_spec.yaml" \
  --evidence "$OUT/analyze_kernel/evidence_db.yaml" \
  --tension "$OUT/analyze_kernel/tension_report.json" \
  --target-hal "hal_tauros_aarch64_g35" \
  --hal-stub \
  -o "$OUT/port_package_kernel"

echo "== platform inventory (DTBO + vendor_boot) =="
"$BASE" port platform "$DEST/dtbo.img" \
  --flash-cfg "$DEST/flash.cfg" \
  -o "$OUT/platform_dtbo"
"$BASE" port platform "$DEST/vendor_boot.img" \
  --flash-cfg "$DEST/flash.cfg" \
  -o "$OUT/platform_vendor_boot"

python3 - <<'PY' "$OUT"
import json, re, sys, yaml
from pathlib import Path
out = Path(sys.argv[1])
lines = ["# Moto G35 real Firmware.zip — CASE SUMMARY\n"]
lines.append("> Product: ums9620 / QogirN6Pro (Unisoc) · Android 14 stock · ≠ TaurOS complete\n")
lines.append("\n| Image | Blocks | Ψ | Confidence | Port package |\n|-------|--------|---|------------|--------------|\n")
for name, pkg in [
    ("analyze_lk", "port_package_lk"),
    ("analyze_boot", "port_package_boot"),
    ("analyze_kernel", "port_package_kernel"),
]:
    tens = json.loads((out/name/"tension_report.json").read_text())
    text = re.sub(r"![A-Za-z0-9_]+", "", (out/name/"hardware_spec.yaml").read_text())
    spec = yaml.safe_load(text)
    p = yaml.safe_load((out/pkg/"port_package.yaml").read_text())
    lines.append(
        f"| `{name}` | {len(spec['blocks'])} | {tens['overall_tension']:.3f} | "
        f"{tens['overall_confidence']:.1%} {tens['conclusiveness']} | "
        f"wrap={p['rewrite_avoidance']['wrap_candidates']} rewrite={p['rewrite_avoidance']['must_rewrite']} "
        f"fossils={len(p['fossil_inventory']['fossils'])} |\n"
    )
lines.append("\n## OS-port platform inventory (DTB)\n\n")
for plat_dir in ["platform_dtbo", "platform_vendor_boot"]:
    py = out / plat_dir / "platform_inventory.yaml"
    if not py.exists():
        continue
    plat = yaml.safe_load(py.read_text())
    r = plat["os_port_readiness"]
    lines.append(
        f"### `{plat_dir}` — readiness {r['score']*100:.0f}% · CPU `{plat['cpu']['isa_hint']}`\n\n"
    )
    lines.append(f"- found: {', '.join(r['found']) or '(none)'}\n")
    lines.append(f"- missing: {', '.join(r['missing']) or '(none)'}\n")
    lines.append(f"- see `{plat_dir}/PLATFORM_INVENTORY.md`\n\n")
lines.append(
    "Required classes: cpu, gic, arm_generic_timer, mmu, dram_controller, uart, "
    "gpio, pmic, storage_emmc_ufs, gpu_framebuffer, device_tree\n"
)
lines.append("\n## Primary atlas\n\n")
lines.append("- **Use `port_package_lk/` first** — Capstone MMIO real, Ψ ConclusiveMatch\n")
lines.append("- `PORT_PACKAGE.md`, `address_driver_map.yaml`, `fossil_inventory.yaml`, `hal_mmio_stub.c`\n")
lines.append("- **Platform gaps** come from DTB — MMIO heuristics alone are not enough for OS port\n")
lines.append("- Boot/kernel packages are heuristic-heavy (many Reverse labels) — cross-check with LK\n")
lines.append("\n## Honesty\n\n")
lines.append("- `generates_os: false` · `auto_fix_complete: false`\n")
lines.append("- Platform inventory ≠ OS bootable / TaurOS turnkey\n")
lines.append("- Firmware.zip / real_fw/ gitignored — not redistributed by this repo\n")
lines.append("- status: OK\n")
(out/"CASE_SUMMARY_REAL_FW.md").write_text("".join(lines))
print("".join(lines))
PY

echo "Real FW port assist OK → $OUT"
echo "Open: $OUT/port_package_lk/PORT_PACKAGE.md"
echo "Open: $OUT/platform_dtbo/PLATFORM_INVENTORY.md"
