#!/usr/bin/env python3
"""Merge B.A.S.E. port_package atlas with psx_to_srl map — deterministic, no AI.

Reads:
  - examples/pilot_saturn/out/port_package/address_driver_map.yaml (optional)
  - mapping/psx_to_srl.yaml

Writes:
  - out/generated/ATLAS_MERGE.md
  - out/generated/atlas_merge.json
"""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

try:
    import yaml
except ImportError as e:  # pragma: no cover
    raise SystemExit("PyYAML required") from e

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
DEFAULT_ATLAS = (
    REPO / "examples" / "pilot_saturn" / "out" / "port_package" / "address_driver_map.yaml"
)

# Heuristic: Saturn MMIO page → suggested SRL surface (table-driven, not ML)
PAGE_HINTS = [
    (0x25C00000, "gpu", ["SRL::VDP1", "SRL::Scene2D"], "VDP1 region"),
    (0x25F80000, "gpu", ["SRL::VDP2"], "VDP2 region"),
    (0x25F00000, "gpu", ["SRL::VDP1", "SRL::VDP2"], "VDP / GPU doorbell-ish"),
    (0x20100000, "input_sio0", ["SRL::Input::Management"], "SMPC region"),
]


def page_base(addr: int) -> int:
    return addr & ~0xFFF


def hint_for(addr: int) -> dict | None:
    pb = page_base(addr)
    for base, sub_id, srl, note in PAGE_HINTS:
        if page_base(base) == pb or abs(pb - page_base(base)) < 0x10000:
            return {"subsystem_id": sub_id, "srl": srl, "note": note, "matched_page": hex(base)}
    return None


def load_yaml_loose(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    clean = re.sub(r"![A-Za-z0-9_]+", "", text)
    return yaml.safe_load(clean) or {}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--map", type=Path, default=ROOT / "mapping" / "psx_to_srl.yaml")
    ap.add_argument("--atlas", type=Path, default=DEFAULT_ATLAS)
    ap.add_argument("-o", "--output", type=Path, default=ROOT / "out" / "generated")
    args = ap.parse_args()

    pmap = yaml.safe_load(args.map.read_text(encoding="utf-8"))
    sub_by_id = {s["id"]: s for s in (pmap.get("subsystems") or [])}

    out = args.output
    out.mkdir(parents=True, exist_ok=True)

    rows = []
    atlas_present = args.atlas.is_file()
    if atlas_present:
        atlas = load_yaml_loose(args.atlas)
        for e in atlas.get("entries") or []:
            addr = int(e.get("source_base") or 0)
            h = hint_for(addr)
            sub = sub_by_id.get((h or {}).get("subsystem_id", ""), {})
            rows.append(
                {
                    "block_id": e.get("block_id"),
                    "source_base": hex(addr),
                    "hal_id": e.get("hal_id"),
                    "strategy_atlas": e.get("strategy"),
                    "rewrite_needed": e.get("rewrite_needed"),
                    "hint": h,
                    "map_strategy": sub.get("strategy"),
                    "map_srl": sub.get("srl"),
                }
            )
    else:
        rows = []

    md = [
        "# ATLAS_MERGE — port_package × psx_to_srl",
        "",
        f"- atlas: `{'present' if atlas_present else 'MISSING — run examples/pilot_saturn/run.sh'}`",
        f"- map: `{args.map.relative_to(ROOT) if args.map.is_relative_to(ROOT) else args.map}`",
        "- honesty: merge ≠ auto-port · `runs_on_saturn=false`",
        "",
    ]
    if not atlas_present:
        md += [
            "_Sem atlas: só a matriz do mapa YAML está disponível via `emit_from_map.py`._",
            "",
        ]
    else:
        md += [
            "| Block | Source | Atlas strategy | SRL hint | Map subsystem |",
            "|-------|--------|----------------|----------|---------------|",
        ]
        for r in rows:
            h = r.get("hint") or {}
            md.append(
                "| `{bid}` | `{src}` | `{st}` | {srl} | `{sub}` |".format(
                    bid=r.get("block_id"),
                    src=r.get("source_base"),
                    st=r.get("strategy_atlas"),
                    srl=", ".join(f"`{x}`" for x in (h.get("srl") or [])) or "—",
                    sub=h.get("subsystem_id") or "—",
                )
            )
        md.append("")

    md += [
        "## Próximos passos determinísticos",
        "",
        "1. `python3 tools/emit_from_map.py --scaffold`",
        "2. `python3 tools/verify_srl_apis.py`",
        "3. Preencher `PORT_CHECKLIST.md` (humano)",
        "4. Compilar scaffold no toolchain SRL (makefile template)",
        "",
    ]
    (out / "ATLAS_MERGE.md").write_text("\n".join(md), encoding="utf-8")
    (out / "atlas_merge.json").write_text(
        json.dumps(
            {
                "atlas_present": atlas_present,
                "atlas_path": str(args.atlas),
                "rows": rows,
                "honesty": {"runs_on_saturn": False, "auto_ports_games": False},
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"merge → {out / 'ATLAS_MERGE.md'} ({len(rows)} rows)")


if __name__ == "__main__":
    main()
