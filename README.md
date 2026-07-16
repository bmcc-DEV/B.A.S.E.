# B.A.S.E. — Behavioral ASIC Synthesis Engine

[![CI](https://github.com/bmcc-DEV/B.A.S.E./actions/workflows/ci.yml/badge.svg)](https://github.com/bmcc-DEV/B.A.S.E./actions/workflows/ci.yml)
[![Formal](https://github.com/bmcc-DEV/B.A.S.E./actions/workflows/formal.yml/badge.svg)](https://github.com/bmcc-DEV/B.A.S.E./actions/workflows/formal.yml)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE.md)
[![Release](https://img.shields.io/github/v/release/bmcc-DEV/B.A.S.E.?display_name=tag)](https://github.com/bmcc-DEV/B.A.S.E./releases/tag/v1.1.0-rc)

> *"O que este hardware faz?" em vez de "Como este hardware foi implementado?"*

**Motor de engenharia reversa comportamental assistida** — evidência → contratos → Reference Design.

> **Tag [`v1.1.0-rc`](https://github.com/bmcc-DEV/B.A.S.E./releases/tag/v1.1.0-rc)** · estável anterior [`v1.0.0`](https://github.com/bmcc-DEV/B.A.S.E./releases/tag/v1.0.0) · [CHANGELOG](CHANGELOG.md) · [Path to v1.1](base-vault/21%20-%20Path%20to%20v1.1/21.00%20-%20Index.md)
>
> Specter VM (Forth-like + Lua) + maturidade **REAL\*** (reconstruct / pcb-draft / evolve / hil-host / fw-host) + wedges RP/STM32 + goldens `diff`.
>
> Demo: [Playbook v1.1](base-vault/21%20-%20Path%20to%20v1.1/21.20%20-%20Forensic%20Playbook.md) · `./examples/pilot/run.sh` · `./examples/pilot_study/run_study.sh`.
>
> **Não** é PCB fabricável, ASIC drop-in, SaaS turnkey, HIL production nem auto-fix.

---

## O que funciona hoje

Fonte da verdade: [**Maturity Matrix**](base-vault/12%20-%20Path%20to%20Real/12.02%20-%20Maturity%20Matrix.md)

### CLI / pipeline

| Área | Estado |
|------|--------|
| `analyze` / `design` / `synth` / `replay` / `prove` / `bir` / `check` / `pipeline` | **REAL\*** no wedge |
| `study` (Specter VM Forth + Lua) | **REAL\*** — loop autónomo; `auto_fix_complete=false` |
| `reconstruct` | **REAL\*** — `stop_reason`; ≠ auto-fix |
| `evolve` | **REAL\*** — métricas do HardwareSpec; opt-in no pipeline |
| `fw` | **REAL\*** host (`make host`); ≠ silício |
| `pcb` | **REAL\*** draft KiCad (`NOT FABRICABLE`) |
| `hil` | **REAL\*** host (enumerate / mock); production gated |

### Wedges / smokes

| Wedge | Smoke |
|-------|-------|
| RP UART / SPI | `run.sh` / `run_t1_b2.sh` |
| STM32 USART/SPI/I2C/TIM/triple | `pilot_stm32/run*.sh` |
| Specter study | `examples/pilot_study/run_study.sh` |

Docs: [Path to v1.1](base-vault/21%20-%20Path%20to%20v1.1/21.00%20-%20Index.md) · [Specter VM Spec](base-vault/21%20-%20Path%20to%20v1.1/21.30%20-%20Specter%20VM%20Spec.md)

---

## Pipeline

```text
Firmware → analyze → Evidence DB → BIR → Contracts → Solver → Reference Design
                         ↓
              study (Forth+Lua) / reconstruct
                         ↓
              [PCB/FW draft — opcional]
```

---

## Quick Start

```bash
git clone https://github.com/bmcc-DEV/B.A.S.E..git
cd B.A.S.E.
cargo build -p base-cli

./examples/pilot/run.sh
./examples/pilot/run_t1_b2.sh
./examples/pilot_study/run_study.sh
```

### Specter study

```bash
base study path/to/hardware_spec.yaml \
  --policy examples/pilot_study/policy.lua \
  --program examples/pilot_study/study.base \
  -o out/study/
# → study_report.json (stop_reason, auto_fix_complete=false)
```

### Análise / design / HIL

```bash
base analyze firmware.bin --mmio-traces mmio.json --classify uart -o output/
base design output/hardware_spec.yaml --pcb -o output/design/
base hil enumerate -o /tmp/hil/
base hil flash /tmp/x.bin --mock-flash -o /tmp/hil/
```

### Z3 (opcional)

```bash
cargo test -p base-core --features solver_z3 --lib smt
```

---

## Arquitetura

```mermaid
flowchart LR
    FW[Firmware] --> SP[SpecterProbe]
    SP --> EVD[Evidence DB]
    EVD --> BIR[BIR]
    BIR --> TC[Temporal Contracts]
    TC --> SOLVER[Contract Solver]
    SOLVER --> RD[Reference Design]
    EVD --> VM[Specter VM]
    Lua[Lua policy] --> VM
    VM --> Report[study_report]
    RD --> PCB[PCB draft]
```

### Tensão Ψ

```text
Ψ(B, H) = ∫ δ(ω_obs, ω_H) dμ
confidence = max(0, 1 - Ψ/(1+Ψ))
```

---

## CLI

| Comando | Notas |
|---------|-------|
| `analyze` / `synth` / `design` | Evidence → Reference Design |
| `study` | Specter Forth + Lua |
| `reconstruct` | Refine estrutural |
| `replay` / `prove` / `event-graph` / `bir` | Contratos |
| `evolve` / `fw` / `pcb` / `check` / `pipeline` | Outputs + validação |
| `hil` | Host REAL\*; production gated |

---

## Mercados

| Mercado | Papel |
|---------|-------|
| Forense / segurança | Wedge principal |
| Educação / pesquisa | Pipeline + Ψ + Specter |
| Preservação industrial | Consultoria + [SOW v1.1](base-vault/21%20-%20Path%20to%20v1.1/21.21%20-%20SOW%20Industrial%20Checklist.md) |
| SaaS | Adiado |

[`COMMERCIAL.md`](COMMERCIAL.md)

### Claims proibidos

PCB fabricável · ASIC drop-in · HIL production · SaaS turnkey · auto-fix completa · “produto industrial completo”

---

## Documentação

| Doc | Papel |
|-----|-------|
| [Maturity Matrix](base-vault/12%20-%20Path%20to%20Real/12.02%20-%20Maturity%20Matrix.md) | Fonte da verdade |
| [Playbook v1.1](base-vault/21%20-%20Path%20to%20v1.1/21.20%20-%20Forensic%20Playbook.md) | Demo |
| [Specter VM Spec](base-vault/21%20-%20Path%20to%20v1.1/21.30%20-%20Specter%20VM%20Spec.md) | Palavras + Lua |
| [CHANGELOG](CHANGELOG.md) | Tags |

---

## Licença

AGPLv3 — [LICENSE.md](LICENSE.md)
