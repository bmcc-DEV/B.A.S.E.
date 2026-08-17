# Honesty — Sega Saturn pilot

B.A.S.E. **descobre** comportamento de hardware (MMIO/contratos/atlas).  
**Não** porta jogos. **Não** corre no Saturn.

| Flag | Valor |
|------|-------|
| `runs_on_saturn` | `false` |
| `ports_games` | `false` |
| `generates_os` | `false` |
| `vdp_complete` | `false` |
| `static_recomp_complete` | `false` |
| `production` | `false` |

## Claims permitidos

- Extrair contratos VDP1/VDP2/SMPC a partir de traces (synth ou lab)
- Entregar `port package` / HAL assist para equipas de port
- Emit/encode SH-2 subset via `base recomp` (CPU only)

## Claims proibidos

- “B.A.S.E. corre jogos PS1/x86 no Saturn”
- “VDP1/VDP2 runtime completo”
- Juntar peshit/psxrecomp = Saturn turnkey

Ver: [PORT_SHORTCUT.md](PORT_SHORTCUT.md) · `base recomp runtime`
