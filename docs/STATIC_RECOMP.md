# Static Recompilation (Path v1.7 → v1.9 → P6.0)

Vault: [`base-vault/27`](../base-vault/27%20-%20Path%20to%20v1.7/27.00%20-%20Index.md) · [`base-vault/28`](../base-vault/28%20-%20Path%20to%20v1.8/28.00%20-%20Index.md) · [`base-vault/29`](../base-vault/29%20-%20Path%20to%20v1.9/29.00%20-%20Index.md)
Crate: `base-recomp`

## Pipeline

```text
x86-32 bytes | ELF .text | PE .text → lift → SIR → encode/decode/emit → multi-ISA
```

18 SIR op kinds: Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec, Push, Pop, LdMem, StMem, CallRel, JmpRel, Cmp, Test, BranchCond, Trap.

14 target ISAs: x86_64, ARM, AArch64, MIPS, PPC, SPARC, SuperH (SH-2/SH-4), Alpha, PA-RISC, M88k, IA-64, i860, ColdFire.

## Quick start

```bash
base recomp targets            # 14 alvos (amd64 ≡ x86_64)
base recomp semantics          # catálogo semântico 13 ISAs + JSON
base recomp verify --hex B80100000083C002C3 --target mips -o output/verify
base recomp verify --all       # cobertura 17 kinds × 6 dimensões por ISA
base recomp encode --hex B80100000083C002C3 --target coldfire -o output/enc
base recomp report --matrix    # matriz P0–P6 por ISA
```

## Encoder / decoder por ISA

