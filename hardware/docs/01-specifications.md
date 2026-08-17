# Especificações Técnicas — B.A.S.E. Hardware (template)

> Valores abaixo são **alvos de referência** alinhados ao template RP2350 em `base-pcb`. Preencher com medição real após B4.

## 1. Visão geral funcional

Placa mínima de referência: MCU RP2350 + flash QSPI + cristal + USB-C + SWD, capaz de exercitar wedges UART/SPI/I2C do pipeline B.A.S.E. em lab.

## 2. Especificações elétricas

| Parâmetro | Mín | Típico | Máx | Unidade |
|-----------|-----|--------|-----|---------|
| VBUS (USB-C) | 4.5 | 5.0 | 5.5 | V |
| VDD_3V3 | 3.0 | 3.3 | 3.6 | V |
| Corrente repouso | — | TBD | TBD | mA |
| Corrente operação | — | TBD | TBD | mA |
| I/O lógico | 3.0 | 3.3 | 3.6 | V |

## 3. Especificações mecânicas

- Dimensões PCB: TBD (draft force-directed ~100×80 mm em `base-pcb`)
- Espessura: 1.6 mm FR-4 (alvo fab)
- Cobre: 1 oz
- Furos de montagem: TBD (M3)

## 4. Interfaces e conectores

| Ref | Tipo | Função | Notas |
|-----|------|--------|-------|
| J1 | USB-C | Alimentação + dados | template `usb_c` |
| J2 | SWD header | Debug | pinout TBD EE |
| U1 | RP2350A | MCU | template `rp2350_minimal` |
| U2 | W25Q128JV (alvo) | Flash QSPI | template |

## 5. Condições ambientais

- Temperatura operação: TBD (−10 °C a +60 °C típico lab)
- Humidade: 10%–90% sem condensação

## 6. Diagrama de blocos

Ver [02-architecture.md](02-architecture.md). SVG em `images/block-diagram.svg` (a adicionar).

## 7. Decisões de design (resumo)

Ver [06-design-rationale.md](06-design-rationale.md).
