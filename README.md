# B.A.S.E. — Behavioral ASIC Synthesis Engine

[![CI](https://github.com/bmcc-DEV/specterprobe/actions/workflows/ci.yml/badge.svg)](https://github.com/bmcc-DEV/specterprobe/actions/workflows/ci.yml)

**Transform hardware behavior into new PCB + firmware.**

> *"O que este hardware faz?" em vez de "Como este hardware foi implementado?"*

## Pipeline

```bash
firmware.bin → analyze → synth → pcb + fw → check → evolve
```

| Step | Command | Output |
|------|---------|--------|
| 1. Analyze | `base analyze firmware.bin` | `hardware_spec.yaml` |
| 2. Synthesize | `base synth hardware_spec.yaml` | `synthesized_spec.yaml` |
| 3. PCB | `base pcb synthesized_spec.yaml` | `.kicad_sch`, `.kicad_pcb`, `bom.csv` |
| 4. Firmware | `base fw synthesized_spec.yaml` | `bootloader.c`, `hal_mmio.c`, drivers |
| 5. Validate | `base check spec trace.csv` | `validation_report.html` |
| 6. Evolve | `base evolve spec` | `evolution_plan.md` |
| **All** | `base pipeline firmware.bin` | Everything above |

## Quick Start

### Pré-requisitos

```bash
rustup toolchain install stable
cargo install base-cli  # or: cargo build -p base-cli
```

### Pipeline completa

```bash
# Pipeline completa com todos os 6 passos
base pipeline firmware.bin --trace traces.csv --target rp2350 -o output/

# Apenas análise e PCB
base analyze firmware.bin -o analyze/
base synth analyze/hardware_spec.yaml -o synth/
base pcb synth/synthesized_spec.yaml --drc -o pcb/
```

### Formatos de trace suportados

- **Saleae CSV**: `Time[s], Channel, Type, Data` (ex: `0.001, CH0, WRITE, 0x1000=0x01`)
- **Custom JSON**: `DeviceTrace` format (serializável com `serde_json`)
- **Wireshark PCAP**: `.pcap` / `.cap` — parsing de USB URB packets

### Exemplo Amiga CD32

```bash
# 1. Extrair Kickstart ROM do CD32
binwalk -e kickstart.rom

# 2. Analisar firmware
base analyze kickstart.rom -o analyze/

# 3. Sintetizar hardware mapping
base synth analyze/hardware_spec.yaml --component-db base-core/component_db -o synth/

# 4. Gerar PCB
base pcb synth/synthesized_spec.yaml -o pcb/

# 5. Gerar firmware
base fw synth/synthesized_spec.yaml --zephyr -o fw/

# 6. Validar
base check synth/synthesized_spec.yaml traces/amiga_cd32.csv -o check/

# 7. Sugerir upgrades
base evolve synth/synthesized_spec.yaml -o evolve/
```

## Cenas de Uso

### Preservação de Hardware

ASICs de consoles retrô (Amiga CD32, SNES, Mega Drive) param de funcionar → B.A.S.E. produz uma PCB compatível com componentes disponíveis.

### Modernização de Sistemas Legados

Power Mac G5, DEC Alpha, servidores Sparc → B.A.S.E. mapeia o comportamento para hardware moderno (Cortex-A, DDR5, NVMe, USB-C) mantendo compatibilidade com software original.

### Fork de Hardware

Xbox 360, PlayStation 3 → Se o ASIC morre, o B.A.S.E. permite reconstruir o comportamento em FPGA + MCU.

## Arquitetura

```
┌─────────────────────────────────────────────┐
│  base-cli (CLI unificada)                   │
├─────────────────────────────────────────────┤
│  base-core    base-pcb    base-fw           │
│  (inferência) (KiCad)     (FW sintético)    │
│  base-check   base-evolve                   │
│  (validação)  (evolução)                    │
├─────────────────────────────────────────────┤
│  SpecterProbe (análise de firmware)         │
└─────────────────────────────────────────────┘
```

## Crates

| Crate | Descrição | Status |
|-------|-----------|--------|
| `specterprobe` | Análise de firmware ARM64 (disasm, CFG, MMIO, behavioral) | ✅ |
| `base-core` | Inferência comportamental + mapeamento hardware | ✅ |
| `base-pcb` | Gerador KiCad (schematic, BOM, PCB layout, templates) | ✅ |
| `base-fw` | Firmware sintético (bootloader, HAL, drivers, Zephyr) | ✅ |
| `base-check` | Validação (trace parser, comparator, HTML/JSON report) | ✅ |
| `base-evolve` | Motor de evolução (bottleneck analysis, migration plans) | ✅ |
| `base-cli` | CLI unificada com pipeline end-to-end | ✅ |

## Licença

AGPL-3.0
