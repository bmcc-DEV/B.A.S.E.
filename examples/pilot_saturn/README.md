# Pilot — Sega Saturn (HW discovery assist)

Atalho B.A.S.E.: **descobrir como o hardware Saturn funciona** (VDP1/VDP2/SMPC) para equipas portarem PS1 / x86 90s.  
**≠** jogos a correr · **≠** peshit/psxrecomp turnkey · `runs_on_saturn=false`.

| Fase | Script |
|------|--------|
| A | `./run.sh` — analyze → prove → reconstruct → `port package` |
| B | `./run_recomp_smoke.sh` — SH-2 lift/encode smoke |
| C | [SOP.md](SOP.md) — traces reais |
| D | [`plugins/psx-saturn-bridge/`](../../plugins/psx-saturn-bridge/) — **driver máquina** `run_machine.sh` (EXE→projeto SRL) |

Docs: [HONESTY.md](HONESTY.md) · [PORT_SHORTCUT.md](PORT_SHORTCUT.md)

```bash
./examples/pilot_saturn/run.sh
./examples/pilot_saturn/run_recomp_smoke.sh
```

Vault: [[06.05 Sega Saturn]] · recomp: `base recomp runtime`
