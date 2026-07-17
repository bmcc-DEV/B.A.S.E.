# Handoff — Wedge P0 G35 → **postmarketOS**

> Assist B.A.S.E. empacota; o OS é **postmarketOS** noutro tree (`pmbootstrap`).
> `generates_os: false` · ≠ Android userspace · ≠ earlycon verificado no silício.

## One-shot

```bash
./examples/pilot_moto_g35/run_path_a.sh
```

→ `out_real/handoff_external/`  
Guia pmOS: [`POSTMARKETOS.md`](POSTMARKETOS.md)

## Bases (ums9620 / manila)

| | |
|--|--|
| UART0 | `0x20200000` |
| GICD / GICR | `0x12000000` / `0x12040000` |
| UFS | `0x22000000` |
| AP clk / pinctrl | `0x20010000` / `0x642e0000` |

## Após flash manual

```bash
./examples/pilot_moto_g35/out_real/handoff_external/lab/lab_watch_assist.sh
```

## O B.A.S.E. não faz

- Compilar pmOS / kernel Unisoc
- Flash automático
- Modem sem blobs

Vault 24.x · tag `v1.6.3-rc`
