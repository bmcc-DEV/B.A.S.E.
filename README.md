# B.A.S.E. — Behavioral ASIC Synthesis Engine

[![CI](https://github.com/Eternet-Mycelium-Network/B.A.S.E./actions/workflows/ci.yml/badge.svg)](https://github.com/Eternet-Mycelium-Network/B.A.S.E./actions/workflows/ci.yml)

**Transform hardware behavior into new PCB + firmware.**

> *"O que este hardware faz?" em vez de "Como este hardware foi implementado?"*

11 crates · 12 fases · 80+ testes · Pipeline end-to-end

---

## Pipeline

```bash
Firmware → analyze → synth → pcb + fw → check → evolve
```

| Step | Command | Output |
|------|---------|--------|
| 1. Analyze | `base analyze firmware.bin --disasm` | `hardware_spec.yaml` + `behavior_graph.dot` + `event_graph.dot` |
| 2. BIR | `base bir spec.bir.yaml --validate` | BIR validation + Graphviz DOT |
| 3. Synthesize | `base synth hardware_spec.yaml` | `synthesized_spec.yaml` |
| 4. PCB | `base pcb synthesized_spec.yaml --drc` | `.kicad_sch`, `.kicad_pcb`, `bom.csv` |
| 5. Firmware | `base fw synthesized_spec.yaml --zephyr` | `bootloader.c`, `hal_mmio.c`, `drivers.c` |
| 6. Validate | `base check spec trace.csv` | `validation_report.html` (com gráficos SVG) |
| 7. Evolve | `base evolve spec` | `evolution_plan.md` |
| 8. Reconstruct | `base reconstruct spec.yaml --threshold 0.9` | Loop recursivo até convergir |
| **All** | `base pipeline firmware.bin --disasm` | Everything above |

## Quick Start

### Pré-requisitos

```bash
rustup toolchain install stable
git clone https://github.com/Eternet-Mycelium-Network/B.A.S.E..git
cd B.A.S.E.
cargo build -p base-cli
```

### Análise com disassembly real

```bash
base analyze firmware.bin --disasm --dot -o output/
# → 520 funções desassembladas, 35K instruções, 757 MMIO candidates
# → behavior_graph.dot (estrutural) + event_graph.dot (causal/temporal)
```

### Pipeline completa

```bash
base pipeline firmware.bin --disasm --trace traces.csv --target rp2350 -o output/
```

### Loop de refinamento recursivo

```bash
base reconstruct output/01_analyze/hardware_spec.yaml --threshold 0.9 --max-iterations 10
```

### BIR — Behavioral IR

```bash
# Validar BIR
base bir device.bir.yaml --validate

# Converter para HardwareSpec legado
base bir device.bir.yaml --to-legacy

# Exportar grafo
base bir device.bir.yaml --dot
```

### BSL — Behavior Specification Language

```bsl
device GPU @ 0x10000000 {
    registers {
        CONTROL @ 0x00: rw = 0;
        STATUS  @ 0x04: ro;
    }
    events {
        DMA_START: write CONTROL[0] = 1;
    }
    interrupts {
        IRQ_GPU: level high;
    }
    timing {
        dma_setup: 100ns..400ns;
    }
    contract {
        must_occur_before: DMA_START -> DMA_COMPLETE;
        window: 10us;
    }
}
```

### HIL Probe (captura de hardware real)

```bash
# Gerar firmware para RP2350
base hil probe-fw > hil_probe/src/main.rs

# Conectar e capturar
base hil capture --vid 0xCAFE --pid 0x4007 --output trace.pcap

# Validar contra hardware real
base check spec.yaml trace.pcap --mode hil
```

## Casos de Uso

### Preservação de Hardware
ASICs de consoles retrô (Amiga CD32, SNES, Mega Drive) → PCB compatível com componentes disponíveis.

### Modernização de Sistemas Legados
Power Mac G5, DEC Alpha → hardware moderno (Cortex-A, DDR5, NVMe, USB-C) mantendo compatibilidade.

### Fork de Hardware
Xbox 360, PlayStation 3 → reconstruir comportamento em FPGA + MCU.

## Arquitetura

```
┌──────────────────────────────────────────────────────┐
│                    base-cli (CLI)                     │
├──────────────────────────────────────────────────────┤
│  base-bir  │  base-bsl  │  base-core                 │
│  (IR)      │  (lang)    │  (inference + solver)      │
│  base-pcb  │  base-fw   │  base-check  │  base-hil   │
│  (KiCad)   │  (FW)      │  (validate)  │  (probe)    │
│  base-evolve  │  base-learn                            │
│  (evolution)  │  (ML models)                          │
├──────────────────────────────────────────────────────┤
│              specterprobe (disassembly)               │
└──────────────────────────────────────────────────────┘
```

## Crates

| Crate | Descrição | Status |
|-------|-----------|--------|
| `specterprobe` | Disassembly ARM64 com Capstone, CFG, MMIO scan | ✅ |
| `base-bir` | Behavioral IR — tipos, validador, contratos temporais | ✅ |
| `base-bsl` | Behavior Specification Language — parser PEG + compiler | ✅ |
| `base-core` | Inferência, Knowledge Graph, Digital Twin, Feedback Loop, Solver | ✅ |
| `base-pcb` | Gerador KiCad — S-expression, schematic, BOM, PCB, DRC | ✅ |
| `base-fw` | Firmware sintético — bootloader, HAL MMU, drivers, devicetree, Zephyr | ✅ |
| `base-check` | Validação — trace Saleae/PCAP/JSON, comparator, HTML report | ✅ |
| `base-evolve` | Evolução — bottleneck analysis, trade-offs, migration plans | ✅ |
| `base-hil` | HIL Cluster — RP2350 probe firmware, host agent | ✅ |
| `base-learn` | Foundation Models — dataset, classifier (rule-based + ONNX) | ✅ |
| `base-cli` | CLI unificada — 8 subcomandos | ✅ |

## B.A.S.E. v2 — 12 Fases Completas

```
Fase  0  ████████████████████████████ 100%  base-bir + base-bsl
Fase  1  ████████████████████████████ 100%  BIR types + validator
Fase  2  ████████████████████████████ 100%  Contratos temporais
Fase  3  ████████████████████████████ 100%  Knowledge Graph (GraphML/Neo4j)
Fase  4  ████████████████████████████ 100%  Genome DB (51 componentes + timing)
Fase  5  ████████████████████████████ 100%  Constraint Solver (ILP + Z3 + heuristic)
Fase  6  ████████████████████████████ 100%  HIL Cluster (RP2350 probe)
Fase  7  ████████████████████████████ 100%  Multi-Source Learning
Fase  8  ████████████████████████████ 100%  Digital Twin (BIR interpreter)
Fase  9  ████████████████████████████ 100%  Closed Feedback Loop
Fase 10  ████████████████████████████ 100%  Evolution Engine
Fase 11  ████████████████████████████ 100%  BSL Language
Fase 12  ████████████████████████████ 100%  Foundation Models
```

## Testes

```bash
cargo test --workspace
# 80+ testes em 11 crates
```

## Licença

AGPL-3.0
