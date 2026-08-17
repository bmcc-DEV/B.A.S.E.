# Guia de montagem

> **Pré-requisito:** Claim B4 (1ª placa / green-wire documentado). Enquanto `NOT FABRICABLE`, este guia é esqueleto.

## Materiais

1. PCB fabricada a partir de `gerbers/` (após gate)
2. BOM: `bom/BASE-bom.csv` com MPN reais
3. Pasta de solda / ferro / estação ar quente conforme pacotes
4. Interactive BOM (quando gerado): `bom/BASE-interactive-bom.html`

## Passos (esqueleto)

1. Inspecionar PCB (contorno, máscara, furos, fiducials)
2. Montar passivos (C/R) — lado indicado no pos
3. Montar U1 (RP2350) e U2 (flash) — reflow
4. Montar USB-C e headers SWD
5. Limpeza e inspeção óptica
6. Seguir [04-testing-procedures.md](04-testing-procedures.md)

## Pick & Place

Após export: `assembly/BASE-top.pos`, `assembly/BASE-bottom.pos`.
