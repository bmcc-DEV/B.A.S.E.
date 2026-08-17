#!/usr/bin/env python3
"""Verify SRL::* symbols from the map exist in SaturnRingLib headers — no AI."""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError as e:  # pragma: no cover
    raise SystemExit("PyYAML required") from e

ROOT = Path(__file__).resolve().parents[1]


def resolve_srl_root(map_data: dict, override: Path | None) -> Path:
    if override:
        return override.resolve()
    link = ROOT / "SaturnRingLib"
    if link.is_dir():
        return link.resolve()
    default = Path("/media/bruno/Bruno/SaturnRingLib")
    if default.is_dir():
        return default
    raise SystemExit("SaturnRingLib not found — run scripts/setup.sh")


def leaf_token(srl_path: str) -> str:
    """SRL::Input::Digital → Digital; SRL::VDP1 → VDP1"""
    parts = [p for p in srl_path.split("::") if p]
    return parts[-1] if parts else srl_path


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--map", type=Path, default=ROOT / "mapping" / "psx_to_srl.yaml")
    ap.add_argument("--srl-root", type=Path, default=None)
    args = ap.parse_args()

    data = yaml.safe_load(args.map.read_text(encoding="utf-8"))
    srl_root = resolve_srl_root(data, args.srl_root)
    headers = list((srl_root / "saturnringlib").glob("srl*.hpp"))
    if not headers:
        print(f"FAIL: no srl*.hpp under {srl_root}", file=sys.stderr)
        return 1

    blob = "\n".join(h.read_text(encoding="utf-8", errors="replace") for h in headers)

    symbols: list[str] = []
    for sub in data.get("subsystems") or []:
        for s in sub.get("srl") or []:
            if s not in symbols:
                symbols.append(str(s))

    missing = []
    for s in symbols:
        tok = leaf_token(s)
        # class/struct/namespace/enum name as token
        if not re.search(rf"\b{re.escape(tok)}\b", blob):
            missing.append(s)

    print(f"SRL root: {srl_root}")
    print(f"headers:  {len(headers)}")
    print(f"symbols:  {len(symbols)}")
    if missing:
        print("MISSING:")
        for m in missing:
            print(f"  - {m}")
        return 1
    print("OK — all mapped SRL leaf tokens found in headers")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
