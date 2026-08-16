# Changelog

Formato aproximado [Keep a Changelog](https://keepachangelog.com/). Tags: `v0.3.0-rc`, `v0.4.0-rc`, `v0.4.0`, `v0.5.0-rc`, `v0.5.0`, `v0.6.0-rc`, `v0.6.0`, `v0.7.0-rc`, `v0.7.0`, `v0.8.0-rc`, `v0.8.0`, `v0.9.0-rc`, `v0.9.0`, `v1.0.0-rc`, `v1.0.0`, `v1.1.0-rc`, `v1.2.0-rc`, `v1.2.0`, `v1.3.0-rc`, `v1.4.0-rc`, `v1.5.0-rc`, `v1.6.0-rc`, `v1.6.1-rc`, `v1.6.2-rc`, `v1.6.3-rc`, `v1.7.0-rc`, `v1.8.0-rc`.

## [Unreleased]

### Added
- **Preservação de semântica das 11 arquiteturas** (`base-recomp`) — PowerPC, MIPS,
  DEC Alpha, PA-RISC, ARM, M88k, IA-64, SPARC, i860, ColdFire, SuperH
  - `semantics.rs` — catálogo semântico serde (registradores, endianness, delay slots,
    flags, register windows, predicação, ABI, quirks, status de encode) → `base recomp semantics`
  - Novos alvos `TargetIsa`: `alpha` · `parisc` (HPPA) · `m88k` · `ia64` · `i860` · `coldfire` (14 no total)
  - Encoders: Alpha (Nop/Ret/imm-adds via LDA; verificado contra LLVM+QEMU opcodes),
    PA-RISC (Nop/`bv %r0(%rp)`; verificado contra SLEIGH Ghidra), ColdFire (68k subset;
    verificado via capstone m68k)
  - M88k/IA-64/i860: emit texto + encode `Unsupported` (honesto; encoder pendente)
- CLI `base recomp semantics` — dump do catálogo + `semantics_catalog.json`
- **Verifier round-trip** (`verify.rs` + `decode.rs`) — `SIR → encode → bytes → decode → SIR′`
  - Comparação literal (`SIR == SIR′`) + semântica (`semantic_key` normaliza Clear/Inc/Dec/SubImm)
  - Decoders subset **Alpha / PA-RISC / ColdFire** (além de MIPS/PPC/SuperH) — invertem
    exatamente o que os encoders emitem; delay slots de `jr`/`rts`/`bv` dobrados no `Ret`;
    ColdFire suporta comprimento variável (moveq/move.l #imm/addi/subi)
  - **Cobertura por dimensão** (nunca um "preservation score" único):
    `enc` / `dec` / `literal` / `semantic` por ISA + status `FULL|PARTIAL|PENDING|NONE`
    (mips/ppc/sh/alpha = semantic 67%, parisc 17%, coldfire 83%; sem decoder → `PENDING`,
    sem encoder → `NONE`; eixos `abi/privileged/mmu/system` = `0%` não modelados)
  - Fix `ppc` r0 colisão (movido p/ `r3..r31`) — round-trip literal de `add3` em 5 ISAs
- **Execução semântica de referência** (`semexec.rs`) — `execute_reference(SIR, state, width, endian)
  == execute_isa(decode(encode(SIR)), state)` (comportamento, não representação)
  - `MachineState { gpr, pc, flags, mem }` · width 64 (Alpha) / 32 (demais)
  - **Memória real**: `load`/`store` (endianness do catálogo semântico, widths 1/2/4/8,
    64 KiB, `MemError::OutOfBounds/BadWidth`) · `Push`/`Pop` executáveis via `SP = VReg 4`
  - `Flags { carry, overflow, zero, negative, extra }` estruturado (ops ainda não setam
    flags — próximo rung)
  - Dimensão `differential` na cobertura: imeds de borda (`0xFFFFFFFF`) + estado de borda
    (alpha 50% — LDA sign-extend a 64 bits detectado; sh 33% — immed 32-bit não encodável;
    mips/ppc 67%; coldfire 83% com push/pop)
  - **Sweep gerado** (`differential_sweep` + CLI `verify --sweep`): kinds × 10 imms × 4
    estados — coldfire 256/256 match, mips 176/176, alpha 160/176 (16 mismatches só em
    add/sub_imm negativos, gap documentado)
  - **Pegou bug real ColdFire**: `Dn` em bits 5-3 sem `<< 3` no grupo
    addi/subi/clr/addq/subq/push (`clr.l d3` virava `0x4283` = `clr.l (a3)+`) — só D0
    passava no roundtrip antigo (VReg 0); corrigido + teste de reg não-zero
  - **Fix domínio de immediates no executor de referência**: imeds SIR são `i32`
    (`semantic_key` documenta), mas `execute()` os tratava como `u32` — o "gap Alpha"
    (LDA sign-extend vs `+0xFFFFFFFF`) era essa inconsistência. `semexec::execute` agora
    sign-extende i32 → largura (`0xFFFFFFFF` = −1 a 64 bits); differential Alpha 50% → 67%,
    sweep 176/176. Provado pelo diferencial: representacional sozinho não vê
  - `semantic_key` normaliza imms para i32 (domínio 32-bit do SIR) — o desvio de largura
    (Alpha 64-bit) fica na dimensão `differential`, não na semântica
- CLI `base recomp verify --hex … --target <isa>` · `--all` (tabela com enc/dec/literal/semantic/exec/differential) · `--sweep --target <isa>`
- **Fix encoder PPC**: VReg → `r3..r31` (r0 lê como 0 no RA de `addi`; colisão
  `MovImm`/`AddImm` que quebrava o round-trip) — golden `ppc_add3.s` atualizado
- **Decoder AArch64** (`decode.rs`) — subset W-reg (LE 32-bit) que inverte o encoder:
  `Nop`/`Ret`/`MOV Wd,WZR` (Clear)/`MOVZ` (MovImm 16-bit)/`ADD`/`SUB` #imm12 com `rn==rd`;
  formas shiftadas (`lsl #12`) → gap, nunca mis-decode. AArch64: enc/dec/literal/semantic
  = 67%, differential 33% (edge `0xFFFFFFFF` não cabe em MOVZ/ADD 12-bit — limitação
  honesta, mesma do SH), sweep 136/136. Sai de `PENDING` → `PARTIAL`
- **Decoder ARM** (`decode.rs`) — subset (LE 32-bit, cond AL) que inverte o encoder:
  `Nop`/`BX LR` (Ret)/`MOV Rd,#imm8` (MovImm; Clear → `MOV #0` normaliza semanticamente)/
  `ADD`/`SUB Rd,Rd,#imm8` com `rn==rd`. Formas fora do subset (rotate≠0, `ADDS` S=1)
  → gap, nunca mis-decode. ARM: enc/dec/semantic = 67%, literal 58% (Clear→`MOV #0`),
  differential 33% (imm8), sweep 104/104. Sai de `PENDING` → `PARTIAL`
- **Decoder SPARC** (`decode.rs`) — subset (BE, 32-bit) que inverte o encoder:
  `sethi %hi(0),%g0` (Nop)/`retl`+delay nop (Ret, fold como MIPS/PPC)/`or %g0,%g0,rd`
  (Clear)/`or %g0,imm13,rd` (MovImm, sign-extend; `0xFFFFFFFF` = −1 encoda) — registradores
  %l0..%l7; rd fora da janela → gap. SPARC: enc/dec/literal/semantic/differential = 33%,
  sweep 56/56. Sai de `PENDING` → `PARTIAL`
- **Decoder x86_64** (`decode.rs`) — subset (comp. variável, LE) que inverte o encoder:
  `0x90`/`0xC3`, `B8+imm32` (MovImm), `05`/`2D`+imm32 (add/sub eax), `83 /0 /5` imm8
  (add/sub), `31 C0-reg-reg` (Clear), `40+/48+` (Inc/Dec), `50+/58+` (Push/Pop). ModRM
  fora do subset / prefixos (`0x66`…) → `Op::Unknown` gap, nunca mis-decode. imm32 total
  + push/pop → 83% em todas as dimensões; só `call`/`jmp` faltam (reloc). sweep 256/256.
  Sai de `PENDING` → `PARTIAL`
- Tests: `r7_verify` (round-trip literal+semântico, scores honestos)
- Tests: `r6_semantics` (encode bytes conhecidos + catálogo 11 entradas + emit all-targets)
- Tests: decode x86_64 (round-trip full subset, imm32 form, prefix = gap); verify cobertura x86_64; sweep inclui x86_64
- **ISA Preservation Layer** — formalização da preservação por evidência (não prosa)
  - `verify.rs`: `preservation_level` (P0–P5, bandas objetivas de cobertura medida) ·
    `preservation_report(isa)` (identidade + codec + semântica + diferencial + gaps +
    claims `hardware_validated:false` / `complete:false`) · `preservation_matrix`
  - CLI `base recomp report [--isa <x> | --matrix]` — relatório **gerado**, nunca manuscrito
  - Vault `base-vault/isa/` — `README.md` (princípios, 3 camadas, níveis, regras de ouro)
    + `report.md` (snapshot gerado das 14 ISAs; matriz + relatórios)
- **Load/store no SIR** (`LdMem`/`StMem`) — memória é camada 3 de preservação
  - SIR ops novos: `dst := mem[base+offset]` / `mem[base+offset] := src` (width, endianness da ISA)
  - Executor de referência: `load`/`store` com endianness do catálogo; `op_kind`/sweep/verify
    estendidos (14 kinds; `execute_pct` 83% → 86%)
  - **ColdFire** única ISA com encoder/decoder de ld/st: `move.l (An),Dn` / `move.l Dn,(An)`
    (capstone-verificados); ColdFire = 86% em todas as dimensões, sweep 268/268
  - **Fix bug real push/pop ColdFire**: encoder antigo `0x29C0` era `move.l d0,(a4)+`
    (não push) e campo `Dn` em bits errados — sweep só usava VReg 0, roundtrip
    consistente-mas-errado; capstone m68k corrigiu (push `0x2F00|dn`, pop `0x201F|dn<<9`)
  - Os 2 kinds novos expõem cobertura honesta: mips/ppc/alpha 67%→57% (não encodam ld/st),
    sparc 33%→29%, x86 83%→71%
- **ld/st nas ISAs restantes** — memória capstone-verificada
  - MIPS `lw/sw` (`0x8C000000`/`0xAC000000`, capstone: `0x8D280000` = `lw $t0,($t1)`)
  - PPC `lwz/stw` (`0x80000000`/`0x90000000`, capstone: `0x80640000` = `lwz r3,0(r4)`)
  - Alpha `ldq/stq` (opcode `0x29`/`0x2D` << 26, width 8; formato de memória LLVM)
  - AArch64 `ldr/str` (`0xB9400000`/`0xB9000000`, offset 0; imm escalado ≠ 0 = gap)
  - Probes/sweep de ld/st usam a largura de palavra da ISA (`word_bits/8`) — Alpha
    naturalmente width 8
  - MIPS/PPC/Alpha/AArch64 → 71% em enc/dec/semantic/differential (P5); sweep limpo
    em todas as ISAs com decoder (mips/ppc/alpha 184/184, aarch64 144/144)
  - Alpha decoder ganhou `decode_alpha` ldq/stq; encoder corrigido de `0x29<<26` (era
    `0x29000000` = opcode 0x0a — shift errado)
- **ld/st ARM/SPARC/x86** — memória completa capstone-verificada
  - ARM `ldr/str` (`0xE5900000`/`0xE5800000`, offset 0; capstone: `0xE5910000` = `ldr r0,[r1]`)
  - SPARC `ld/st` (op=3, op3=0x00/0x04, %l0..%l7; capstone: `0xE0046000` = `ld [%l1],%l0`)
  - x86 ModRM `8B/89 00|rm` (`mov eax,[base]`/`mov [base],eax`; capstone-verificado)
  - **x86_64 atinge 86% em todas as dimensões** (12/14 kinds — só call/jmp, reloc);
    ARM 71% (43% diff), SPARC 43%; sweep limpo: x86 268/268, arm 112/112, sparc 64/64
  - SH `mov.l @Rm,Rn` ficou como gap: capstone SH não cobre o formato — não inventar
    opcode sem fonte confiável (honestidade)
- Tests: `ld_st_execute_and_differential_on_coldfire` (memória BE/LE + differential),
  encode/decoder ColdFire capstone-goldens (push/pop/ld/st), cobertura 14 kinds
  atualizada em r7/verify

### Changed
- `base recomp targets` → 14 alvos; docs `docs/STATIC_RECOMP.md` seção Path v1.9

### Added (continuação Path v1.9)
- **`hardware/`** — scaffold OSHWA-style (docs, BOM template, fab notes, scripts KiCad CLI, ponte `base pcb`)
  - Honesty: `engineering_draft — NOT FABRICABLE` até Claim B (Industrial Gate)
  - `hardware/scripts/validate-project.sh` · `export-gerbers.sh` · `ingest-base-pcb.sh`
- **Path to v1.9** — PE `.text`, símbolos→call/jmp labels, `encode` SIR→bytes (MIPS/PPC/SPARC/SH/ARM/…), runtime stub Saturn/DC
- CLI `base recomp pe|encode|runtime`
- Feature opcional `capstone` em `base-recomp`
- Honesty: `runs_on_saturn` / `runs_on_dreamcast` = false

## [v1.8.0-rc] — 2026-07-18

### Added
- **Path to v1.8** (`base-vault/28`) — lifter x86 alargado + ELF `.text`
- `base recomp elf` — load ELF32/64 x86 → SIR → emit (rejeita arch ≠ x86)
- Novos SIR ops: `SubImm`, `Clear`, `Inc`/`Dec`, `Push`/`Pop`, `CallRel`/`JmpRel`
- `gaps_markdown` — gaps listados no `RECOMP_REPORT.md`
- Fixture `base-recomp/tests/fixtures/add3.o`
- Call/jmp emit honesto (`ud2` até symbols; sem `call imm` inválido)

## [v1.7.0-rc] — 2026-07-18

### Added
- **Path to v1.7** (`base-vault/27`) — decompilação/recompilação estática x86 → multi-ISA via SIR
- Crate **`base-recomp`** — lift subset x86-32 → SIR → emit (`x86_64`/`amd64`, ARM, AArch64, MIPS, PPC, SPARC, SH-2/SH-4)
- CLI `base recomp lift|file|targets|roundtrip|assemble-arm` — honesty: `static_recomp_complete: false` · `win32_abi_complete: false`
- R2: goldens x86_64 + host roundtrip (`as`/`cc`)
- R3: goldens ARM + AArch64 + assemble-only ARM (`arm-none-eabi-as`)
- R4: goldens MIPS + PowerPC + SPARC
- R5: goldens SuperH SH-2/SH-4 (Saturn/Dreamcast class banners)
- Docs [STATIC_RECOMP.md](docs/STATIC_RECOMP.md)
- Crate **`base-reason`** — QRM + belief graph + hypothesis set + triad gate + session strengthen/forget (≠ Transformer)
- CLI `base reason report` / `base reason g35` — HW atlas → reason report + receipt draft (`lab_assist`)
- Manifesto [docs/PLATFORM_RE.md](docs/PLATFORM_RE.md) · piloto [examples/pilot_moto_g35/REASONING.md](examples/pilot_moto_g35/REASONING.md)
- `run_path_a.sh` — one-shot handoff externo (pack + phandle resolve + validate)
- `resolve_dt_phandles.py` — UART `clocks = <&…>` a partir do vendor FDT
- `lab_watch_assist.sh` — monitor ADB read-only → actualiza receipt (≠ flash)
- Guia **postmarketOS** (`POSTMARKETOS.md`) — alvo de port sem Android userspace (manila/ums9620)
- `EXTERNAL_TREE.md` · `pack_external_handoff.sh` — tree externo / handoff pack

## [v1.6.3-rc] — 2026-07-17

### Added
- `base port clocks-pinctrl` — hints USB×DTB (clock-names, controllers, pinctrl) + DTSI snippet
- `run_wedge_specter_live.sh` — Specter twin/watch nas bases P0+GICR + QMP opt-in
- Spec/trace wedge com GICR `0x12040000`

### Changed
- QMP `human-monitor`: HMP `Error:` → falha real (savevm sem block deixa de reportar `ok: true`)

## [v1.6.2-rc] — 2026-07-17

### Changed
- DTB `reg` parse: respeita `#address-cells`/`#size-cells` + walk de `ranges` (GICR ums9620 `0x12040000`)
- Wedge stub DTSI/HAL emite GICD+GICR quando o atlas resolve `gic_redistributor`

## [v1.6.1-rc] — 2026-07-17

**G35 wedge absoluto** — USB live → atlas P0 → stub DT/earlycon → Specter/QEMU → fase C assist.
Fecha o assist de bring-up no repo; port do OS = tree externo ([WEDGE_HANDOFF](examples/pilot_moto_g35/WEDGE_HANDOFF.md)).
≠ OS turnkey · ≠ flash automático · ≠ earlycon no silício.

### Added
- `base port usb-probe` / `usb-cross` — inventário USB vivo + cruzamento DTB
- Atlas P0 absoluto `wedge_mmio_map.yaml` (UART `0x20200000` · GIC `0x12000000` · UFS `0x22000000`)
- `base port wedge-p0` — board stub DTSI + earlycon hints + HAL host
- Piloto: `run_wedge_pipeline.sh` · `run_wedge_qemu_smoke.sh` · `run_wedge_hw_assist.sh`
- Vault `24.41`–`24.44` · `WEDGE_HANDOFF.md`

### Not
- Walk completo de `ranges` FDT · máquina QEMU ums9620 · flash automático · OS bootável / TaurOS turnkey

## [v1.6.0-rc] — 2026-07-16

Path to v1.6: **Twin↔guest** + BIR DigitalTwin + QMP savevm + continuous watch.
≠ OS turnkey · ≠ HIL production.

### Added
- Vault `base-vault/26 - Path to v1.6/`
- F0 `base virt twin` — Spec MMIO shadow vs Evidence
- F1 `base virt bir-twin` — Spec+Evidence → BIR → DigitalTwin
- F2 QMP `savevm`/`loadvm`/`probe-savevm`
- F3 `base virt watch` — continuous NDJSON↔twin ticks
- Spec stub MAME `hardware_spec_mame_stub.yaml`

### Not
- Snapshot garantido em todas as máquinas QEMU · claim OS turnkey

## [v1.5.0-rc] — 2026-07-16

Path to v1.5: **Specter Live** (QEMU→Evidence→Ψ→study; adapters MAME/libretro).
≠ OS turnkey · ≠ HIL production · ≠ emuladores embutidos.

### Added
- Vault `base-vault/25 - Path to v1.5/`
- Crate `base-virt` + CLI `base virt ingest|score|run|qmp|study`
- E2 TCG plugin `base-virt/plugin/` · E3 QMP · E4 Study↔Live · E5 `TraceSource` (ndjson/mame/libretro)
- Pilot `examples/pilot_moto_g35/run_virt_live.sh`

### Not
- MAME/RetroArch runtime in-tree · plugin no CI default · claim TaurOS/ReactOS

## [v1.4.0-rc] — 2026-07-16

Path to v1.4: **OS Port Validation Assist** (forense → QEMU → hardware).
≠ port ReactOS/TaurOS automático turnkey.

### Added
- Vault `base-vault/24 - Path to v1.4/` (Gate A/B/C + SOW + playbook)
- Pilot Moto G35: `examples/pilot_moto_g35/` fase A (`run.sh`) + B (`run_qemu_smoke.sh`) + C SOP
- Pilot iMac G3: `examples/pilot_imac_g3/` fase A + QEMU esqueleto + `REACTOS_EXTERNAL.md`
- Use case [[06.04 iMac G3 late 2001]]
- CI opt-in smokes G35/iMac fase A

### Not
- Capstone PowerPC · ReactOS/TaurOS build in-tree · claim “port validado de vez” sem A+B+C+SOW

## [v1.3.0-rc] — 2026-07-16

Path to v1.3: HIL Lab Gate A — `base hil lab-status` + SOP + smoke.
Smoke: `run_hil_lab.sh` (production=false; lab_assist_ready=false no CI default).

### Added
- `base_hil::lab_gate::evaluate_lab_gate` (A1–A5)
- CLI `base hil lab-status --sop … [--sow-signed]`
- `examples/hil_lab/SOP.md` + `run_hil_lab.sh` + CI

### Not
- HIL production / CI flash / PCB fab / auto-fix

## [v1.2.0] — 2026-07-16

Promoção de `v1.2.0-rc` (milestone docs: SOW Industrial Gate + mapa Paleo).
Segue: Path to v1.3 — HIL Lab (Gate A).

## [v1.2.0-rc] — 2026-07-16

Path to v1.2 B0–B5 (**docs**): SOW Industrial Gate + mapa PaleoComputação → B.A.S.E.
Sem mudança obrigatória de código forense; gates `run.sh` / `run_study.sh` intactos.

### Added
- B1: [[base-vault/22 - Path to v1.2/22.30 - SOW Industrial Gate|SOW Industrial Gate]] — pré-condições HIL lab / PCB eng. / auto-fix parcial
- B2: mapa Paleo (fósseis, Ψ, falsificação) → ética B.A.S.E. (PDF software ≠ wedge)
- B3–B4: SOW/playbook v1.2 + COMMERCIAL/README
- B5: tag `v1.2.0-rc`

### Not
- Implementação PCB fabricável / HIL production / auto-fix (só o *gate* para quando)

## [v1.1.0-rc] — 2026-07-16

Path to v1.1 A0–A5: Specter VM (Forth-like + Lua) + maturidade REAL\* (reconstruct/pcb-draft/evolve/hil-host/fw-host).
Smoke verde: `run.sh`, `run_t1_b2.sh`, `run_study.sh` (+ STM32 opt-in herdados).

### Added
- A1–A3: crate `base-vm` + `base study` + `examples/pilot_study/run_study.sh` + CI
- A4: evolve métricas do HardwareSpec; goldens PCB `NOT FABRICABLE`; testes reconstruct/hil host
- A5: playbook/SOW v1.1 + Maturity Matrix REAL\* + tag `v1.1.0-rc`

### Not
- PCB fabricável, ASIC drop-in, SaaS turnkey, HIL production, auto-fix completa

## [v1.0.0] — 2026-07-16

Promoção de `v1.0.0-rc` após smoke local verde (`run.sh`, `run_t1_b2.sh`, `pilot_stm32/run.sh`, `run_w1_spi.sh`, `run_x3_i2c.sh`, `run_y3_triple.sh`, `run_z2_tim.sh`).

Mesmo conteúdo funcional de `v1.0.0-rc` (Z0–Z5). **v1.0 ≠** produto industrial completo.

## [v1.0.0-rc] — 2026-07-16

Path to v1.0 Z0–Z5: goldens SPI STM32 + TIM2 opt-in + Maturity Matrix sync + oferta docs.
Smoke verde: `run.sh`, `run_t1_b2.sh`, `pilot_stm32/run.sh`, `run_w1_spi.sh`, `run_x3_i2c.sh`, `run_y3_triple.sh`, `run_z2_tim.sh`.

### Added
- Z1: goldens SPI STM32 — `expected_spi/` event-graph + prove; `run_w1_spi.sh` `diff`; teste `pilot_stm32_spi_goldens`
- Z2: dual STM32 USART1 + TIM2 @ `0x40000000` — `run_z2_tim.sh`, classify `timer`/`tim`, teste `pilot_stm32_tim`
- Z3: Maturity Matrix sync — wedges RP/STM32 + goldens/flags/HIL EXPERIMENTAL; README alinhado
- Z4: playbook + SOW checklist v1.0 + COMMERCIAL sync (≠ produto industrial completo)
- Z5: tag `v1.0.0-rc`

### Not
- PCB fabricável, ASIC drop-in, SaaS turnkey, HIL production, “v1.0 = produto industrial completo”

## [v0.9.0] — 2026-07-16

Promoção de `v0.9.0-rc` após smoke local verde (`run.sh`, `run_t1_b2.sh`, `pilot_stm32/run.sh`, `run_w1_spi.sh`, `run_x3_i2c.sh`, `run_y3_triple.sh`).

Mesmo conteúdo funcional de `v0.9.0-rc` (Y0–Y5). Segue: Path to v1.0.

## [v0.9.0-rc] — 2026-07-16

Path to v0.9 Y0–Y5: I2C1 pins + goldens I2C + triple USART/SPI/I2C + oferta docs.
Smoke verde: `run.sh`, `run_t1_b2.sh`, `pilot_stm32/run.sh`, `run_w1_spi.sh`, `run_x3_i2c.sh`, `run_y3_triple.sh`.

### Added
- Y1: pins I2C1 STM32F103C8 (PB6 SCL / PB7 SDA) + labels no draft sch; matcher `scl`/`sda`
- Y2: goldens I2C STM32 — `expected_i2c/` event-graph + prove; `run_x3_i2c.sh` `diff`; teste `pilot_stm32_i2c_goldens`
- Y3: triple STM32 USART1+SPI2+I2C1 — `run_y3_triple.sh`, `mmio_usart_spi_i2c.json`, teste `pilot_stm32_triple`
- Y4: playbook + SOW checklist v0.9 + COMMERCIAL sync
- Y5: tag `v0.9.0-rc`

### Not
- PCB fabricável, ASIC drop-in, SaaS turnkey, HIL production, auto-fix completa, Amiga/CD32 como wedge de release

## [v0.8.0] — 2026-07-16

Promoção de `v0.8.0-rc` após smoke local verde (`run.sh`, `run_t1_b2.sh`, `pilot_stm32/run.sh`, `run_w1_spi.sh`, `run_x3_i2c.sh`).

Mesmo conteúdo funcional de `v0.8.0-rc` (X0–X5). Segue: Path to v0.9.

## [v0.8.0-rc] — 2026-07-16

Path to v0.8 X0–X5: SPI2 pins + RP goldens diff + I2C1 dual + oferta docs.
Smoke verde: `run.sh`, `run_t1_b2.sh`, `pilot_stm32/run.sh`, `run_w1_spi.sh`, `run_x3_i2c.sh`.

### Added
- X1: pins SPI2 STM32F103C8 (PB12–15) + labels no draft sch; matcher `mosi`/`miso`/`nss`
- X2: goldens RP — `run.sh` `diff` vs `expected/` (sem overwrite); `proof_report.golden.json`
- X3: dual STM32 USART1 + I2C1 @ `0x40005400` — `run_x3_i2c.sh`, teste `pilot_stm32_i2c`
- X4: playbook + SOW checklist v0.8 + COMMERCIAL sync
- X5: tag `v0.8.0-rc`

### Not
- PCB fabricável, ASIC drop-in, SaaS turnkey, HIL production, auto-fix completa, Amiga/CD32 como wedge de release

## [v0.7.0] — 2026-07-16

Promoção de `v0.7.0-rc` após smoke local verde (`run.sh`, `run_t1_b2.sh`, `pilot_stm32/run.sh`, `run_w1_spi.sh`).

Mesmo conteúdo funcional de `v0.7.0-rc` (W0–W5). Segue: Path to v0.8.

## [v0.7.0-rc] — 2026-07-16

Path to v0.7 W0–W5: STM32 SPI2 dual + goldens + reconstruct honesty + oferta docs.
Smoke verde: `run.sh`, `run_t1_b2.sh`, `pilot_stm32/run.sh`, `run_w1_spi.sh`.

### Added
- W1: dual STM32 USART1 + SPI2 @ `0x40003800` — `run_w1_spi.sh`, fixtures, teste `pilot_stm32_spi`
  - SPI1 @ `0x40013000` omitido (colisão página 4K com USART1)
- W2: goldens STM32 — `expected/event_graph.*`, `proof_report.golden.json`; smoke `diff` (não overwrite)
- W3: `reconstruct` UX — `stop_reason`/`stagnated`; `--continuous` = cap 1000 (≠ auto-fix)
- W4: playbook + SOW checklist v0.7 + COMMERCIAL sync
- W5: tag `v0.7.0-rc`

### Not
- PCB fabricável, ASIC drop-in, SaaS turnkey, HIL production, auto-fix completa, Amiga/CD32 como wedge de release

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
