# Pilot — Moto G35 5G (OS Port Validation Assist)

Unisoc T760 / AArch64. **≠** TaurOS completo gerado pelo B.A.S.E.

| Fase | Script |
|------|--------|
| A Forense | `./run.sh` |
| B QEMU | `./run_qemu_smoke.sh` (`HIL_FW_IMAGE=…`) |
| C Hardware | [SOP.md](SOP.md) + `hw_boot_receipt.example.json` |

```bash
python3 gen_boot.py   # ANDROID! synth + mmio
./run.sh
```

Vault: `base-vault/24 - Path to v1.4/`

## Firmware real (Firmware.zip)

```bash
# Firmware.zip na raiz do repo (gitignored)
./examples/pilot_moto_g35/run_real_fw.sh
# → out_real/port_package_lk/PORT_PACKAGE.md  (primário)
```

Ver [CASE_SUMMARY_REAL_FW.md](CASE_SUMMARY_REAL_FW.md).
