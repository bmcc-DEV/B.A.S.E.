# HIL / lab SOP — Moto G35 5G (fase C)

> Boot em hardware real. **Não** é flash production turnkey nem TaurOS drop-in.

## 1. Pré-checks

- [ ] Fase A verde: `./examples/pilot_moto_g35/run.sh`
- [ ] Fase B (opcional): QEMU com `HIL_FW_IMAGE`
- [ ] SOW §OS-port assinado ([[24.21 - SOW OS-Port Checklist]])
- [ ] Build **externo** TaurOS / boot.img real disponível

## 2. Lab (fastboot / EDL — conforme unlock do dispositivo)

```bash
# Inventário vivo via USB (ADB autorizado ou bootloader) — read-only
./examples/pilot_moto_g35/run_usb_probe.sh
./examples/pilot_moto_g35/run_usb_cross.sh
./examples/pilot_moto_g35/run_wedge_p0.sh
# → out_real/wedge_p0/ (DTSI + earlycon hints) — ≠ verified on silicon

# Exemplo flash — adaptar ao lab do Cliente (≠ CI):
# adb reboot bootloader
# fastboot flash boot out/tauros_boot.img
# fastboot reboot
```

## 3. Receipt

```bash
./examples/pilot_moto_g35/run_wedge_hw_assist.sh
# → out_real/wedge_hw/hw_boot_receipt.draft.json + PHASE_C_CHECKLIST.md
# READ-ONLY — não faz flash
```

Preencher draft → `hw_boot_receipt.json` (não commitir):

- hash do binário
- operador / data
- resultado: boot_ok / panic / hang / earlycon_seen
- `production: false` · `flashed` só se flash manual ocorreu

## 4. Proibido

- Claim “port validado de vez” sem A+B+C + SOW  
- Flash no CI default  
- `mode=production`  

Ref: `base-vault/24 - Path to v1.4/24.30 - OS Port Validation Gate.md`
