# B.A.S.E. — Estratégia Comercial

> [README.md](README.md) · [LICENSE.md](LICENSE.md) · **Estratégia Comercial**
>
> **Nota v0.2 (Path to Real):** ofertas abaixo distinguem o que o software entrega
> sozinho vs. o que exige engenheiro humano. Claims de “PCB drop-in” / “ASIC substituído”
> estão **arquivados** até o piloto R6 e aceite de cliente industrial.

> Licença: AGPLv3 — uso comercial permitido; modificações em serviço de rede devem ser compartilhadas.
> Uso proprietário fechado: licença comercial (consultar).

---

## Modelo de Licenciamento

| Uso | Licença | Custo |
|-----|---------|-------|
| Open source / pesquisa | AGPLv3 | Gratuito |
| Empresa ≤ 10 funcionários | AGPLv3 | Gratuito |
| Empresa > 10 funcionários (uso interno) | AGPLv3 | Gratuito (modificações públicas se serviço de rede) |
| **Produto proprietário** | **Comercial** | **Consultar** |
| **Serviço gerenciado (SaaS)** | **Comercial** | **Consultar** |

---

## Mercado 1 — Forense / Segurança (**wedge atual**)

### Problema
Analisar firmware embedded sem código-fonte: IoT, roteadores, sensores.

### O que B.A.S.E. entrega hoje
```bash
base analyze firmware.bin --disasm --dot -o analysis/
base design analysis/hardware_spec.yaml -o analysis/design/
base replay trace.csv --contracts contracts.yaml
# → Evidence DB, HardwareSpec, Reference Design, violações de contrato
```

### Não inclui (ainda)
- Prova criminal “pronta para tribunal” sem revisão humana
- Z3 formal em todas as builds (simbólico default; Z3 opcional)

### Precificação orientativa
| Serviço | Preço |
|---------|-------|
| Análise + relatório de viabilidade | R$ 5.000 — 8.000 |
| Scan / assinatura (quando SaaS existir) | sob proposta |

---

## Mercado 2 — Preservação Industrial (**consultoria + tool**)

### Problema
ASICs legados sem reposição.

### Posicionamento honesto
B.A.S.E. **acelera** diagnóstico e Reference Design. Port completo (PCB fabricável + FW em silício + certificação) é **projeto de engenharia** com humanos no loop — não um botão `pipeline`.

```bash
base analyze firmware.bin --disasm -o study/
base design study/hardware_spec.yaml -o study/design/
# → insumos para engenheiro; PCB gerado = engineering draft
```

### Precificação orientativa
| Serviço | Preço |
|---------|-------|
| Análise + relatório de viabilidade | R$ 5.000 |
| Port completo (time humano + tool) | R$ 30.000 — 150.000 |
| Suporte anual BOM | R$ 10.000/ano |

---

## Mercado 3 — Educação / Pesquisa

### Solução
Pipeline visual (DOT/Mermaid), contratos, métrica Ψ — ver [examples/pilot](examples/pilot/).

| Serviço | Preço |
|---------|-------|
| Licença educacional (instituição) | R$ 5.000/ano |
| Workshop 2 dias | R$ 20.000 |

---

## Mercado 4 — SaaS (**depois do R6**)

Adiado até existir piloto documentado e Maturity Matrix estável.
Não vender “PCB + firmware prontos” no plano Starter.

---

## Canais

| Canal | Foco |
|-------|------|
| GitHub / vault Obsidian | Transparência técnica |
| Eventos de segurança | Demo forense com piloto |
| Parcerias acadêmicas | Ψ + paleocomputação |
| Cases G5 / Xbox / Alpha | Pesquisa — **não** claim de produto |

---

## Próximo passo imediato

1. Executar Path to Real R0–R6 (`base-vault/12 - Path to Real/`)
2. Publicar case study do wedge MCU ARM
3. Só então reabrir pricing SaaS / port turnkey
