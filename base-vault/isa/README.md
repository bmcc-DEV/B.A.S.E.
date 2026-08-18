---
created: 2026-08-16
updated: 2026-08-16
tags: [isa, preservation, semantics, evidence]
aliases: [ISA Preservation, Architecture Preservation Layer]
---

# ISA Preservation Layer

> Uma arquitetura só é considerada **preservada** quando existir um conjunto verificável
> de artefatos que permitam **recriar, testar e auditar** seu comportamento essencial,
> mesmo na ausência de hardware imediato.
>
> Não é "temos um PDF". É formato + semântica + comportamento + **evidência contínua**.

## Princípio

Preservação real no B.A.S.E. = **modelo + codec + testes + evidência + gaps declarados**.
Cada um destes nasce do código (`base-recomp`) e é **medido, não declarado**:

```text
SIR ──encode──▶ bytes ──decode──▶ SIR′        (formato)
estado + SIR ──▶ estado′                       (semântica)
execute_reference(SIR) == execute_isa(roundtrip(SIR))   (comportamento)
condicional: taken/not-taken × 14 conds          (P6)
```

Fonte da verdade: `base-recomp/src/semantics.rs` (catálogo) + `verify.rs` (cobertura +
níveis) + `semexec.rs` (diferencial/sweep). Este diretório guarda **snapshots** gerados
por `base recomp report`, nunca tabelas escritas à mão (elas driftam).

## As 3 camadas de confiança

| Camada | Pergunta | Evidência |
|--------|----------|-----------|
| **Formato** | o binário é reconhecido e reconstruído? | `round-trip` literal (`encode → decode → mesma instrução`) |
| **Semântica** | o efeito da instrução é modelado? | equivalência `semantic_key` (registradores/immediates/PC) |
| **Comportamento** | o estado visível preserva? | `differential` com memória + endian + push/pop + sweep gerado |
| **Condicional** | decisões condicionais preservam? | sweep taken/not-taken × 14 conds × 3 outcomes (P6) |

## 18 SIR op kinds

Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec, Push, Pop, LdMem, StMem, CallRel, JmpRel, Cmp, Test, BranchCond, Trap.

ISAs com flags (18/18): x86_64, AArch64, ARM, ColdFire, PPC, SPARC.
ISAs sem flags (15/18): MIPS, SuperH, Alpha, PA-RISC (Trap emit sem Cmp/Test/BC).
Pending (emit only): M88k, IA-64, i860.

## Níveis de preservação P0–P6

Bandas objetivas, derivadas de números medidos (`verify.rs::preservation_level`):

| Nível | Nome | Condição medida |
|-------|------|-----------------|
| P0 | Identified | alvo existe (nome) |
| P1 | Documented | entrada no catálogo semântico (identidade + gaps) |
| P2 | Format-preserved | decoder existe, round-trip literal > 0 |
| P3 | Semantic-preserved | equivalência semântica > 0 |
| P4 | Behavior-preserved | differential > 0 **e** semantic ≥ 33% |
| P5 | Evidence-sealed | differential ≥ 67% **e** semantic ≥ 67% **e** sweep selado |
| P6 | Conditional-preserved | 100% em todas as dimensões **e** sweep condicional limpo |

## Níveis por ISA (agosto 2026)

| ISA | Nível | Encode | Decode | Flags | Trap |
|-----|-------|--------|--------|-------|------|
| x86_64 | **P6.1** | 18/18 | ✅ | EFLAGS (cf/pf/af/zf/sf/of) | ✅ ud2 |
| AArch64 | **P6.1** | 18/18 | ✅ | NZCV | ✅ brk |
| ARM | **P6.1** | 18/18 | ✅ | CPSR NZCV | ✅ bkpt |
| ColdFire | **P6.1** | 18/18 | ✅ | CCR (z/n/c/v/x) | ✅ illegal |
| PPC | **P6.1** | 18/18 | ✅ | CR0 (lt/gt/eq/so) | ✅ trap |
| SPARC | **P6.1** | 18/18 | ✅ | icc (z/n/c/v) | ✅ ta 1 |
| MIPS | **P5.1** | 15/18 | ✅ | — sem flags | ✅ break |
| SuperH | **P5.1** | 15/18 | ✅ | T flag | ✅ trapa |
| Alpha | **P5.1** | 15/18 | ✅ | — sem flags | ✅ call_pal |
| PA-RISC | **P5.1** | 15/18 | ✅ | — sem flags | ✅ break 0,0 |
| M88k | P1 | emit | — | — | ✅ trap 1 |
| IA-64 | P1 | emit | — | — | ✅ break.i 0 |
| i860 | P1 | emit | — | — | ✅ bpt |

## Regras de ouro

1. Sem teste, não é preservação.
2. Sem decoder, não há round-trip.
3. Sem diferencial, não há comportamento.
4. Sem gap documentado, há overclaim.
5. Sem evidence pack (relatório gerado), não há preservação auditável.
6. Sem sweep condicional, não há P6.

## Geração de evidência

```bash
base recomp report --isa mips      # relatório de uma ISA (evidência, não prosa)
base recomp report --matrix        # matriz de preservação (nível por ISA)
base recomp report                 # matriz + relatórios de todas as ISAs
base recomp verify --all           # cobertura por dimensão (18 kinds)
base recomp verify --sweep --target coldfire   # sweep comportamental + condicional
```

Snapshots gerados: [`report.md`](report.md) (toda ISA).

## Mapa

| Pasta/arquivo | Conteúdo |
|---------------|----------|
| `README.md` | este documento (princípios, camadas, níveis, regras) |
| `report.md` | **snapshot gerado** — matriz + relatório por ISA (nunca editar à mão) |

Relacionado: [[29 - Path to v1.9/29.13 - Semantic Preservation]] · [`docs/STATIC_RECOMP.md`](../../docs/STATIC_RECOMP.md)
