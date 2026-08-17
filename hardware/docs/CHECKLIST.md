# Checklist — documentação e Claim B (PCB fabricável)

Fonte contratual: [SOW Industrial Gate § Claim B](../../base-vault/22%20-%20Path%20to%20v1.2/22.30%20-%20SOW%20Industrial%20Gate.md).

## Gate B (promover só com todos verdes)

| ID | Critério | Verde se | Estado |
|----|----------|----------|--------|
| B1 | Draft base | sch/pcb com pins; banner removido **só** após B2–B5 | ☐ scaffold |
| B2 | Footprints | DB revisado por EE | ☐ |
| B3 | DRC / netlist | DRC + netlist elétrico revisado | ☐ |
| B4 | Fab | 1ª placa montada / green-wire documentado | ☐ |
| B5 | Aceite | EE + Cliente assinam release Gerber | ☐ |

## Arquivos de design (FONTE)

- [ ] Esquema KiCad (`.kicad_sch`) — ERC passou
- [ ] Layout KiCad (`.kicad_pcb`) — DRC passou
- [ ] Bibliotecas customizadas em `kicad/libs/`
- [ ] Regras (`.kicad_dru`)
- [ ] Projeto (`.kicad_pro`) versionado em `hardware/kicad/`

## Fabricação

- [ ] Gerbers RS-274X (todas as camadas)
- [ ] Drill Excellon (PTH + NPTH)
- [ ] Job file (`.gbrjob`)
- [ ] `fabrication/fab-drawing.pdf` + `fabrication-notes.md`

## Montagem

- [ ] BOM CSV com MPN reais (não TBD)
- [ ] BOM interativo HTML
- [ ] Pick & Place top + bottom
- [ ] Assembly drawing + STEP 3D

## Documentação

- [x] README pacote + overview
- [x] Specs / architecture / assembly / test / troubleshooting / rationale (esqueleto)
- [ ] Fotos/renders reais
- [ ] Troubleshooting pós-protótipo

## Legal / meta

- [ ] Licença hardware (CERN-OHL-S-2.0) no release do pacote
- [ ] Licença docs (CC BY-SA)
- [ ] Entrada no CHANGELOG raiz
- [ ] Tag git do pacote (ex. `hardware-v1.0.0`) **após** B5

## Honesty

- [x] Não afirmar fabricável no README raiz enquanto B incompleto
- [x] Scripts recusam export “release” se banner `NOT FABRICABLE` presente (salvo `--force-draft`)
