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

## Níveis de preservação P0–P5

Bandas objetivas, derivadas de números medidos (`verify.rs::preservation_level`):

| Nível | Nome | Condição medida |
|-------|------|-----------------|
| P0 | Identified | alvo existe (nome) |
| P1 | Documented | entrada no catálogo semântico (identidade + gaps) |
| P2 | Format-preserved | decoder existe, round-trip literal > 0 |
| P3 | Semantic-preserved | equivalência semântica > 0 |
| P4 | Behavior-preserved | differential > 0 **e** semantic ≥ 33% |
| P5 | Evidence-sealed | differential ≥ 67% **e** semantic ≥ 67% **e** sweep selado (mismatches só com causa documentada) |

## Regras de ouro

1. Sem teste, não é preservação.
2. Sem decoder, não há round-trip.
3. Sem diferencial, não há comportamento.
4. Sem gap documentado, há overclaim.
5. Sem evidence pack (relatório gerado), não há preservação auditável.

## Geração de evidência

```bash
base recomp report --isa mips      # relatório de uma ISA (evidência, não prosa)
base recomp report --matrix        # matriz de preservação (nível por ISA)
base recomp report                 # matriz + relatórios de todas as ISAs
base recomp verify --all           # cobertura por dimensão
base recomp verify --sweep --target coldfire   # matriz gerada de comportamentos
```

Snapshots gerados: [`report.md`](report.md) (toda ISA).

## Mapa

| Pasta/arquivo | Conteúdo |
|---------------|----------|
| `README.md` | este documento (princípios, camadas, níveis, regras) |
| `report.md` | **snapshot gerado** — matriz + relatório por ISA (nunca editar à mão) |

Relacionado: [[29 - Path to v1.9/29.13 - Semantic Preservation]] · [`docs/STATIC_RECOMP.md`](../../docs/STATIC_RECOMP.md)
