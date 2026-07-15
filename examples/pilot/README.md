# Pilot fixture — Path to Real (R0)

Wedge sintético de **UART MMIO estilo PL011** em `0x40034000`.

Este diretório **não** reivindica um SoC real. O `fw.bin` é placeholder;
a fonte da verdade comportamental é `mmio.json` + `trace.csv` + `contracts.yaml`.

## Wedge decidido (R0)

| Campo | Valor |
|-------|-------|
| Classe | MCU / peripheral MMIO (ARM-like) |
| Peripheral | UART @ `0x40034000` |
| Por quê | Endereços estáveis, tipos Saleae (WRITE/READ/IRQ), encaixa Capstone/heurística |
| Fora de escopo | GPU, high-speed SerDes, Power Mac / Xbox / Alpha |
| R6 | Trocar `fw.bin` por firmware real no mesmo padrão MMIO |

Vault: `base-vault/12 - Path to Real/12.16 - Sprint R6 Pilot.md`

## Arquivos

| Arquivo | Papel |
|---------|-------|
| `fw.bin` | Placeholder binário (pequeno) |
| `mmio.json` | Acessos MMIO (ground truth para `analyze --mmio-traces`) |
| `pilot.bsl` | Spec BSL → BIR + contratos Saleae |
| `trace.csv` | Trace pass (Saleae-like) |
| `trace_fail.csv` | Trace com latência acima do contrato |
| `contracts.yaml` | Contratos manuais (SAT) |
| `contracts.unsat.yaml` | Fixture UNSAT para `prove` |
| `run.sh` | Smoke E2E (bir → analyze → prove → replay → event-graph → fw) |
| `out/` | Gerado (gitignored) |
| `expected/` | Goldens (fields, schema, event_graph) |

## Como rodar

```bash
cargo build -p base-cli
chmod +x examples/pilot/run.sh
./examples/pilot/run.sh
```

Comandos avulsos: ver README raiz do repositório.

## Esperado no smoke

- `out/analyze/hardware_spec.yaml` + `evidence_db.yaml`
- `out/design/reference_design.yaml` com CPU ≠ TBD
- `out/prove/proof_report.json` com contratos SAT
- `out/prove_unsat/` com `proved: false`
- `out/fw/firmware_host` exit 0

## Limitações honestas

- Parser Saleae só reconhece WRITE / READ / IRQ (não `dma_*` via CSV)
- PCB não faz parte do `run.sh` de propósito
- Classificação UART vem de `--classify uart` + traces, não de magic ML
