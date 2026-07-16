# Pilot fixture — Path to Real v0.2 (R6)

Wedge sintético de **UART MMIO estilo PL011** em `0x40034000`.

Este diretório **não** reivindica um SoC real. O `fw.bin` é placeholder;
a fonte da verdade comportamental é `mmio.json` + `trace.csv` + `contracts.yaml`.

**Case study (vault):** [`12.20 - Pilot Case Study`](../../base-vault/12%20-%20Path%20to%20Real/12.20%20-%20Pilot%20Case%20Study.md)

## Wedge

| Campo | Valor |
|-------|-------|
| Classe | MCU / peripheral MMIO (ARM-like) |
| Peripheral | UART @ `0x40034000` |
| Por quê | Endereços estáveis, tipos Saleae (WRITE/READ/IRQ) |
| Fora de escopo | GPU, SerDes HS, Power Mac / Xbox / Alpha |
| v0.3+ | Capstone UART realista (`gen_fw.py`) |
| v0.4 T1 | SPI0 @ `0x4003c000` como **2º bloco** (opt-in `run_t1_b2.sh`) |

## Arquivos

| Arquivo | Papel |
|---------|-------|
| `fw.bin` | Blob AArch64-LE raw (`gen_fw.py`) — Capstone encontra `0x40034000` |
| `gen_fw.py` | Regenera `fw.bin` sem toolchain ARM |
| `SHA256SUMS` | Integridade das fixtures UART (gate) |
| `mmio.json` | Acessos MMIO UART |
| `pilot.bsl` | Spec BSL → BIR + contratos Saleae |
| `trace.csv` / `trace_fail.csv` / `trace_slow.csv` | Pass / fail / latency inject |
| `contracts.yaml` / `contracts.unsat.yaml` | SAT + UNSAT |
| `run.sh` | Smoke E2E gate UART → `out/CASE_SUMMARY.md` |
| `mmio_uart_spi.json` | T1 B2 — UART + SPI0 |
| `contracts_spi.yaml` / `trace_spi.csv` / `pilot_spi.bsl` | T1 B2 SPI |
| `SHA256SUMS.b2` / `run_t1_b2.sh` | Smoke opt-in dual-block |
| `out/` / `out_t1_b2/` | Gerado (gitignored) |
| `expected/` | Goldens + template CASE_SUMMARY |

## Como rodar

```bash
cargo build -p base-cli
cd examples/pilot && sha256sum -c SHA256SUMS && cd ../..
./examples/pilot/run.sh
# T1 B2 (não substitui o gate):
./examples/pilot/run_t1_b2.sh
# classify dual: --classify '0x40034000=uart,0x4003c000=spi'
```

## Esperado no smoke

- `out/analyze/` — HardwareSpec + Evidence + `tension_report.json`
- `out/design/reference_design.yaml` — CPU MCU ≠ TBD, contratos ≥70%
- `out/prove*` — SAT e UNSAT honestos
- `out/check_skip` / `check_dual` — sem self-pass; TIMING_VIOLATION no dual
- `out/fw/firmware_host` exit 0
- `out/CASE_SUMMARY.md`

## Limitações honestas

- **Host smoke ≠ silício**
- Parser Saleae: WRITE / READ / IRQ
- PCB fora do `run.sh` (`pipeline --pcb` → `NOT FABRICABLE`)
- Classificação UART: `--classify uart` + traces
