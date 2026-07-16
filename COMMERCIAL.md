# B.A.S.E. — Estratégia Comercial

> [README.md](README.md) · [LICENSE.md](LICENSE.md) · **Estratégia Comercial**
>
> **Nota v0.6 (Path):** oferta **forense** com Capstone UART/SPI (RP) + STM32 USART1
> (Capstone sintético + pins PA9/PA10 + draft sch), Z3 opcional,
> HIL EXPERIMENTAL (`base hil` / `hil_usb` / `hil_programmer` — ≠ production).
> Port industrial = **consultoria + [SOW v0.6](base-vault/16%20-%20Path%20to%20v0.6/16.21%20-%20SOW%20Industrial%20Checklist.md)**.
> Playbook: [Forensic Playbook v0.6](base-vault/16%20-%20Path%20to%20v0.6/16.20%20-%20Forensic%20Playbook.md) · [CHANGELOG](CHANGELOG.md).
> Claims “PCB drop-in” / “ASIC substituído” / “SaaS turnkey” / “HIL production” continuam arquivados.

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
| **Serviço gerenciado (SaaS)** | **Comercial** | **Consultar** — **não** disponível como turnkey |

---

## Mercado 1 — Forense / Segurança (**wedge atual**)

### Problema
Analisar firmware embedded sem código-fonte: IoT, roteadores, sensores.

### O que B.A.S.E. entrega hoje
```bash
./examples/pilot/run.sh
./examples/pilot/run_t1_b2.sh
./examples/pilot_stm32/run.sh
base hil enumerate -o /tmp/hil/
base hil flash /tmp/x.bin --mock-flash -o /tmp/hil/
```

Demo: [Playbook v0.6](base-vault/16%20-%20Path%20to%20v0.6/16.20%20-%20Forensic%20Playbook.md) ·
[Case study](base-vault/12%20-%20Path%20to%20Real/12.20%20-%20Pilot%20Case%20Study.md).

### Não inclui (ainda)
- Prova criminal “pronta para tribunal” sem revisão humana
- Z3 formal em todas as builds
- Flash HIL automático / “production”
- Substituição do gate RP pelo STM32

### Precificação orientativa
| Serviço | Preço |
|---------|-------|
| Análise + relatório de viabilidade | R$ 5.000 — 8.000 |
| Scan / assinatura (quando SaaS existir) | sob proposta |

---

## Mercado 2 — Preservação Industrial (**consultoria + SOW**)

### Problema
ASICs / MCUs legados sem reposição.

### Posicionamento honesto
B.A.S.E. **acelera** diagnóstico e Reference Design (RP e/ou STM32). Port completo é **projeto de engenharia** com humanos no loop.

Use o [SOW Industrial Checklist v0.6](base-vault/16%20-%20Path%20to%20v0.6/16.21%20-%20SOW%20Industrial%20Checklist.md).

```bash
base analyze firmware.bin --mmio-traces mmio.json --classify uart -o study/
base design study/hardware_spec.yaml --preferred-manufacturer STMicroelectronics -o study/design/
# → insumos para engenheiro; PCB gerado = engineering draft NOT FABRICABLE
```

### Precificação orientativa
| Serviço | Preço |
|---------|-------|
| Análise + relatório de viabilidade | R$ 5.000 |
| Port completo (time humano + tool) | R$ 30.000 — 150.000 |
| Lab HIL EXPERIMENTAL (add-on) | sob SOW §7 |
| Suporte anual BOM | R$ 10.000/ano |

---

## Mercado 3 — Educação / Pesquisa

### Solução
Pipeline visual (DOT/Mermaid), contratos, métrica Ψ — ver [examples/pilot](examples/pilot/) e [pilot_stm32](examples/pilot_stm32/).

| Serviço | Preço |
|---------|-------|
| Licença educacional (instituição) | R$ 5.000/ano |
| Workshop 2 dias | R$ 20.000 |

---

## Mercado 4 — SaaS (**adiado**)

Playbook existe; SaaS permanece adiado.
Não vender “PCB + firmware prontos” nem HIL “plug-and-flash”.

---

## Canais

| Canal | Foco |
|-------|------|
| GitHub / vault Obsidian | Transparência técnica |
| Eventos de segurança | Demo forense (RP + STM32) |
| Parcerias acadêmicas | Ψ + paleocomputação |
| Cases G5 / Xbox / Alpha | Pesquisa — **não** claim de produto |

---

## Próximo passo imediato

1. ✅ Path to Real → v0.5 (`v0.5.0`)
2. ✅ Path to v0.6 V0–V4 (Capstone/pins/`base hil` + oferta)
3. Demo: `run.sh` + `pilot_stm32` + `base hil enumerate`
4. Pricing SaaS / port turnkey só com SOW
