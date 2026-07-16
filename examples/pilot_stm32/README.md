# Pilot STM32 — Path to v0.5 U1 / v0.6 V1 Capstone

Wedge sintético **STM32F103 USART1** @ `0x40013800` (APB2, Blue Pill–style).

**Não** substitui o gate RP (`examples/pilot/run.sh`).

| Campo | Valor |
|-------|-------|
| SoC alvo (mapeamento) | STM32F103C8 via `--preferred-manufacturer STMicroelectronics` |
| Peripheral | USART1 @ `0x40013800` (regs reais) |
| Analyze page | bloco @ `0x40013000` (máscara 4K do clustering) |
| Capstone (V1) | `fw.bin` AArch64 sintético (`gen_fw.py`) — **não** Thumb Cortex-M3 |
| Offsets no silício | SR=`+0x00`, DR=`+0x04`, CR1=`+0x0C` relativos a `0x40013800` |
| IRQ line (trace) | `0x25` (37 decimal — USART1) |

## Como rodar

```bash
cargo build -p base-cli
# regenerar blob Capstone (opcional):
python3 examples/pilot_stm32/gen_fw.py
./examples/pilot_stm32/run.sh
```

Smoke inclui:
1. `analyze --disasm` — Capstone resolve USART1 sem traces  
2. `analyze --mmio-traces` — path feliz design/synth  

## Arquivos

| Arquivo | Papel |
|---------|-------|
| `gen_fw.py` | Gera `fw.bin` AArch64 @ página USART1 |
| `fw.bin` | Blob Capstone (sintético) |
| `mmio.json` | Acessos MMIO USART1 |
| `contracts.yaml` / `trace.csv` | Prove + replay |
| `pilot.bsl` | BIR |
| `SHA256SUMS` | Integridade |
| `run.sh` | Smoke opt-in |
| `out/` | Gerado (gitignored) |
