# base-hil — **EXPERIMENTAL**

Template de probe HIL (host agent + gerador de firmware stub RP2350).

| Claim | Status |
|-------|--------|
| Compila no host sem hardware | ✅ `cargo test -p base-hil` |
| Enumerate USB real | ✅ feature `hil_usb` (rusb) — **não** no CI default |
| Flash automático sem probe | ❌ **`FlashDenied::NotDetected`** |
| Path Detected offline | ✅ `with_presence(Detected)` / `BASE_HIL_MOCK_DETECTED` |
| Dry-run flash (sem silício) | ✅ `with_mock_flash(Detected)` → `mock_dry_run` |
| Programador USB/externo | ✅ feature `hil_programmer` — **EXPERIMENTAL**, ≠ production |
| CLI `base hil` | ✅ V3 — `enumerate` / `flash` / `lab-status` (≠ pipeline default) |
| Ligado ao `base pipeline` default | ❌ não |

## CLI (V3)

```bash
base hil enumerate -o out/                 # default → Simulated
base hil flash fw.bin --mock-flash -o out/ # mock_dry_run
base hil lab-status --sop examples/hil_lab/SOP.md --mock-detected -o out/
# cargo build -p base-cli --features hil_programmer,hil_usb
# ./examples/hil_lab/run_hil_lab_assist.sh  # A1/A2 GREEN rehearsal
```

## Enumerate (U2)

1. `BASE_HIL_MOCK_DETECTED` → `Detected`
2. Feature `hil_usb` + USB VID:PID → `Detected`
3. Senão → `Simulated`

VID:PID canônico: `0xCAFE:0x4007`.

## Programador (U3)

`HilAgent::try_flash` com `Detected` e **sem** `mock_flash`:

| Condição | Resultado |
|----------|-----------|
| Sem feature `hil_programmer` | `FlashDenied::ProgrammerUnimplemented` |
| Feature, sem `BASE_HIL_ALLOW_FLASH` | `FlashDenied::AllowFlashRequired` |
| Allow sem `BASE_HIL_PROGRAMMER_CMD` | `FlashDenied::ProgrammerCmdMissing` |
| Allow + CMD ok | `FlashReceipt { mode: "experimental_external_cmd" }` |

`{image}` no CMD é substituído pelo path temporário do binário.

```bash
# CI / default — sem USB, sem programador
cargo test -p base-hil

# Programador EXPERIMENTAL (sem hardware; comando no-op)
cargo test -p base-hil --features hil_programmer

# Máquina local com probe + ferramenta
export BASE_HIL_ALLOW_FLASH=1
export BASE_HIL_PROGRAMMER_CMD='picotool load {image}'
cargo test -p base-hil --features hil_programmer,hil_usb
```

**Zero claim production:** `mode` nunca é `"production"`. Host ≠ silício; comando externo é responsabilidade do operador.

## Uso

```rust
use base_hil::{HilAgent, ProbePresence, DEFAULT_PROBE_PID, DEFAULT_PROBE_VID};

let a = HilAgent::connect(DEFAULT_PROBE_VID, DEFAULT_PROBE_PID)?; // Simulated
assert!(a.try_flash(&[0]).is_err());

let m = HilAgent::with_mock_flash(ProbePresence::Detected);
assert_eq!(m.try_flash(&[1])?.mode, "mock_dry_run");
```

← vault: [Sprint V3 CLI HIL](../base-vault/16%20-%20Path%20to%20v0.6/16.13%20-%20Sprint%20V3%20CLI%20HIL.md)
