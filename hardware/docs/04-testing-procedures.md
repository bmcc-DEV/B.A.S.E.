# Procedimentos de teste

> Preencher com medição real após protótipo. Integra com HIL lab (`base-hil`) sob SOW — ≠ flash production no CI.

## Smoke elétrico (pré-firmware)

| # | Teste | Esperado | Pass? |
|---|-------|----------|-------|
| 1 | Continuidade GND | malha contínua | ☐ |
| 2 | Curto 3V3–GND | open | ☐ |
| 3 | VBUS → 3V3 | LDO sobe 3.3 V ±5% | ☐ |
| 4 | SWD detect | probe Detected | ☐ |

## Smoke funcional

| # | Teste | Método | Pass? |
|---|-------|--------|-------|
| 1 | Enum USB | host vê dispositivo | ☐ |
| 2 | UART wedge | `examples/` smoke | ☐ |
| 3 | Flash QSPI | boot / ID JEDEC | ☐ |

## Registo

Guardar logs e fotos em `docs/images/` e receipts HIL no lab do cliente (não no CI default).
