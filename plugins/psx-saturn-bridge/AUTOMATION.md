# Automação — caminho máquina (sem IA, sem humano no loop)

Default: **`scripts/run_machine.sh`**. Checklist humano saiu do caminho crítico.

## Pipeline

```bash
./plugins/psx-saturn-bridge/scripts/run_machine.sh
# EXE real:
./plugins/psx-saturn-bridge/scripts/run_machine.sh --exe /path/to/SLUS_XXX.XX --name MyPort
```

| Tool | Papel |
|------|--------|
| `tools/machine_port.py` | **Driver**: EXE → projeto SRL + receipt |
| `tools/psx_exe.py` | Parser PS-X EXE + fixture |
| `tools/verify_srl_apis.py` | Gate: símbolos SRL existem |
| `tools/merge_atlas.py` | Evidência MMIO B.A.S.E. (opcional) |
| `tools/emit_from_map.py` | Biblioteca de emit (usada pelo driver) |

## Como a fidelidade sobe (só máquina)

1. Expandir `GP0_TABLE` em `machine_port.py` com corpos VDP1/Scene2D reais  
2. Tabela GTE → `SRL::Math`  
3. Lift MIPS leaf → SIR → `base recomp encode --target sh2`  
4. Walker ISO → lista `SRL::Cd` + copy/reencode por extensão  
5. Overlay capture inventário → dispatch table gerada  

Cada incremento = patch de tabela/parser + re-run `run_machine.sh`. Sem etapa “humano preenche checklist”.

## Gates

- `human_required == false`  
- `machine_closed_loop == true`  
- `game_fidelity_complete == false` até cobertura de opcodes atingir critério medido (contador no receipt)

Receipt: `out/machine_port/<Name>/MACHINE_RECEIPT.json`
