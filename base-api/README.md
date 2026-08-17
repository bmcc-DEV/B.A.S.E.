# base-api — Canonical v1 (identify · prove · usage)

> Lab meter · `saas_production: false` · **No LLM in proof path**  
> Manifesto: [[01.10 Platform RE Manifesto]] · OpenAPI: [`openapi.yaml`](openapi.yaml)

## Arranque

```bash
export BASE_API_KEY=sk-base-dev-local
export BASE_API_CREDITS=10000
cargo run -p base-api
# → http://127.0.0.1:8787
# → GET /v1/openapi.yaml
```

## Endpoints canónicos

| Método | Path | Auth | Papel |
|--------|------|------|-------|
| POST | `/v1/identify` | Bearer | FW + MMIO → identificação por contrato |
| POST | `/v1/prove` | Bearer | YAML contratos → proof report |
| GET | `/v1/usage` | Bearer | Saldo / consumo |

Meta: `/health` · `/v1/prices` · `/v1/openapi.yaml`

## Pricing (units)

| Item | Units |
|------|-------|
| KiB firmware (ceil, min 1 em identify) | 1 |
| Evento MMIO | 2 |
| Contrato | 10 |

`prove` cobra só contratos. 402 se créditos insuficientes.

## Smoke

```bash
./base-api/examples/smoke_identify.sh
# prove:
curl -s http://127.0.0.1:8787/v1/prove \
  -H "Authorization: Bearer sk-base-dev-local" \
  -H "Content-Type: application/json" \
  -d "{\"contracts_yaml\": $(python3 -c 'import json; print(json.dumps(open(\"examples/pilot_saturn/contracts.yaml\").read()))')}"
```

## Honesty

`auto_fix_complete: false` · `generates_os: false` · Stripe = plug futuro
