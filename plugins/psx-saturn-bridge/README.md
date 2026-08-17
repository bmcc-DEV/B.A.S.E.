# Plugin — PSX → Saturn (driver máquina)

Caminho default: **a máquina** lê PS-X EXE e emite projeto [SaturnRingLib](https://srl.reye.me/) — sem passo humano.

```bash
./plugins/psx-saturn-bridge/scripts/run_machine.sh
./plugins/psx-saturn-bridge/scripts/run_machine.sh --exe jogo.EXE --name GamePort
```

| Peça | Papel |
|------|--------|
| `tools/machine_port.py` | Driver autónomo |
| `vendor/psxrecomp/` | Referência de arquitetura recomp (PolyForm NC) |
| `SaturnRingLib/` | Symlink destino de build |
| `out/machine_port/` | Projeto gerado + `MACHINE_RECEIPT.json` |

## Honesty (curto)

- `machine_closed_loop=true` · `human_required=false`  
- `game_fidelity_complete=false` (subset GP0/GTE — expandir tabelas)  
- `runs_on_saturn=false` no núcleo B.A.S.E.

Detalhe: [`HONESTY.md`](HONESTY.md) · [`AUTOMATION.md`](AUTOMATION.md)
