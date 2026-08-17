# Visão geral — Pacote Hardware B.A.S.E.

## O que é

Pacote de documentação e fontes KiCad para placas de referência derivadas do pipeline B.A.S.E. (`base-pcb`). Não é um produto de consumo: é o **caminho orgânico** de draft → revisão EE → fabricação.

## Público-alvo

| Persona | Usa para |
|---------|----------|
| EE / layout | Abrir `kicad/`, corrigir footprints, passar DRC/ERC |
| Lab / HIL | Montar após B4, seguir testes e troubleshooting |
| Cliente SOW | Verificar Claim B no Industrial Gate |
| Contribuidor open-hardware | Reproduzir e modificar após release fabricável |

## Fluxo orgânico (iterativo)

```text
FASE 1 CONCEITO     → README + 00-overview + diagrama rascunho
FASE 2 ESQUEMA      → 01-specifications parcial + .kicad_sch (draft)
FASE 3 LAYOUT       → .kicad_pcb + 06-design-rationale
FASE 4 PROTÓTIPO    → fotos + 05-troubleshooting + 04-testing
FASE 5 REVISÃO EE   → BOM MPN + Gerbers + fab notes (gate B2–B5)
FASE 6 RELEASE      → tag + CHANGELOG + remover banner NOT FABRICABLE
```

## Estado atual

**FASE 1–2 (scaffold).** Artefatos em `gerbers/`, `bom/` com MPN e `assembly/` ficam vazios ou `.gitkeep` até o gate. O CLI `base pcb` continua a emitir apenas `engineering_draft — NOT FABRICABLE`.
