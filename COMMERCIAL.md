# B.A.S.E. — Estratégia Comercial

> [README.md](README.md) · [LICENSE.md](LICENSE.md) · **Estratégia Comercial**
>
> **Nota v0.4 (`v0.4.0-rc`):** oferta **forense** com Capstone UART + SPI opt-in (`run_t1_b2.sh`),
> Z3 opcional (`formal.yml` + `proof_report.backend`), PCB pin-aware UART/SPI rotulado,
> HIL EXPERIMENTAL (`try_flash` / mock_dry_run ≠ silício).
> Port industrial = **consultoria + [SOW](base-vault/13%20-%20Path%20to%20v0.3/13.21%20-%20SOW%20Industrial%20Template.md)**.
> Playbook: [Forensic Playbook](base-vault/13%20-%20Path%20to%20v0.3/13.20%20-%20Forensic%20Playbook.md) · plano [v0.4](base-vault/14%20-%20Path%20to%20v0.4/14.00%20-%20Index.md).
> Claims “PCB drop-in” / “ASIC substituído” / “SaaS turnkey” / “HIL flasheou” continuam arquivados.

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
| **Serviço gerenciado (SaaS)** | **Comercial** | **Consultar** — **não** disponível como turnkey em v0.3 |

---

## Mercado 1 — Forense / Segurança (**wedge atual**)

### Problema
Analisar firmware embedded sem código-fonte: IoT, roteadores, sensores.

### O que B.A.S.E. entrega hoje
```bash
./examples/pilot/run.sh   # ou run_v03.sh (alias documentado)
base analyze firmware.bin --disasm --dot -o analysis/
base design analysis/hardware_spec.yaml -o analysis/design/
base replay trace.csv --contracts contracts.yaml
# → Evidence DB, HardwareSpec, Reference Design, violações de contrato
```

Demo guiada: [Playbook](base-vault/13%20-%20Path%20to%20v0.3/13.20%20-%20Forensic%20Playbook.md) ·
[Case study](base-vault/12%20-%20Path%20to%20Real/12.20%20-%20Pilot%20Case%20Study.md).

### Não inclui (ainda)
- Prova criminal “pronta para tribunal” sem revisão humana
- Z3 formal em todas as builds (simbólico default; Z3 via `solver_z3` + [formal.yml](.github/workflows/formal.yml))
- Flash HIL automático sem probe detectado

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
B.A.S.E. **acelera** diagnóstico e Reference Design. Port completo (PCB fabricável + FW em silício + certificação) é **projeto de engenharia** com humanos no loop — não um botão `pipeline`.

Use o [SOW Industrial Template](base-vault/13%20-%20Path%20to%20v0.3/13.21%20-%20SOW%20Industrial%20Template.md): escopo / não-escopo / aceite.

```bash
base analyze firmware.bin --disasm -o study/
base design study/hardware_spec.yaml -o study/design/
# → insumos para engenheiro; PCB gerado = engineering draft (pins anotados no wedge RP2040)
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

## Mercado 4 — SaaS (**ainda adiado em v0.3**)

Piloto + playbook existem; SaaS permanece adiado até retenção / ops.
Não vender “PCB + firmware prontos” nem HIL “plug-and-flash” no plano Starter.

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

1. ✅ Path to Real R0–R6 + case study v0.2
2. ✅ Path to v0.3 S0–S5 · tag `v0.3.0-rc`
3. ✅ Path to v0.4 T0–T5 · tag `v0.4.0-rc`
4. Demo forense: `./examples/pilot/run.sh` + opt-in `./examples/pilot/run_t1_b2.sh`
5. Pricing SaaS / port turnkey só com aceite industrial explícito (SOW)
