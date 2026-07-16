# Changelog

Formato aproximado [Keep a Changelog](https://keepachangelog.com/). Tags: `v0.3.0-rc`, `v0.4.0-rc`, `v0.4.0`, `v0.5.0-rc`, `v0.5.0`, `v0.6.0-rc`, `v0.6.0`.

## [Unreleased] — Path to v0.7

### Added
- W1: dual STM32 USART1 + SPI2 @ `0x40003800` — `run_w1_spi.sh`, fixtures, teste `pilot_stm32_spi`
  - SPI1 @ `0x40013000` omitido (colisão página 4K com USART1)
- W2: goldens STM32 — `expected/event_graph.*`, `proof_report.golden.json`; smoke `diff` (não overwrite)
- W3: `reconstruct` UX — `stop_reason`/`stagnated`; `--continuous` = cap 1000 (≠ auto-fix)

## [v0.6.0] — 2026-07-16

Promoção de `v0.6.0-rc` após smoke local verde (`run.sh`, `pilot_stm32/run.sh`, `base-hil` / `base hil`).

Mesmo conteúdo funcional de `v0.6.0-rc` (V0–V5). Segue: Path to v0.7.

## [v0.6.0-rc] — 2026-07-16

Path to v0.6 V0–V5: STM32 Capstone/pins + `base hil` EXPERIMENTAL + oferta docs.
Smoke verde: `run.sh`, `run_t1_b2.sh`, `pilot_stm32/run.sh`, `base-hil`, `base hil enumerate|flash --mock-flash`.

### Added
- V1: Capstone STM32 USART1 — `gen_fw.py` AArch64 @ `0x40013000`/`0x40013800`; smoke `--disasm`
- V2: pins STM32F103C8 (PA9/PA10 USART1) + labels no draft KiCad; `base pcb` carrega component DB
- V3: `base hil enumerate|flash` — wrapper EXPERIMENTAL sobre `base-hil` (sem pipeline default)
- V4: playbook + SOW checklist v0.6 + COMMERCIAL sync

### Changed
- Matcher sch: interface `uart` aceita funções `usart*`

### Not
- PCB fabricável, ASIC drop-in, SaaS turnkey, HIL production, Thumb silício no blob Capstone STM32

## [v0.5.0] — 2026-07-16

Promoção de `v0.5.0-rc` após smoke local verde (`run.sh`, `run_t1_b2.sh`, `pilot_stm32/run.sh`, `base-hil` + `hil_programmer`).

Mesmo conteúdo funcional de `v0.5.0-rc` (U0–U5). Segue: Path to v0.6.

## [v0.5.0-rc] — 2026-07-16

Path to v0.5 U0–U5: segundo SoC (STM32) + HIL USB/programmer EXPERIMENTAL + oferta docs.
Smoke verde: `run.sh`, `run_t1_b2.sh`, `pilot_stm32/run.sh`, `base-hil` (+ `hil_programmer`).

### Added
- U1: wedge STM32F103 USART1 @ `0x40013800` — `examples/pilot_stm32/`, smoke opt-in
- `--preferred-manufacturer` em `base synth` / `base design` (path feliz → `STM32F103C8`)
- U2: feature `hil_usb` (rusb) — enumerate VID:PID → `Detected`; CI default sem USB
- U3: feature `hil_programmer` — flash EXPERIMENTAL via CMD externo (`ALLOW_FLASH`); ≠ production
- U4: playbook + SOW checklist v0.5 + COMMERCIAL (STM32 + HIL limits, sem overclaim)

### Changed
- Mapper: com preferência de fabricante, ranking prioriza mfg sobre score/preço

### Not
- PCB fabricável, ASIC drop-in, SaaS turnkey, HIL production, flash na CI default

## [v0.4.0] — 2026-07-16

Promoção de `v0.4.0-rc` após smoke local verde (`run.sh`, `run_t1_b2.sh`, testes piloto/SMT/HIL).

### Added
- Segundo bloco SPI0 @ `0x4003c000` (opt-in `examples/pilot/run_t1_b2.sh`)
- `--classify 0xADDR=kind,...` por página 4K
- `ProofBackend` / campo `backend` em `proof_report.json` (`symbolic` | `z3` | `graph`)
- Pins RP2350A (GP0–29) + `spi0_*` no RP2040; labels SPI no draft KiCad
- HIL `try_flash` / `FlashDenied` / `with_mock_flash` (`mock_dry_run` ≠ silício)

### Changed
- Playbook forense e COMMERCIAL alinhados a v0.4
- Maturity: `base-hil` EXPERIMENTAL com path Detected tipado

### Not
- PCB fabricável, ASIC drop-in, SaaS turnkey, flash HIL automático sem probe

## [v0.4.0-rc] — 2026-07-15

Path to v0.4 T0–T5 (mesmo conteúdo funcional promovido em `v0.4.0`).

## [v0.3.0-rc] — 2026-07-15

Path to v0.3 S0–S5: Capstone UART, formal.yml + Z3 0.20, pins RP2040, HIL EXPERIMENTAL host-only, playbook + SOW.

## [v0.2] — 2026-07-15

Path to Real R0–R6: piloto UART @ `0x40034000`, Evidence→Design, check sem self-pass, PCB `NOT FABRICABLE`, case study.
