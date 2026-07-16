# HIL Lab SOP (Gate A3) — template

> Copiar/adaptar ao lab do Cliente. **Não** é flash de produção turnkey.

## 1. Operador

- Nome / contato: _______________
- Só flasheia com SOW §HIL assinado (Gate A5).

## 2. Pré-checks

```bash
# Default CI: A1/A2 BLOCK
base hil lab-status --sop examples/hil_lab/SOP.md -o /tmp/hil_gate/

# Lab rehearsal A1+A2 (Detected offline + programmer; A5 ainda aberto):
cargo build -p base-cli --features hil_programmer
export BASE_HIL_ALLOW_FLASH=1
export BASE_HIL_PROGRAMMER_CMD='test -f {image}'   # ou picotool load {image}
base hil lab-status --sop examples/hil_lab/SOP.md --mock-detected -o /tmp/hil_gate/
# → A1/A2 GREEN; lab_assist_ready=false até --sow-signed
```

**A1 real (USB):** build com `--features hil_usb,hil_programmer` + probe VID:PID — sem `--mock-detected`.

## 3. Dry-run (obrigatório antes de silício)

```bash
base hil flash firmware.bin --mock-flash -o /tmp/hil/
# mode=mock_dry_run — zero silício
```

## 4. Lab-assist (só se Gate A verde)

```bash
export BASE_HIL_ALLOW_FLASH=1
export BASE_HIL_PROGRAMMER_CMD='picotool load {image}'   # exemplo
# build com --features hil_programmer[,hil_usb] conforme lab
base hil flash firmware.bin --mock-detected -o /tmp/hil/   # rehearsal
# ou sem --mock-detected se USB Detected
# mode=experimental_external_cmd — ainda ≠ "production"
```

Smoke: `./examples/hil_lab/run_hil_lab_assist.sh`

## 5. Rollback / log

- Guardar `hil_flash_receipt.json` + hash do binário.
- Se falhar: reverter imagem anterior documentada no SOW.

## 6. Proibido

- Flash no CI default  
- Claim `production` / SaaS plug-and-flash  
- Flash sem Detected / sem ALLOW_FLASH  
- Passar `--sow-signed` sem contrato  

Ref: `base-vault/22 - Path to v1.2/22.30 - SOW Industrial Gate.md`
