#!/usr/bin/env python3
"""Deterministic emit from mapping/psx_to_srl.yaml — no AI.

Emits:
  - psx_pad_map.hpp       (button wrap from YAML)
  - srl_includes.hpp      (#include hints from SRL::* list)
  - PORT_CHECKLIST.md     (subsystem strategies)
  - subsystem_matrix.json (machine-readable work queue)
  - scaffold/main.cxx     (optional --scaffold)
"""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

try:
    import yaml
except ImportError as e:  # pragma: no cover
    raise SystemExit("PyYAML required: pip install pyyaml") from e

ROOT = Path(__file__).resolve().parents[1]
MAP_PATH = ROOT / "mapping" / "psx_to_srl.yaml"

# Saturn Digital::Button tokens allowed in generated code
SRL_DIGITAL_BUTTONS = {
    "A", "B", "C", "X", "Y", "Z", "L", "R", "START",
    "Up", "Down", "Left", "Right",
}


def load_map(path: Path) -> dict:
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise SystemExit("map root must be a mapping")
    if data.get("schema") != "psx_to_srl/v1":
        raise SystemExit(f"unsupported schema: {data.get('schema')}")
    return data


def emit_pad_hpp(data: dict) -> str:
    input_sub = next((s for s in data["subsystems"] if s["id"] == "input_sio0"), None)
    if not input_sub:
        raise SystemExit("subsystem input_sio0 missing")
    bm = input_sub.get("button_map") or {}
    lines = [
        "// AUTO-GENERATED from mapping/psx_to_srl.yaml — do not edit by hand",
        "// Regenerar: python3 tools/emit_from_map.py",
        "#pragma once",
        "#include <srl.hpp>",
        "",
        "namespace PsxSaturnBridge {",
        "",
        "struct PadMap {",
        "    using Digital = SRL::Input::Digital;",
        "",
    ]
    for psx, saturn in bm.items():
        if psx == "DPad":
            continue
        name = re.sub(r"[^A-Za-z0-9_]", "", str(psx))
        if saturn is None or str(saturn).lower() == "null":
            lines.append(f"    /** PS1 {psx}: sem botão Saturn — tratar em UI/remap. */")
            lines.append(
                f"    static bool {name}(const Digital&) {{ return false; }}"
            )
            lines.append("")
            continue
        btn = str(saturn).strip()
        if btn not in SRL_DIGITAL_BUTTONS:
            raise SystemExit(f"button_map {psx}→{btn}: not in {sorted(SRL_DIGITAL_BUTTONS)}")
        lines.append(
            f"    static bool {name}(const Digital& p) {{ "
            f"return p.IsHeld(Digital::Button::{btn}); }}"
        )
    # D-Pad
    dpad = bm.get("DPad") or ["Up", "Down", "Left", "Right"]
    for d in dpad:
        d = str(d)
        if d not in SRL_DIGITAL_BUTTONS:
            raise SystemExit(f"DPad entry {d} invalid")
        lines.append(
            f"    static bool {d}(const Digital& p) {{ "
            f"return p.IsHeld(Digital::Button::{d}); }}"
        )
    lines += ["};", "", "} // namespace PsxSaturnBridge", ""]
    return "\n".join(lines)


def collect_srl_symbols(data: dict) -> list[str]:
    seen: list[str] = []
    for sub in data.get("subsystems") or []:
        for s in sub.get("srl") or []:
            s = str(s).strip()
            if s and s not in seen:
                seen.append(s)
    return seen


def emit_includes_hpp(symbols: list[str]) -> str:
    lines = [
        "// AUTO-GENERATED — umbrella include for SRL port scaffold",
        "#pragma once",
        "#include <srl.hpp>",
        "",
        "// Referenced APIs (from psx_to_srl.yaml):",
    ]
    for s in symbols:
        lines.append(f"//   - {s}")
    lines.append("")
    return "\n".join(lines)


