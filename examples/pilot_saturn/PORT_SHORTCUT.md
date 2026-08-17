# Port shortcut — PS1 / x86 90s → Sega Saturn

## Ideia

Usar o B.A.S.E. para **cortar caminho na descoberta de hardware** (o que VDP/SMPC/CD fazem),  
depois o port do jogo é humano + runtime/recomp **externos** (peshit, psxrecomp, Jo Engine, etc.).

```text
dumps/traces  →  analyze / prove / port package  →  atlas HAL Saturn
                                                      ↓
         plugins/psx-saturn-bridge (psxrecomp ref + SaturnRingLib) + base-recomp SH-2 smoke
                                                      ↓
                                         jogo portado (equipa / tree SRL externa)
```

Plugin isolado: [`plugins/psx-saturn-bridge/`](../../plugins/psx-saturn-bridge/) — mapa PS1→SRL, scaffold, Cursor rules. `runs_on_saturn=false`.

## Fontes → destino

| Fonte | Destino | Papel B.A.S.E. |
|-------|---------|----------------|
| PS1 | Saturn | Mapear VDP/SMPC vs o que o port precisa emular/traduzir |
| x86 90s | Saturn | Idem + gaps Win32/DOS (fora do wedge A) |

## Fases neste piloto

| Fase | Script | Entrega |
|------|--------|---------|
| A | `./run.sh` | HardwareSpec + contratos + `hal_saturn_assist` |
| B | `./run_recomp_smoke.sh` | SH-2 encode smoke (`runs_on_saturn=false`) |
| C | [SOP.md](SOP.md) | Dumps reais / Mednafen traces |

Dreamcast (x86/PS2 → DC) = wedge irmão futuro — mesmo padrão, SH-4/PowerVR.
