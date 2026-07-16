# B.A.S.E. — Estratégia Comercial

> [README.md](README.md) · [LICENSE.md](LICENSE.md) · **Estratégia Comercial**
>
> **Nota v1.1:** oferta **forense** com Specter VM (Forth-like + Lua), maturidade REAL\* host/draft,
> Capstone UART/SPI (RP) + STM32 multi-peripheral + goldens `diff`, reconstruct/study com
> `stop_reason` (≠ auto-fix), HIL host REAL\* (≠ production).
> Port industrial = **consultoria + [SOW v1.1](base-vault/21%20-%20Path%20to%20v1.1/21.21%20-%20SOW%20Industrial%20Checklist.md)**.
> Playbook: [Forensic Playbook v1.1](base-vault/21%20-%20Path%20to%20v1.1/21.20%20-%20Forensic%20Playbook.md).
> Tag: [`v1.1.0-rc`](https://github.com/bmcc-DEV/B.A.S.E./releases/tag/v1.1.0-rc) · estável [`v1.0.0`](https://github.com/bmcc-DEV/B.A.S.E./releases/tag/v1.0.0).
> Claims “PCB drop-in” / “ASIC” / “SaaS turnkey” / “HIL production” / “auto-fix” arquivados.

> Licença: AGPLv3 — uso comercial permitido; modificações em serviço de rede devem ser compartilhadas.

---

## Modelo de Licenciamento

| Uso | Licença | Custo |
|-----|---------|-------|
| Open source / pesquisa | AGPLv3 | Gratuito |
| Empresa ≤ 10 funcionários | AGPLv3 | Gratuito |
| Empresa > 10 funcionários (uso interno) | AGPLv3 | Gratuito (modificações públicas se serviço de rede) |
| **Produto proprietário** | **Comercial** | **Consultar** |
| **Serviço gerenciado (SaaS)** | **Comercial** | **Consultar** — **não** turnkey |

---

## Mercado 1 — Forense / Segurança (**wedge atual**)

### O que entrega
```bash
./examples/pilot/run.sh
./examples/pilot_study/run_study.sh
base study spec.yaml --policy policy.lua -o out/
base hil enumerate -o /tmp/hil/
```

### Não inclui
- Prova criminal sem revisão humana · HIL production · auto-fix · produto industrial completo

### Precificação orientativa
| Serviço | Preço |
|---------|-------|
| Análise + relatório de viabilidade | R$ 5.000 — 8.000 |

---

## Mercado 2 — Preservação Industrial (**consultoria + SOW**)

B.A.S.E. **acelera** diagnóstico e Reference Design. Port completo = engenharia humana + tool.

[SOW Industrial Checklist v1.1](base-vault/21%20-%20Path%20to%20v1.1/21.21%20-%20SOW%20Industrial%20Checklist.md)

| Serviço | Preço |
|---------|-------|
| Análise + viabilidade | R$ 5.000 |
| Port completo | R$ 30.000 — 150.000 |
| Lab HIL host (add-on) | sob SOW |

---

## Mercado 3 — Educação / Pesquisa

| Serviço | Preço |
|---------|-------|
| Licença educacional | R$ 5.000/ano |
| Workshop 2 dias | R$ 20.000 |

---

## Mercado 4 — SaaS (**adiado**)

Não vender PCB/HIL production/auto-fix turnkey.

---

## Próximo passo

1. ✅ Path to v1.0 (`v1.0.0`)
2. ✅ Path to v1.1 → `v1.1.0-rc`
3. Demo: `run.sh` + `run_study.sh`
4. Promoção `v1.1.0` após smoke estável (opcional)
