# B.A.S.E. Platform — Automated RE (HW / SW)

B.A.S.E. is an **evidence-assisted automated reverse-engineering platform**, split into Hardware-facing perception and Software reasoning.

## Honesty

- `generates_os: false`
- `auto_fix_complete: false`
- Flash = lab assist / manual — receipts never `production`
- No Transformers / ONNX in the reasoning path
- ≠ “magic RE of any binary”

## Division

| Side | Role | Crates |
|------|------|--------|
| **Hardware-facing** | Acquire immutable evidence | `specterprobe`, `base-virt` (QMP/Live), `base-port` (wedge/USB×DT), `base-hil`, `base-core` evidence |
| **Software reasoning** | Questions → beliefs → hypotheses → triad | `base-reason` |

Loop: **observe → ask → hypothesize → lab/receipt → strengthen/forget**.

## CLI

```bash
./target/debug/base reason g35 -o output/reason
./target/debug/base reason report --wedge path/to/wedge_mmio_map.yaml --format json
```

## Phase 2 (documented only)

- RPU-lite INT8 ops on belief graph if API needs it
- Relativistic causal zones (Twin vs Live vs lab clocks)
- HDL experiment — **≠ fabricable** until explicit gate

## Out of scope here

Dreamcast/Minecraft, GeoVex/vHGPU, OS turnkey, production flash.
