# G35 · Reasoning slice (HW → SW)

> Path A handoff is **Hardware-facing evidence**.  
> `base reason` turns it into **open questions + triad** — ≠ OS boot claim · ≠ flash.

## Inputs

- Atlas: `out_real/handoff_external/atlas/wedge_mmio_map.yaml`
- Optional: `--twin-miss`, `--evidence-id`

## Commands

```bash
# From repo root (after cargo build -p base-cli)
./target/debug/base reason g35 -o output/reason_g35

# Explicit wedge + receipt draft
./target/debug/base reason report \
  --wedge examples/pilot_moto_g35/out_real/handoff_external/atlas/wedge_mmio_map.yaml \
  --receipt-draft \
  --format markdown \
  -o output/reason_g35
```

## Outputs

| File | Meaning |
|------|---------|
| `reason_report.md` / `.json` | QRM questions, hypotheses %, triad verdict |
| `reason_receipt_draft.json` | Lab receipt **draft** (`flashed: false`, mode `lab_assist`) |

## Triad

- **Truth** — needs `--evidence-id` (or empty → Block closing claims)
- **Coherence** — fails with `--incoherent`
- **Causality** — assist default OK without CausalEdge

Honesty: `generates_os: false` · `auto_fix_complete: false`.

See also: [POSTMARKETOS.md](POSTMARKETOS.md), [WEDGE_HANDOFF.md](WEDGE_HANDOFF.md), [docs/PLATFORM_RE.md](../../docs/PLATFORM_RE.md).
