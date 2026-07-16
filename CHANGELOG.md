# Changelog

Formato aproximado [Keep a Changelog](https://keepachangelog.com/). Tags: `v0.3.0-rc`, `v0.4.0-rc`, `v0.4.0`.

## [Unreleased] — Path to v0.5

### Added
- U1: wedge STM32F103 USART1 @ `0x40013800` — `examples/pilot_stm32/`, smoke opt-in
- `--preferred-manufacturer` em `base synth` / `base design` (path feliz → `STM32F103C8`)
- U2: feature `hil_usb` (rusb) — enumerate VID:PID → `Detected`; CI default sem USB
- U3: feature `hil_programmer` — flash EXPERIMENTAL via CMD externo (`ALLOW_FLASH`); ≠ production

### Changed
- Mapper: com preferência de fabricante, ranking prioriza mfg sobre score/preço

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
