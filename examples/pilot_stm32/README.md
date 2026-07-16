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
| Y1 pins I2C1 | PB6 SCL / PB7 SDA no draft sch |

## Como rodar

```bash
cargo build -p base-cli
# regenerar blob Capstone (opcional):
python3 examples/pilot_stm32/gen_fw.py
./examples/pilot_stm32/run.sh          # USART-only (gate opt-in)
./examples/pilot_stm32/run_w1_spi.sh   # USART + SPI2 (W1; não substitui run.sh)
./examples/pilot_stm32/run_x3_i2c.sh   # USART + I2C1 (X3; não substitui run.sh)
./examples/pilot_stm32/run_y3_triple.sh # USART + SPI2 + I2C1 (Y3; não substitui os acima)
```

Smoke inclui:
1. `analyze --disasm` — Capstone resolve USART1 sem traces  
2. `analyze --mmio-traces` — path feliz design/synth  
3. **W2 goldens** — `diff` event-graph + prove fields vs `expected/`
4. **Y2 goldens I2C** — `diff` vs `expected_i2c/` (nunca overwrite)
5. **Y3 triple** — 3 páginas classify Evidence→Design→PCB

## Goldens (`expected/`)

| Arquivo | Papel |
|---------|-------|
| `event_graph.dot` / `.mmd` | Causal graph USART (smoke `diff`) |
| `proof_report.golden.json` | Prove simbólico estável (sem `smt_lib`) |
| `hardware_spec.fields.yaml` | Allowlist de campos HardwareSpec |
| `CASE_SUMMARY.template.md` | Campos estáveis do resumo |

## Goldens I2C (`expected_i2c/` — Y2)

| Arquivo | Papel |
|---------|-------|
| `event_graph.dot` / `.mmd` | Causal graph I2C1 (smoke `diff`) |
| `proof_report.golden.json` | Prove I2C1 estável (sem `smt_lib`) |

## W1 — dual USART + SPI2

| Campo | Valor |
|-------|-------|
| USART1 | `0x40013800` → page `0x40013000` |
| SPI2 | `0x40003800` → page `0x40003000` |
| Classify | `0x40013000=uart,0x40003000=spi` |
| Porquê SPI2 | SPI1 @ `0x40013000` colide com USART1 na mesma página 4K |
| IRQ SPI2 | `0x24` (36) |

## X3 — dual USART + I2C1

| Campo | Valor |
|-------|-------|
| I2C1 | `0x40005400` → page `0x40005000` |
| Classify | `0x40013000=uart,0x40005000=i2c` |
| IRQ I2C1_EV | `0x1f` (31) |
| Y1 pins I2C1 | PB6 SCL / PB7 SDA no draft sch (`NOT FABRICABLE`) |
| Y2 goldens | `expected_i2c/` — event-graph + prove (`diff`) |
| Smoke | `run_x3_i2c.sh` |

## Y3 — triple USART + SPI2 + I2C1

| Campo | Valor |
|-------|-------|
| Classify | `0x40013000=uart,0x40003000=spi,0x40005000=i2c` |
| Fixture | `mmio_usart_spi_i2c.json` |
| Smoke | `run_y3_triple.sh` |
| PCB | PA9/10 + PB13/14/15 + PB6/7 (`NOT FABRICABLE`) |

## Arquivos

| Arquivo | Papel |
|---------|-------|
| `gen_fw.py` | Gera `fw.bin` AArch64 @ página USART1 |
| `fw.bin` | Blob Capstone (sintético) |
| `mmio.json` | Acessos MMIO USART1 |
| `mmio_usart_spi.json` | Dual USART+SPI2 (W1) |
| `mmio_usart_i2c.json` | Dual USART+I2C1 (X3) |
| `mmio_usart_spi_i2c.json` | Triple USART+SPI2+I2C1 (Y3) |
| `contracts.yaml` / `trace.csv` | Prove + replay USART |
| `contracts_spi.yaml` / `trace_spi.csv` | Prove + replay SPI2 |
| `contracts_i2c.yaml` / `trace_i2c.csv` | Prove + replay I2C1 |
| `pilot.bsl` / `pilot_spi.bsl` / `pilot_i2c.bsl` | BIR |
| `expected/` | Goldens W2 USART (verificados, não sobrescritos) |
| `expected_i2c/` | Goldens Y2 I2C1 (verificados, não sobrescritos) |
| `SHA256SUMS` / `SHA256SUMS.w1` / `SHA256SUMS.x3` / `SHA256SUMS.y3` | Integridade |
| `run.sh` | Smoke USART opt-in + goldens |
| `run_w1_spi.sh` | Smoke dual W1 opt-in |
| `run_x3_i2c.sh` | Smoke dual X3 opt-in |
| `run_y3_triple.sh` | Smoke triple Y3 opt-in |
| `out/` / `out_w1_spi/` / `out_x3_i2c/` / `out_y3_triple/` | Gerado (gitignored) |
