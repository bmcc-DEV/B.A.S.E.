# Moto G35 real Firmware.zip — CASE SUMMARY

> Product: **ums9620 / QogirN6Pro** (Unisoc) · Android 14 stock (`UOA34.216-174-1`) · ≠ TaurOS complete

| Image | Blocks | Ψ | Confidence | Port package |
|-------|--------|---|------------|--------------|
| `analyze_lk` | 45 | 0.164 | 85.9% `conclusive_match` | wrap=35 rewrite=10 fossils=184 |
| `analyze_boot` | 53 | 0.303 | 76.8% `inconclusive` | wrap=53 rewrite=0 fossils=22 |
| `analyze_kernel` | 415 | 0.192 | 83.9% `inconclusive` | wrap=415 rewrite=0 fossils=12 |

## Primary atlas (usar primeiro)

`examples/pilot_moto_g35/out_real/port_package_lk/`

- Capstone MMIO real no **lk-sign.bin** (Little Kernel)
- Ψ **ConclusiveMatch** (~86%)
- 35 wrap / 10 rewrite / 184 fossils (Unknown purpose/blocks)
- Ficheiros: `PORT_PACKAGE.md`, `address_driver_map.yaml`, `fossil_inventory.yaml`, `hal_mmio_stub.c`

## Boot notes

- `boot-gki.img` hit Unisoc-range pages (ex. `0xa9073000`) — útil cruzar com mapa LK
- `EXEC_KERNEL_IMAGE.bin` = heuristic-heavy (415 Gpu labels) — secundário

## Reproduzir

```bash
./examples/pilot_moto_g35/run_real_fw.sh
```

## Honesty

- `generates_os: false` · `Firmware.zip` / `real_fw/` gitignored
- status: **OK**
