# P6 Conditional Execution Specification

## Objetivo
Round-trip 100% (encoder + decoder + literal + semantic + differential + execute) para **compare + branch condicional** nas 11 ISAs P5.

## Escopo (P6.0)

### SIR Ops novos
```rust
// base-recomp/src/sir.rs
Op::Cmp { rd: Reg, rs: Reg },       // rd - rs, set flags
Op::Test { rd: Reg, rs: Reg },      // rd & rs, set flags (x86)
Op::BranchCond { cond: Cond, target: u64 }, // cond branch
```

### Flag State (semexec)
```rust
// base-recomp/src/semexec.rs
struct Flags {
    n: bool, // negative
    z: bool, // zero
    c: bool, // carry/borrow
    v: bool, // overflow
}
```

### Condições (condicionais mapeadas por ISA)
| Cond | Significado | AArch64 | x86_64 | ARM | PPC | MIPS | SPARC | SH | CF | PA-RISC |
|------|-------------|---------|--------|-----|-----|------|-------|----|----|---------|
| EQ   | Z=1         | EQ      | E/Z    | EQ  | EQ  | EQ   | EQ    | EQ | EQ | =       |
| NE   | Z=0         | NE      | NE/NZ  | NE  | NE  | NE   | NE    | NE | NE | <>      |
| LT   | N!=V        | LT      | L/NGE  | LT  | LT  | LT   | LT    | LT | LT | <       |
| GE   | N==V        | GE      | GE/NL  | GE  | GE  | GE   | GE    | GE | GE | >=      |
| GT   | Z=0 && N==V | GT      | G/NLE  | GT  | GT  | GT   | GT    | GT | GT | >       |
| LE   | Z=1 || N!=V | LE      | LE/NG  | LE  | LE  | LE   | LE    | LE | LE | <=      |
| CS/HS| C=1         | CS/HS   | B/CS/C | CS  | -   | -    | CS    | CS | CS | -       |
| CC/LO| C=0         | CC/LO   | NB/NC  | CC  | -   | -    | CC    | CC | CC | -       |
| MI   | N=1         | MI      | S      | MI  | -   | -    | -     | -  | -  | -       |
| PL   | N=0         | PL      | NS     | PL  | -   | -    | -     | -  | -  | -       |
| VS   | V=1         | VS      | O      | VS  | -   | -    | -     | -  | -  | -       |
| VC   | V=0         | VC      | NO     | VC  | -   | -    | -     | -  | -  | -       |
| HI   | C=1 && Z=0  | HI      | A      | HI  | -   | -    | -     | HI | HI | -       |
| LS   | C=0 || Z=1  | LS      | NA     | LS  | -   | -    | -     | LS | LS | -       |

### Sweep Matrix (por ISA)
- **Cmp/Test**: immediate 0, ±1, max_int, min_int, 0x80000000, 0x7FFFFFFF
- **BranchCond**: taken / not-taken × cada cond × cada cmp result
- **Total**: ~500 casos/ISA

## Fora de Escopo (P6.1+)
- Flag manipulation direta (MSR/MRS, EFLAGS)
- Conditional select/move (CSEL, CMOV)
- Conditional execution ARM (IT block, Thumb)
- Delay-slot annotation em branch (MIPS/SPARC/SH/PA-RISC) — fase separada
- Privileged, MMU, FP, SIMD, atomics, memory ordering

## Ordem de Implementação

| Fase | ISA | Prioridade | Observação |
|------|-----|------------|------------|
| P6.0.1 | AArch64 | Referência | NZCV limpo, 32-bit fixed |
| P6.0.2 | x86_64 | Alto | EFLAGS complexo, var-len |
| P6.0.3 | ARM | Médio | CPSR, IT block futuro |
| P6.0.4 | ColdFire | Médio | CCR subset, 16-bit fixed |
| P6.0.5 | PPC | Médio | CR0-CR7, BO/BI encoding |
| P6.0.6 | MIPS | Delay-slot | SLT/SLTU + BEQ/BNE |
| P6.0.7 | SPARC | Delay-slot | icc/xcc + Bicc |
| P6.0.8 | SuperH | Delay-slot | T bit + BF/BT |
| P6.0.9 | PA-RISC | Delay-slot | cond + COMBT/COMBF |

## Critérios de Aceite (por ISA)
- [ ] Encoder: Cmp/Test + BranchCond → bytes válidos (capstone/llvm-mc)
- [ ] Decoder: bytes → SIR ops exatos
- [ ] Literal: SIR → bytes → SIR idêntico
- [ ] Semantic: executor atualiza flags + PC condicional
- [ ] Differential: exec nativo (qemu-user) = executor
- [ ] Sweep: 100% casos passam

## Dependências
- `base-recomp`: sir.rs, semexec.rs, isa/<arch>.rs
- `base-transpiler`: lowering cond branch
- `base-codegen`: emissão cond branch
- Testes: `base-recomp/tests/conditional_<arch>.rs`

## Entregável Final
`docs/P6_CONDITIONAL.md` + 11 ISAs com sweep 100% + Evidence-Sealed P6.