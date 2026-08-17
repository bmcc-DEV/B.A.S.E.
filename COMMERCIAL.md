# B.A.S.E. — Estratégia Comercial

> [README.md](README.md) · **Estratégia Comercial**
>
> **Nota v1.4:** forense + Industrial Gate + HIL Lab +
> **[OS Port Validation Assist](base-vault/24%20-%20Path%20to%20v1.4/24.00%20-%20Index.md)**
> (Moto G35 / iMac G3) — ≠ port ReactOS/TaurOS turnkey / PCB fab / HIL production.
> Tags: [`v1.4.0-rc`](https://github.com/bmcc-DEV/B.A.S.E./releases/tag/v1.4.0-rc) · [`v1.3.0-rc`](https://github.com/bmcc-DEV/B.A.S.E./releases/tag/v1.3.0-rc).

---

## Mercados

| Mercado | Entrega |
|---------|---------|
| Forense | `run.sh` / `run_study.sh` |
| OS-port assist | G35 / iMac fases A→B→C sob SOW |
| Industrial | Gate → HIL lab → PCB eng. / fix parcial |
| SaaS / Identify API | **EXPERIMENTAL** lab meter (`base-api`) — `saas_production: false` |

```bash
./examples/pilot_moto_g35/run.sh
./examples/pilot_imac_g3/run.sh
cargo run -p base-api   # POST /v1/identify — pay-as-you-go units
```

## Próximo

1. ✅ `v1.4.0-rc` OS-port assist  
2. Lab Cliente: dumps reais + TaurOS/ReactOS externos → B/C  
3. Path PCB eng. (Gate B) sob demanda  
4. `base-api` Identify — Stripe/prepaid quando houver cliente (hoje créditos locais)  
