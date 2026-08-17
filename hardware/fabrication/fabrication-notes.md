# Notas de Fabricação — B.A.S.E. Hardware

> **Status:** rascunho. Não enviar a fábrica enquanto o silkscreen/title block contiver `NOT FABRICABLE` e o Claim B estiver incompleto.

## Stackup (2 camadas — alvo)

| Camada | Material | Espessura |
|--------|----------|-----------|
| Top Copper | 1 oz (35 µm) | 35 µm |
| Substrate | FR-4 | ~1.53 mm |
| Bottom Copper | 1 oz (35 µm) | 35 µm |
| **Total** | | **1.6 mm** |

## Especificações

- **Material:** FR-4, Tg ≥ 130 °C
- **Acabamento:** HASL lead-free (ENIG se fine-pitch QFN exigir)
- **Máscara:** verde (default) / a especificar
- **Serigrafia:** branco
- **Tolerância contorno:** ±0.15 mm
- **Furo mínimo:** 0.3 mm
- **Trilha/espaço mín.:** 0.15 mm / 0.15 mm
- **Impedância controlada:** não (2L draft)

## Requisitos especiais

- [ ] Vias tented
- [ ] Sem cobre sob furos de montagem M3
- [ ] Fiducials SMD (3 topo, 3 fundo) se montagem automática
- [ ] Remover texto `NOT FABRICABLE` do silkscreen antes do release

## Contacto EE

TBD — preencher no SOW do cliente.
