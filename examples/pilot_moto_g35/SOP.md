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
# → out_real/usb_cross/BRINGUP_CHECKLIST.md

# Exemplo flash — adaptar ao lab do Cliente (≠ CI):
# adb reboot bootloader
# fastboot flash boot out/tauros_boot.img
# fastboot reboot
```

## 3. Receipt

Preencher `hw_boot_receipt.example.json` → `hw_boot_receipt.json` (não commitir secrets):

- hash do binário
- operador / data
- resultado: boot console / panic / hang
- `production: false`

## 4. Proibido

- Claim “port validado de vez” sem A+B+C + SOW  
- Flash no CI default  
- `mode=production`  

Ref: `base-vault/24 - Path to v1.4/24.30 - OS Port Validation Gate.md`
