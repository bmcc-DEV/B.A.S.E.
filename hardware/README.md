# B.A.S.E. Hardware — Pacote PCB (OSHWA-style)

> **Versão pacote:** v0.1.0-scaffold | **Data:** 2026-07-20 | **Status:** `engineering_draft` — **NOT FABRICABLE**

Pacote de documentação e artefatos de PCB alinhado a práticas OSHWA / KiCad.
Destino dos drafts gerados por `base pcb` e do trabalho de EE até o **Claim B** do [SOW Industrial Gate](../base-vault/22%20-%20Path%20to%20v1.2/22.30%20-%20SOW%20Industrial%20Gate.md).

## Honesty

| Claim | Estado |
|-------|--------|
| Gerbers para fábrica | **bloqueado** até B2–B5 (EE + DRC + 1ª placa + aceite) |
| `base pcb` CLI | gera **engineering_draft** com banner `NOT FABRICABLE` |
| BOM com MPN reais | template apenas; Manufacturer=TBD nos drafts |
| Fabricável no README raiz | **não** — ver honesty do projeto |

## O que é?

Estrutura canónica para levar um design B.A.S.E. (referência RP2350 / USB-C / wedge) de *draft orquestrado* a *projeto sob engenharia humana*. Qualquer pessoa com este diretório deve, **quando o gate estiver verde**, fabricar, montar, testar e modificar a placa sem falar com o autor.

## Especificações rápidas (alvo de referência)

| Parâmetro | Valor (alvo) |
|-----------|----------------|
| MCU | RP2350A (template `base-pcb`) |
| Alimentação | 3.3 V lógica / USB-C 5 V entrada |
| Camadas | 2 (draft) |
| Status DRC / ERC | pendente EE |
| Banner silkscreen | `engineering_draft — NOT FABRICABLE` |

## Documentação

| Doc | Conteúdo |
|-----|----------|
| [00-overview](docs/00-overview.md) | Propósito, público, fluxo draft → fab |
| [01-specifications](docs/01-specifications.md) | Specs elétricas/mecânicas (template) |
| [02-architecture](docs/02-architecture.md) | Diagrama de blocos |
| [03-assembly-guide](docs/03-assembly-guide.md) | Montagem (preencher após B4) |
| [04-testing-procedures](docs/04-testing-procedures.md) | Validação |
| [05-troubleshooting](docs/05-troubleshooting.md) | Problemas conhecidos |
| [06-design-rationale](docs/06-design-rationale.md) | Porquê das decisões |
| [CHECKLIST](docs/CHECKLIST.md) | Gate B1–B5 + release |
| [fabrication-notes](fabrication/fabrication-notes.md) | Notas de fábrica |

## Ponte com o software

```text
base analyze/synth → base pcb --output hardware/kicad/
        ↓
  *.kicad_sch / *.kicad_pcb  (draft + banner)
        ↓
  EE review (B2–B5) → scripts/export-gerbers.sh
        ↓
  gerbers/ + bom/ + assembly/  (só após gate verde)
```

- Crate: [`base-pcb`](../base-pcb/)
- Spec S-expr: [`base-vault/03.03`](../base-vault/03%20-%20Technical%20Specs/03.03%20KiCad%20S-Expression.md)
- Índice docs projeto: [`docs/hardware/README.md`](../docs/hardware/README.md)

## Licença (quando houver PCB sob EE)

| Camada | Licença prevista |
|--------|------------------|
| Hardware (sch/pcb/gerber) | CERN-OHL-S-2.0 (a confirmar no release) |
| Documentação | CC BY-SA 4.0 |
| Firmware neste pacote | MIT (se aplicável) |

Código do motor B.A.S.E. permanece sob a licença do repositório raiz ([LICENSE.md](../LICENSE.md)).

## Scripts

```bash
# Smoke completo (recomendado — sem placeholders)
./hardware/scripts/run_smoke.sh

# Ou passo a passo com caminhos reais:
cargo run -p base-cli -- pcb hardware/fixtures/synthesized_spec.yaml \
  --project BASE -o /tmp/pcb_out
./hardware/scripts/ingest-base-pcb.sh /tmp/pcb_out BASE
./hardware/scripts/validate-project.sh

./hardware/scripts/export-gerbers.sh          # falha se NOT FABRICABLE
./hardware/scripts/export-gerbers.sh --force-draft  # só rascunho
./hardware/scripts/generate-bom.sh
```

Não use `<spec.yaml>` no shell — os `<…>` são notação de documentação; o bash interpreta como redirecionamento.