# Honesty — psx-saturn-bridge (caminho máquina)

| Claim | Valor | Nota |
|-------|-------|------|
| `machine_closed_loop` | **true** | Driver corre EXE→projeto SRL sozinho |
| `machine_emits_project` | **true** | `out/machine_port/*/…` |
| `human_required` | **false** | Humano ≠ passo do pipeline default |
| `game_fidelity_complete` | **false** | Cobertura GP0/GTE/MIPS ainda subset |
| `auto_ports_games` | **false** | ≠ claim de jogo comercial fiel |
| `runs_on_saturn` | **false** | Núcleo B.A.S.E. sem guest runtime VDP |
| `ships_bios_or_discs` | **false** | |

## Modelo

```text
PS-X EXE  →  machine_port.py  →  projeto SaturnRingLib (makefile+src)
                 │
                 ├─ PadMap (YAML)
                 ├─ GP0→SRL tabela
                 ├─ inventário MIPS
                 └─ receipt JSON
```

Incrementar fidelidade = **expandir tabelas/tradutores na máquina**, não reintroduzir checklist humano no caminho crítico.

## O que a máquina já faz

1. Parse PS-X EXE + fixture CI  
2. Emite árvore SRL completa  
3. Verifica APIs SRL nos headers  
4. Opcional: `base recomp encode --target sh2`  
5. Receipt com `human_required=false`

## O que ainda é subset (máquina, não humano)

Corpos reais de GP0 textured/tri, GTE→Math, lift MIPS completo → SH-2, ISO/CD assets.  
Estão listados em `MACHINE_RECEIPT.json` → `next_machine_increments`.
