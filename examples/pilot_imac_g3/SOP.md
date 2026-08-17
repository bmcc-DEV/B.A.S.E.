# Lab SOP — iMac G3 late 2001 (fase C)

> Boot em hardware Late 2001. ReactOS + Haiku = builds **externos** (PPC retired / experimental).  
> Só após fase B verde com pelo menos `QEMU_PPC_KERNEL` (referência).

## Pré-requisitos

- [ ] Fase A: `./examples/pilot_imac_g3/run.sh` (dual port package)
- [ ] Fase B: `./examples/pilot_imac_g3/run_qemu_smoke.sh` com referência verde
- [ ] SOW assinado ([24.21](../../base-vault/24%20-%20Path%20to%20v1.4/24.21%20-%20SOW%20OS-Port%20Checklist.md))
- [ ] Imagem boot preparada (OpenBSD/LinuxPPC e/ou Haiku/ReactOS stub)

## Checklist dual OS

### Referência (LinuxPPC / OpenBSD)

- [ ] Imagem em CD / netboot / disco
- [ ] OpenFirmware: `boot cd:,\\:tbxi` ou netboot conforme lab
- [ ] Receipt preenchido (`target_os: reference`)

### Haiku (externo)

- [ ] `HAIKU_IMAGE` validada em QEMU
- [ ] Boot OF no Late 2001
- [ ] Receipt (`target_os: haiku`, `haiku_external: true`)

### ReactOS (externo)

- [ ] `REACTOS_IMAGE` validada em QEMU (se disponível)
- [ ] Boot OF no Late 2001
- [ ] Receipt (`target_os: reactos`, `reactos_external: true`)

## Receipt

Copiar `hw_boot_receipt.example.json` → preencher no lab (`production: false`).

## Proibido

- Claim port ReactOS ou Haiku turnkey pelo B.A.S.E.
- Capstone PPC REAL* nesta path
- `generates_os=true` / `production: true`
