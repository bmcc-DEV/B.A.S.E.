# B.A.S.E. Platform — Automated RE (HW / FW)

> **Evidence-driven reverse engineering — contracts, Ψ, own API. No AI in the core.**  
> Manifesto: [`01.10 Platform RE Manifesto`](../base-vault/01%20-%20Architecture/01.10%20Platform%20RE%20Manifesto.md)

B.A.S.E. is an **evidence-assisted** RE platform: Hardware-facing perception + structured reasoning (**without** Transformers on the proof path).

## Honesty

- `generates_os: false`
- `auto_fix_complete: false`
- `saas_production: false` (API lab meter until Stripe)
- Flash = lab assist / manual — receipts never `production`
- No Transformers / ONNX in the reasoning/proof path
- ≠ “magic RE of any binary”

## Division

| Side | Role | Crates |
|------|------|--------|
| **Hardware-facing** | Acquire immutable evidence | `specterprobe`, `base-virt`, `base-port`, `base-hil`, `base-core` evidence |
| **Software reasoning** | Questions → beliefs → hypotheses → triad | `base-reason` |
| **API** | Pay-as-you-go identify / prove / usage | `base-api` |

Loop: **observe → ask → hypothesize → lab/receipt → strengthen/forget**.

## Canonical API v1

| Endpoint | Role |
|----------|------|
| `POST /v1/identify` | FW + MMIO → per-contract identification |
| `POST /v1/prove` | Contracts YAML → SMT/symbolic proof |
| `GET /v1/usage` | Credits / units |

```bash
cargo run -p base-api
# GET /v1/openapi.yaml
```

Spec: [`base-api/openapi.yaml`](../base-api/openapi.yaml)

## CLI

```bash
./target/debug/base reason g35 -o output/reason
./target/debug/base reason report --wedge path/to/wedge_mmio_map.yaml --format json
```

## Out of scope (core)

Dreamcast/Minecraft game turnkey, OS turnkey, production flash, LLM-in-proof-path.
