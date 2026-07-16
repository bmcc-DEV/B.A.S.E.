# base-hil — **EXPERIMENTAL**

Template de probe HIL (host agent + gerador de firmware stub RP2350).

| Claim | Status |
|-------|--------|
| Compila no host sem hardware | ✅ `cargo test -p base-hil` |
| Captura USB real / CMSIS-DAP | ❌ stub simulado |
| Flash automático sem probe | ❌ **`FlashDenied::NotDetected`** |
| Path Detected offline | ✅ `with_presence(Detected)` / `BASE_HIL_MOCK_DETECTED` |
| Dry-run flash (sem silício) | ✅ `with_mock_flash(Detected)` → `mock_dry_run` |
| Ligado ao `base pipeline` default | ❌ não |

## Uso

```bash
cargo test -p base-hil
cargo build -p base-hil
```

```rust
use base_hil::{HilAgent, ProbePresence};

// CI / default
let a = HilAgent::connect(0xCAFE, 0x4007)?; // Simulated
assert!(a.try_flash(&[0]).is_err());

// Offline Detected (testes) — programador real ainda ausente
let d = HilAgent::with_presence(ProbePresence::Detected);
assert!(d.try_flash(&[0]).is_err()); // ProgrammerUnimplemented

// Dry-run explícito (ainda ≠ silício)
let m = HilAgent::with_mock_flash(ProbePresence::Detected);
let receipt = m.try_flash(&[1, 2, 3])?;
assert_eq!(receipt.mode, "mock_dry_run");
```

Env opcional: `BASE_HIL_MOCK_DETECTED=1` faz `enumerate_presence` / `connect` retornar Detected **sem USB**.

## Requisitos futuros (fora de v0.4 T4)

- Enumerate USB real / CMSIS-DAP
- Programador sob Detected sem `mock_flash`
- CLI `base hil …` (não existe)

← vault: [Sprint T4](../base-vault/14%20-%20Path%20to%20v0.4/14.14%20-%20Sprint%20T4%20HIL%20Detected.md)
