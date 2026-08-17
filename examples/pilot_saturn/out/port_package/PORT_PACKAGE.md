# PORT_PACKAGE — atlas de port (B.A.S.E.)

> ≠ OS turnkey: `generates_os: false` · `auto_fix_complete: false` — heurísticas de MMIO sozinhas **não** bastam para gerar o sistema operacional completo.

- claim: `port_package_assist` · generates_os: **false** · auto_fix_complete: **false**
- target HAL: `hal_saturn_assist` (abstract HAL — bind concrete ISA in SOW; fossils = do-not-invent)
- source arch: `Unknown("")`

## Rewrite avoidance

| Wrap candidates | Must rewrite |
|-----------------|-------------|
| 1 | 2 |

- Use address_driver_map.yaml as the single source for source→HAL binds.
- Treat fossil_inventory.yaml as do-not-invent list before coding drivers.
- Generate host HAL stubs with `base fw` after synth — still ≠ silicon OS.
- ISA-specific asm/boot still requires human/external OS tree (TaurOS/ReactOS).

## Address / driver map

| Block | Source | HAL id | Strategy | Rewrite? |
|-------|--------|--------|----------|----------|
| RegisterFile_25c00 | `0x25c00000` | `hal_unknown_25c00000` | Stub | YES |
| Doorbell_25f00 | `0x25f00000` | `hal_gpu_25f00000` | Trap | wrap |
| RegisterFile_20100 | `0x20100000` | `hal_unknown_20100000` | Stub | YES |

## Driver checklist

- [ ] `RegisterFile_25c00` → `hal_unknown_25c00000` — REWRITE or deep HAL — insufficient observation / Unknown
- [x] `Doorbell_25f00` → `hal_gpu_25f00000` — WRAP — trap/MMU map + reuse contracts; avoid full rewrite
- [ ] `RegisterFile_20100` → `hal_unknown_20100000` — REWRITE or deep HAL — insufficient observation / Unknown

## Fossil inventory (Paleo estrato)

Summary: unobs_reg=0 unknown_block=2 unknown_purpose=2 high_ψ=0 orphan=0

- **UnknownBlock** RegisterFile_25c00 — block RegisterFile_25c00 kind=Unknown confidence=0.68
  - hint: Classify (uart/spi/…) or capture more MMIO before porting
- **UnknownPurpose** RegisterFile_25c00 — RegisterFile_25c00+0x4 purpose unknown
  - hint: Name/purpose from datasheet or Capstone before HAL bind
- **UnknownBlock** RegisterFile_20100 — block RegisterFile_20100 kind=Unknown confidence=0.68
  - hint: Classify (uart/spi/…) or capture more MMIO before porting
- **UnknownPurpose** RegisterFile_20100 — RegisterFile_20100+0x4 purpose unknown
  - hint: Name/purpose from datasheet or Capstone before HAL bind

## Artefactos

- `port_package.yaml` — pacote completo
- `address_driver_map.yaml` — binds source→HAL
- `fossil_inventory.yaml` — não inventar
- `PORT_PACKAGE.md` — este atlas

## Honesty

- ≠ OS turnkey: `generates_os: false` · `auto_fix_complete: false` — heurísticas de MMIO sozinhas **não** bastam para gerar o sistema operacional completo.
- `generates_os: false` · `auto_fix_complete: false`
- Checklist 100% ≠ OS pronto / bootável / TaurOS — só pré-requisitos descobertos (DTB/evidência).

Ref: `base-vault/24 - Path to v1.4/` · Paleo map `22.31`
