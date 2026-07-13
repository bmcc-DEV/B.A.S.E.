# B.A.S.E. — Behavioral ASIC Synthesis Engine

[![CI](https://github.com/Eternet-Mycelium-Network/B.A.S.E./actions/workflows/ci.yml/badge.svg)](https://github.com/Eternet-Mycelium-Network/B.A.S.E./actions/workflows/ci.yml)

**Transform hardware behavior into new PCB + firmware.**

> *"O que este hardware faz?" em vez de "Como este hardware foi implementado?"*

## Pipeline

```bash
firmware.bin → analyze → synth → pcb + fw → check → evolve
```

| Step | Command | Output |
|------|---------|--------|
| 1. Analyze | `base analyze firmware.bin` | `hardware_spec.yaml` + `behavior_graph.dot` |
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

# Apenas análise e visualização
base analyze firmware.bin --dot -o analyze/
dot -Tpng -O analyze/behavior_graph.dot  # requer graphviz

# Apenas PCB a partir de spec existente
base synth analyze/hardware_spec.yaml -o synth/
base pcb synth/synthesized_spec.yaml --drc -o pcb/
```

### Formatos de trace suportados

- **Saleae CSV**: `Time[s], Channel, Type, Data` (ex: `0.001, CH0, WRITE, 0x1000=0x01`)
- **Custom JSON**: `DeviceTrace` format (serializável com `serde_json`)
- **Wireshark PCAP**: `.pcap` / `.cap` — parsing de USB URB packets

### Exemplo Amiga CD32

```bash
# 1. Analisar firmware
base analyze examples/amiga_cd32/hardware_spec.yaml --dot -o analyze/

# 2. Sintetizar hardware mapping
base synth analyze/hardware_spec.yaml --component-db base-core/component_db -o synth/

# 3. Gerar PCB
base pcb synth/synthesized_spec.yaml -o pcb/

# 4. Gerar firmware
base fw synth/synthesized_spec.yaml --zephyr -o fw/

# 5. Validar
base check synth/synthesized_spec.yaml traces.csv -o check/

# 6. Sugerir upgrades
base evolve synth/synthesized_spec.yaml -o evolve/
```

## Visualização: Behavior Graph

O B.A.S.E. pode exportar o grafo comportamental inferido no formato Graphviz DOT:

```bash
base analyze firmware.bin --dot -o analyze/
dot -Tpng -O analyze/behavior_graph.dot
```

Isso produz um diagrama como:

```
┌──────┐   MMIO    ┌─────────┐  +0x00  ┌───────────┐
│ CPU  │──────────►│  GPU    │────────►│  control  │
│      │           │0x10000000│  +0x04  │  buf_addr │
│400MHz│           └─────────┘  +0x08  │  length   │
│PPC   │           ┌─────────┐         └───────────┘
│      │──────────►│  DMA    │
└──────┘           │0x10020000│
   │               └─────────┘
   │      IRQ      ┌──────────┐
   │◄─────────────│ IRQ Ctrl│
                  └──────────┘
```

## Casos de Uso

### Preservação de Hardware

ASICs de consoles retrô (Amiga CD32, SNES, Mega Drive) param de funcionar → B.A.S.E. produz uma PCB compatível com componentes disponíveis.

### Modernização de Sistemas Legados

Power Mac G5, DEC Alpha, servidores Sparc → B.A.S.E. mapeia o comportamento para hardware moderno (Cortex-A, DDR5, NVMe, USB-C) mantendo compatibilidade com software original.

### Fork de Hardware

Xbox 360, PlayStation 3 → Se o ASIC morre, o B.A.S.E. permite reconstruir o comportamento em FPGA + MCU.

## Desafios Técnicos

### Gerenciamento de Latência na HAL Sutética

Um dos maiores desafios do B.A.S.E. é manter a compatibilidade temporal com o hardware original. Firmware antigo frequentemente contém **polling loops com timeouts hardcode** que assumem uma latência específica do ASIC original.

**Problema:** Se o novo hardware for muito mais rápido que o original, o polling loop pode ler "pronto" antes do periférico estar realmente pronto. Se for mais lento, o timeout estoura.

**Solução implementada no `base-fw`:**

O gerador de firmware analisa o perfil de temporização original (`TimingProfile` com `min_ns`, `max_ns`, `avg_ns` por operação) e gera código C com compensação:

```c
// gerado automaticamente
static inline void timing_compensate(const char *block, uint32_t op) {
    // GPU wake: original 3us, novo 0.5us → delay de 2.5us
    if (block == BLOCK_GPU && op == OP_WAKE) {
        busy_wait_us(2.5);
    }
    // Audio DMA: precisa de timing preciso para sample rate
    if (block == BLOCK_AUDIO && op == OP_DMA) {
        hw_timer_delay(AUDIO_SAMPLE_INTERVAL);
    }
}
```

O sistema também gera uma tabela `original_timing[]` com todos os perfis medidos para referência durante validação.

### Hardware-in-the-Loop (HIL) Probe

Para hardwares muito obscuros onde gerar traces manualmente é inviável, o B.A.S.E. tem espaço arquitetural para um **probe de hardware dedicado**. Um microcontrolador versátil (RP2040/RP2350) ou FPGA leve conectado aos barramentos legados poderia:

1. **Escravizar** o barramento alvo (paralelo 8/16/32 bits, ISA, PCI, PCMCIA)
2. **Capturar** ciclos de leitura/escrita com timestamp preciso
3. **Traduzir** para o formato `DeviceTrace` que o `base check` consome
4. **Opcionalmente injetar** estímulos controlados (memory scrubbing, MMIO fuzzing)

O formato de saída já é compatível — o `base check` aceita CSV e JSON, e o parser PCAP entende capturas USB de analisadores lógicos.

```
┌──────────────┐   barramento   ┌────────────┐  USB/SPI   ┌──────────┐
│ Hardware     │◄──────────────►│ RP2350     │───────────►│ base     │
│ Original     │   (ISA/PCI/    │ Probe      │  capture   │ check    │
│ (morbundo)   │    parallel)   │            │  .pcap/.csv│          │
└──────────────┘                └────────────┘            └──────────┘
```

### Validação com Múltiplos Modos

O `base check` oferece três modos de validação:

| Modo | Descrição | Quando usar |
|------|-----------|-------------|
| **Estático** | Verifica consistência do `HardwareSpec` sem execução | Após `analyze` |
| **Replay Simulado** | Executa trace original contra o modelo comportamental | Antes de fabricar PCB |
| **Replay HW** | Executa trace contra hardware real | Após fabricar protótipo |

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
| `base-core` | Inferência comportamental + mapeamento hardware + Graphviz DOT | ✅ |
| `base-pcb` | Gerador KiCad (schematic, BOM, PCB layout, templates, DRC) | ✅ |
| `base-fw` | Firmware sintético (bootloader, HAL c/ timing, drivers, Zephyr) | ✅ |
| `base-check` | Validação (trace Saleae/PCAP/JSON, comparator, HTML/JSON report) | ✅ |
| `base-evolve` | Motor de evolução (bottleneck analysis, migration plans) | ✅ |
| `base-cli` | CLI unificada com pipeline end-to-end e export DOT | ✅ |

## Licença

AGPL-3.0
