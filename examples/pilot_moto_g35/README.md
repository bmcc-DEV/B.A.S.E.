# Pilot — Moto G35 5G (OS Port Validation Assist)

Unisoc T760 / AArch64. **≠** TaurOS completo gerado pelo B.A.S.E.

| Fase | Script |
|------|--------|
| A Forense | `./run.sh` |
| B QEMU | `./run_qemu_smoke.sh` (`HIL_FW_IMAGE=…`) |
| B+ Specter Live | `./run_virt_live.sh` (NDJSON→Ψ; QEMU opcional) |
| Twin↔guest | `base virt twin --spec virt/hardware_spec_mame_stub.yaml --evidence …` |
| USB HW probe | `./run_usb_probe.sh` (ADB/fastboot/lsusb → `out_real/usb_probe/`) |
| USB×DTB bring-up | `./run_usb_cross.sh` → `out_real/usb_cross/BRINGUP_CHECKLIST.md` |
| Wedge P0 stub | `./run_wedge_p0.sh` → `out_real/wedge_p0/` (DTSI/earlycon/HAL) |
| Wedge QEMU smoke | `./run_wedge_qemu_smoke.sh` → Specter twin + QEMU virt |
| Fase C assist | `./run_wedge_hw_assist.sh` → receipt draft (sem flash) |
| C Hardware | [SOP.md](SOP.md) + `hw_boot_receipt.example.json` |

```bash
python3 gen_boot.py   # ANDROID! synth + mmio
./run.sh
./run_virt_live.sh    # Path to v1.5 — ≠ TaurOS
```

Vault: `base-vault/24 - Path to v1.4/` · `base-vault/25 - Path to v1.5/`

## Firmware real (Firmware.zip)

```bash
# Firmware.zip na raiz do repo (gitignored)
./examples/pilot_moto_g35/run_real_fw.sh
# → out_real/port_package_lk/PORT_PACKAGE.md  (primário)
```

Ver [CASE_SUMMARY_REAL_FW.md](CASE_SUMMARY_REAL_FW.md).