| ISA | Encoder ops | Decoder | Notes |
|-----|------------|---------|-------|
| **x86_64** | 17/17 (ModRM subset + CallRel/JmpRel + Cmp/Test/BC) | ✅ round-trip | imm32 total; prefixos → gap |
| **AArch64** | 17/17 (W-reg, MOVZ/ADD/SUB #imm12, SUBS/ANDS, B.cond) | ✅ round-trip | lsl #12 → gap |
| **ARM** | 17/17 (imm8, MOV/ADD/SUB/CMP/TST/B<cond>) | ✅ round-trip | rotate≠0 → gap |
| **ColdFire** | 17/17 (68k subset + push/pop + CMP.L + Bcc.w) | ✅ round-trip | Test encodes as CMP (gap doc.) |
| **PPC** | 17/17 (r3..r31 + cmpw/cmplw + BC BO/BI) | ✅ round-trip | 8 BO/BI canônicos; encoder approxima 14 |
| **SPARC** | 17/17 (%l0..%l7 + subcc/andcc + bicc 14 conds) | ✅ round-trip | BE: subcc/andcc %g0 discard |
| **MIPS** | 14/17 (lw/sw + arith) | ✅ round-trip | — sem flags |
| **SuperH** | 14/17 (mov.l @Rm,Rn etc.) | ✅ round-trip | T flag; delay slots |
| **Alpha** | 14/17 (ldq/stq 64-bit + LDA) | ✅ round-trip | — sem flags |
| **PA-RISC** | 14/17 (LDI/LDO/LDW/STW + bv) | ✅ round-trip | — sem flags |
| M88k · IA-64 · i860 | emit texto | pendente | `Err(Unsupported)` |

## Semantic execution (`semexec.rs`)

```text
execute_reference(SIR, state, width, endian) == execute_isa(decode(encode(SIR)), state)
```

`MachineState { gpr[32], pc, flags, mem[64KiB] }` · width = 64 (Alpha) / 32 (demais) ·
`Flags { carry, overflow, zero, negative, extra }` — NZCV via `set_nzcv_sub`/`set_nzcv_and` ·
`eval_cond_nzcv()` evalua os 14 `Cond` variants. Memória: `load`/`store` com endianness
do catálogo, widths 1/2/4/8, `MemError::OutOfBounds/BadWidth`.

## P6.0 — Conditional control flow

SIR ops: `Cmp { rd, rs }` (flags from rd-rs), `Test { rd, rs }` (flags from rd&rs),
`BranchCond { cond: Cond, target }` (14 variants: Eq/Ne/Lt/Ge/Gt/Le/Cs/Cc/Mi/Pl/Vs/Vc/Hi/Ls).

6 ISAs com encode/decode de conditional: x86_64, AArch64, ARM, ColdFire, PPC, SPARC.

Sweep condicional: 70 programs — Cmp × {LT=0, EQ, GT=positive} × 14 conds + Test × {zero, nonzero} × 14 conds.
Cada programa gera taken e not-taken, 4 estados iniciais = 560 differential cases.

### Conditional per ISA

| ISA | Cmp | Test | BranchCond | Notes |
|-----|-----|------|------------|-------|
| x86_64 | CMP r/m32,r32 | TEST r/m32,r32 | Jcc rel32 (all 14) | Full EFLAGS subset |
| AArch64 | SUBS WZR,Wn,Wm | ANDS WZR,Wn,Wm | B.cond imm19 (all 14) | Full NZCV |
| ARM | CMP Rd,Rs | TST Rd,Rs | B<cond> imm24 (all 14) | Full CPSR |
| ColdFire | CMP.L Dn,Dm | CMP.L (as TST fallback) | Bcc.w disp16 (all 14) | TST gap documented |
| PPC | cmpw rA,rB | cmplw rA,rB | BC BO/BI (8 canonical) | CR0 has 4 bits only |
| SPARC | subcc %rs1,%rs2,%g0 | andcc %rs1,%rs2,%g0 | b<cond> disp22 (all 14) | icc/xcc |
| MIPS/SuperH/Alpha/PaRISC | — | — | — | No flags register |

### Accepted P6 conditions

```text
P6 = P5 + 100% all dimensions + sweep condicional clean
```

Not in P6: MMU, interrupts, privileged, FP, SIMD, atomics, precise memory ordering, full exceptions.

## Differential sweep

```text
coldfire: 268 base + conditional  · all match
x86_64:   268 base + conditional  · all match
mips:     184 base (no conditional) · all match
ppc:      184 base + conditional  · all match
alpha:    184 base (no conditional) · all match
sparc:    base + conditional      · all match
aarch64:  base + conditional      · all match
arm:      base + conditional      · all match
```

Sweep space: kinds × 10 imms × 4 states × (conditional expansion for P6 ISAs).

## Cobertura por dimensão

18 kinds × 6 dimensões (encoder/decoder/literal/semantic/exec/differential):

| ISA | enc | dec | sem | diff | exec | status | Level |
|-----|-----|-----|-----|------|------|--------|-------|
| x86_64 | 100% | 100% | 100% | 100% | 100% | FULL | **P6** |
| AArch64 | 100% | 100% | 100% | 100% | 100% | FULL | **P6** |
| ARM | 100% | 100% | 100% | 100% | 100% | FULL | **P6** |
| ColdFire | 100% | 100% | 100% | 100% | 100% | FULL | **P6** |
| PPC | 100% | 100% | 100% | 100% | 100% | FULL | **P6** |
| SPARC | 100% | 100% | 100% | 100% | 100% | FULL | **P6** |
| MIPS | 83% | 83% | 83% | 83% | 100% | FULL | P5.1 |
| SuperH | 83% | 83% | 83% | 83% | 100% | FULL | P5.1 |
| Alpha | 83% | 83% | 83% | 83% | 100% | FULL | P5.1 |
| PA-RISC | 83% | 83% | 83% | 83% | 100% | FULL | P5.1 |
| M88k | 0% | 0% | 0% | 0% | 100% | NONE | P1 |
| IA-64 | 0% | 0% | 0% | 0% | 100% | NONE | P1 |
| i860 | 0% | 0% | 0% | 0% | 100% | NONE | P1 |

`exec` = 100% em todas (executor de referência modela todos os 18 kinds).
83% = 15/18 kinds (sem cmp/test/bcond — sem flags register; trap emit sem encoder).
`abi/privileged/mmu/system` = **0%** (não modelados).

## Bugs pegos pelo verifier

1. **ColdFire Dn bits 5-3**: `clr.l d3` → `0x4283` = `clr.l (a3)+`. Sweep com VReg não-zero detectou.
2. **PPC r0 colisão**: `addi r0,0,5` lê RA como 0. Fix: encoder usa `r3..r31`.
3. **Alpha i32 domain**: executor tratava imeds como `u32`, mas SIR é `i32`. Fix: sign-extend.
4. **ColdFire push/pop encoding**: `0x29C0` era `move.l d0,(a4)+`, não push. Capstone validou.

## Honesty

`static_recomp_complete: false` · `win32_abi_complete: false` · `runs_any_pe: false`
Encode/decode/executor são parciais até validados contra cross-assembler / QEMU / capstone.
Eixos `abi/privileged/mmu/system` = 0% (separados, não misturados num score único).