def emit_checklist(data: dict) -> str:
    honesty = data.get("honesty") or {}
    lines = [
        "# PORT_CHECKLIST — gerado de psx_to_srl.yaml",
        "",
        f"- `runs_on_saturn`: **{honesty.get('runs_on_saturn')}**",
        f"- `auto_ports_games`: **{honesty.get('auto_ports_games')}**",
        "",
        "Regenerar: `python3 tools/emit_from_map.py`",
        "",
        "| # | Subsystem | Strategy | SRL | Done |",
        "|---|-----------|----------|-----|------|",
    ]
    for i, sub in enumerate(data.get("subsystems") or [], 1):
        srl = ", ".join(f"`{x}`" for x in (sub.get("srl") or []))
        lines.append(
            f"| {i} | `{sub['id']}` | `{sub.get('strategy')}` | {srl} | [ ] |"
        )
    lines += [
        "",
        "## Notas por subsystem",
        "",
    ]
    for sub in data.get("subsystems") or []:
        note = (sub.get("notes") or "").strip()
        lines.append(f"### `{sub['id']}`")
        lines.append("")
        lines.append(note or "_(sem nota)_")
        lines.append("")
    lines += [
        "## Pipeline B.A.S.E.",
        "",
    ]
    for k, v in (data.get("base_pipeline") or {}).items():
        lines.append(f"- **{k}**: `{v}`")
    lines.append("")
    return "\n".join(lines)


def emit_matrix(data: dict) -> dict:
    return {
        "schema": data.get("schema"),
        "honesty": data.get("honesty"),
        "work_queue": [
            {
                "id": s["id"],
                "strategy": s.get("strategy"),
                "rewrite_needed": s.get("strategy")
                in ("rewrite", "rewrite_io", "replace", "asset_pipeline", "manual"),
                "srl": s.get("srl") or [],
                "psx": s.get("psx") or [],
            }
            for s in (data.get("subsystems") or [])
        ],
    }


def emit_scaffold_main() -> str:
    return """\
// AUTO-GENERATED scaffold — PsxSaturnBridge::PadMap from YAML
#include <srl.hpp>
#include "psx_pad_map.hpp"
#include "srl_includes.hpp"

using namespace SRL::Types;
using namespace PsxSaturnBridge;

int main()
{
    SRL::Core::Initialize(HighColor::Colors::Black);
    SRL::Debug::Print(1, 1, "psx-saturn-bridge (generated)");

    SRL::Input::Digital pad(0);
    while (1)
    {
        if (pad.IsConnected() && PadMap::Start(pad))
            SRL::Debug::Print(1, 3, "Start");
        SRL::Core::Synchronize();
    }
    return 0;
}
"""


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("-o", "--output", type=Path, default=ROOT / "out" / "generated")
    ap.add_argument("--map", type=Path, default=MAP_PATH)
    ap.add_argument("--scaffold", action="store_true", help="also write scaffold/main.cxx")
    args = ap.parse_args()

    data = load_map(args.map)
    out: Path = args.output
    out.mkdir(parents=True, exist_ok=True)

    symbols = collect_srl_symbols(data)
    (out / "psx_pad_map.hpp").write_text(emit_pad_hpp(data), encoding="utf-8")
    (out / "srl_includes.hpp").write_text(emit_includes_hpp(symbols), encoding="utf-8")
    (out / "PORT_CHECKLIST.md").write_text(emit_checklist(data), encoding="utf-8")
    (out / "subsystem_matrix.json").write_text(
        json.dumps(emit_matrix(data), indent=2) + "\n", encoding="utf-8"
    )

    if args.scaffold:
        sc = out / "scaffold"
        sc.mkdir(exist_ok=True)
        (sc / "main.cxx").write_text(emit_scaffold_main(), encoding="utf-8")
        # copy pad + includes next to main for local builds
        (sc / "psx_pad_map.hpp").write_text((out / "psx_pad_map.hpp").read_text(encoding="utf-8"), encoding="utf-8")
        (sc / "srl_includes.hpp").write_text((out / "srl_includes.hpp").read_text(encoding="utf-8"), encoding="utf-8")

    print(f"emitted → {out}")
    for p in sorted(out.rglob("*")):
        if p.is_file():
            print(f"  {p.relative_to(out)}")


if __name__ == "__main__":
    main()
