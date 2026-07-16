# Pilot STM32 — Path to v0.5 U1 / v0.6 Capstone / v0.7 W1 SPI

Wedge sintético **STM32F103 USART1** @ `0x40013800` (APB2, Blue Pill–style).

**Não** substitui o gate RP (`examples/pilot/run.sh`).

| Campo | Valor |
|-------|-------|
| SoC alvo (mapeamento) | STM32F103C8 via `--preferred-manufacturer STMicroelectronics` |
| Peripheral | USART1 @ `0x40013800` (regs reais) |
| Analyze page | bloco @ `0x40013000` (máscara 4K do clustering) |
| Capstone (V1) | `fw.bin` AArch64 sintético (`gen_fw.py`) — **não** Thumb Cortex-M3 |
| Pins (V2) | PA9/PA10 USART1 no `stm32f103c8.yaml`; labels no draft KiCad |
| Offsets no silício | SR=`+0x00`, DR=`+0x04`, CR1=`+0x0C` relativos a `0x40013800` |
| IRQ line (trace) | `0x25` (37 decimal — USART1) |
| W1 dual SPI | SPI2 @ `0x40003800` (APB1) — SPI1 partilha página 4K com USART1 |
| X1 pins SPI2 | PB13 SCK / PB14 MISO / PB15 MOSI (+ PB12 NSS) no draft sch |

## Como rodar

```bash
cargo build -p base-cli
# regenerar blob Capstone (opcional):
python3 examples/pilot_stm32/gen_fw.py
./examples/pilot_stm32/run.sh          # USART-only (gate opt-in)
./examples/pilot_stm32/run_w1_spi.sh   # USART + SPI2 (W1; não substitui run.sh)
```

Smoke inclui:
1. `analyze --disasm` — Capstone resolve USART1 sem traces  
2. `analyze --mmio-traces` — path feliz design/synth  
3. **W2 goldens** — `diff` event-graph + prove fields vs `expected/`

## Goldens (`expected/`)

| Arquivo | Papel |
|---------|-------|
| `event_graph.dot` / `.mmd` | Causal graph USART (smoke `diff`) |
| `proof_report.golden.json` | Prove simbólico estável (sem `smt_lib`) |
| `hardware_spec.fields.yaml` | Allowlist de campos HardwareSpec |
| `CASE_SUMMARY.template.md` | Campos estáveis do resumo |

## W1 — dual USART + SPI2

| Campo | Valor |
|-------|-------|
| USART1 | `0x40013800` → page `0x40013000` |
| SPI2 | `0x40003800` → page `0x40003000` |
| Classify | `0x40013000=uart,0x40003000=spi` |
| Porquê SPI2 | SPI1 @ `0x40013000` colide com USART1 na mesma página 4K |
| IRQ SPI2 | `0x24` (36) |

## Arquivos

| Arquivo | Papel |
|---------|-------|
| `gen_fw.py` | Gera `fw.bin` AArch64 @ página USART1 |
| `fw.bin` | Blob Capstone (sintético) |
| `mmio.json` | Acessos MMIO USART1 |
| `mmio_usart_spi.json` | Dual USART+SPI2 (W1) |
| `contracts.yaml` / `trace.csv` | Prove + replay USART |
| `contracts_spi.yaml` / `trace_spi.csv` | Prove + replay SPI2 |
| `pilot.bsl` / `pilot_spi.bsl` | BIR |
| `expected/` | Goldens W2 (verificados, não sobrescritos) |
| `SHA256SUMS` / `SHA256SUMS.w1` | Integridade |
| `run.sh` | Smoke USART opt-in + goldens |
| `run_w1_spi.sh` | Smoke dual W1 opt-in |
| `out/` / `out_w1_spi/` | Gerado (gitignored) |
