# Golden notes — reference_design (R3)

Campos estáveis após `./examples/pilot/run.sh`:

| Campo | Esperado no wedge |
|-------|-------------------|
| `architecture.cpu.part` | MCU com `uart` (tipicamente `RP2040`, preço mínimo) |
| `architecture.cpu.interface` | `uart` |
| `contracts.satisfied / total` | ≥ 70% (piloto: 3/3) |
| `validation.contracts_verified` | `true` |
| `validation.status` | `contracts_satisfied` |
| `bom.estimated_cost` | > 0 e ≤ 80 (synth budget) |
| Não | `ECP5-*`, `TBD`, `unassigned` |

Regressão: `cargo test -p base-core --test pilot_design`.
