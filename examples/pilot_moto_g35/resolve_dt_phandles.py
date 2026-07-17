#!/usr/bin/env python3
"""Resolve DT phandles (clocks / pinctrl) from an FDT blob — assist only.

≠ OS turnkey · ≠ verified rates · generates_os=false
"""
from __future__ import annotations

import argparse
import json
import re
import struct
import sys
from pathlib import Path


FDT_BEGIN_NODE, FDT_END_NODE, FDT_PROP, FDT_END = 1, 2, 3, 9


def extract_largest_fdt(data: bytes) -> bytes:
    magic = b"\xd0\x0d\xfe\xed"
    best = b""
    i = 0
    while True:
        j = data.find(magic, i)
        if j < 0:
            break
        if j + 8 <= len(data):
            sz = struct.unpack_from(">I", data, j + 4)[0]
            if 64 <= sz <= len(data) - j and sz < 16 * 1024 * 1024:
                blob = data[j : j + sz]
                if len(blob) > len(best):
                    best = blob
        i = j + 4
    if not best and data[:4] == magic:
        return data
    if not best:
        raise SystemExit("no FDT found")
    return best


def parse_tree(blob: bytes) -> dict:
    totalsize = struct.unpack_from(">I", blob, 4)[0]
    off_struct = struct.unpack_from(">I", blob, 8)[0]
    off_strings = struct.unpack_from(">I", blob, 12)[0]
    size_strings = struct.unpack_from(">I", blob, 32)[0]
    size_struct = struct.unpack_from(">I", blob, 36)[0]
    strings = blob[off_strings : off_strings + size_strings]
    struct_blob = blob[off_struct : off_struct + size_struct]

    def get_string(soff: int) -> str:
        end = strings.find(b"\x00", soff)
        return strings[soff:end].decode("ascii", "replace")

    pos = 0
    stack: list[str] = []
    nodes: dict[str, dict] = {}
    while pos < len(struct_blob):
        token = struct.unpack_from(">I", struct_blob, pos)[0]
        pos += 4
        if token == FDT_BEGIN_NODE:
            end = struct_blob.find(b"\x00", pos)
            name = struct_blob[pos:end].decode("ascii", "replace")
            pos = (end + 4) & ~3
            stack.append(name)
            nodes["/".join(stack)] = {"props": {}, "name": name}
        elif token == FDT_END_NODE:
            stack.pop()
        elif token == FDT_PROP:
            plen, nameoff = struct.unpack_from(">II", struct_blob, pos)
            pos += 8
            pname = get_string(nameoff)
            raw = struct_blob[pos : pos + plen]
            pos = (pos + plen + 3) & ~3
            path = "/".join(stack)
            nodes.setdefault(path, {"props": {}, "name": stack[-1] if stack else ""})
            nodes[path]["props"][pname] = raw
        elif token == FDT_END:
            break
        elif token == 4:  # NOP
            continue
        else:
            break
    return nodes


def u32s(raw: bytes) -> list[int]:
    return [struct.unpack_from(">I", raw, i)[0] for i in range(0, len(raw), 4)]


def strings_list(raw: bytes) -> list[str]:
    return [s.decode("ascii", "replace") for s in raw.split(b"\x00") if s]


def label_from_path(path: str) -> str:
    if "clock-controller@20010000" in path:
        return "ap_clk"
    if path.rstrip("/").endswith("apapb-gate") or path.endswith("/apapb-gate"):
        return "apapb_gate"
    if path.rstrip("/").endswith("ext-26m") or "/ext-26m" in path:
        return "ext_26m"
    leaf = path.rsplit("/", 1)[-1]
    leaf = re.sub(r"@.*", "", leaf)
    leaf = re.sub(r"[^A-Za-z0-9_]", "_", leaf)
    if not leaf or leaf[0].isdigit():
        leaf = "n_" + leaf
    return leaf


def build_phandle_index(nodes: dict) -> dict[int, str]:
    out = {}
    for path, n in nodes.items():
        props = n.get("props", {})
        for key in ("phandle", "linux,phandle"):
            if key in props and len(props[key]) >= 4:
                out[struct.unpack_from(">I", props[key], 0)[0]] = path
    return out


def clock_cells(nodes: dict, path: str) -> int:
    props = nodes.get(path, {}).get("props", {})
    if "#clock-cells" in props and len(props["#clock-cells"]) >= 4:
        return struct.unpack_from(">I", props["#clock-cells"], 0)[0]
    return 0


def decode_clocks(cells: list[int], ph2path: dict[int, str], nodes: dict) -> list[dict]:
    i = 0
    out = []
    while i < len(cells):
        ph = cells[i]
        path = ph2path.get(ph)
        if path is None:
            out.append(
                {
                    "error": f"cell {ph:#x} not a known phandle (stopped)",
                    "remaining": [f"{c:#x}" for c in cells[i:]],
                }
            )
            break
        ncells = clock_cells(nodes, path)
        args = cells[i + 1 : i + 1 + ncells]
        out.append(
            {
                "phandle": ph,
                "path": path,
                "label": label_from_path(path),
                "clock_cells": ncells,
                "args": args,
                "dts_ref": (
                    f"&{label_from_path(path)}"
                    + ("" if not args else " " + " ".join(str(a) for a in args))
                ),
            }
        )
        i += 1 + ncells
    return out


def find_uart_nodes(nodes: dict) -> list[str]:
    hits = []
    for path, n in nodes.items():
        props = n.get("props", {})
        low = path.lower()
        if "serial@" not in low and "uart" not in low:
            continue
        if "clocks" in props or "clock-names" in props:
            hits.append(path)
    return sorted(hits)


def resolve(dtb_path: Path) -> dict:
    data = dtb_path.read_bytes()
    blob = extract_largest_fdt(data)
    nodes = parse_tree(blob)
    ph2path = build_phandle_index(nodes)
    uart_out = []
    for path in find_uart_nodes(nodes):
        props = nodes[path]["props"]
        names = strings_list(props.get("clock-names", b""))
        cells = u32s(props.get("clocks", b""))
        decoded = decode_clocks(cells, ph2path, nodes) if cells else []
        clocks_line = None
        if decoded and all("error" not in d for d in decoded):
            refs = ", ".join(f"<{d['dts_ref']}>" for d in decoded)
            clocks_line = f"clocks = {refs};"
        uart_out.append(
            {
                "path": path,
                "label": label_from_path(path),
                "clock_names": names,
                "clocks_cells_hex": [f"{c:#x}" for c in cells],
                "resolved": decoded,
                "clocks_dts": clocks_line,
                "compatible": strings_list(props.get("compatible", b"")),
            }
        )
    return {
        "source": str(dtb_path),
        "phandle_count": len(ph2path),
        "uart_bindings": uart_out,
        "generates_os": False,
        "auto_flash_complete": False,
        "note": "phandle assist from vendor FDT — still wire labels into your board DTSI",
    }


def render_merged_dtsi(res: dict, uart0_base: str = "0x20200000") -> str:
    """Minimal merged fragment with resolved clocks for first UART if possible."""
    lines = [
        "/* SPDX-License-Identifier: GPL-2.0-only OR MIT */",
        "/* AUTO-GENERATED by resolve_dt_phandles.py — assist only */",
        "/* generates_os: false · auto_flash_complete: false */",
        "",
        "/ {",
        "    soc {",
        "        #address-cells = <2>;",
        "        #size-cells = <2>;",
        "        ranges;",
        "",
    ]
    # Emit provider stubs referenced by UART0
    uarts = res.get("uart_bindings") or []
    u0 = next((u for u in uarts if u["path"].endswith("serial@0")), uarts[0] if uarts else None)
    providers = []
    if u0:
        for d in u0.get("resolved") or []:
            if "error" in d:
                continue
            providers.append(d)
    seen = set()
    for d in providers:
        lab = d["label"]
        if lab in seen:
            continue
        seen.add(lab)
        path = d["path"]
        lines.append(f"        /* provider from {path} */")
        if "ext-26m" in path or d.get("clock_cells") == 0 and "fixed" in path:
            lines.append(f"        {lab}: {lab} {{")
            lines.append('            compatible = "fixed-clock";')
            lines.append("            #clock-cells = <0>;")
            lines.append("            /* clock-frequency: copy from vendor DT */")
            lines.append("        };")
        elif "gate" in path:
            lines.append(f"        {lab}: {lab} {{")
            lines.append('            compatible = "sprd,ums9620-apapb-gate";')
            lines.append("            #clock-cells = <1>;")
            lines.append("            /* reg/syscon: copy from vendor DT */")
            lines.append("        };")
        else:
            lines.append(f"        {lab}: clock-controller {{")
            lines.append('            compatible = "sprd,ums9620-ap-clk";')
            lines.append("            #clock-cells = <1>;")
            lines.append("            /* reg: prefer 0x20010000 from atlas */")
            lines.append("        };")
        lines.append("")

    if u0:
        names = u0.get("clock_names") or []
        lines.append(f"        /* UART0 — {u0['path']} */")
        base = uart0_base
        lines.append(f"        serial0: serial@{base[2:]} {{")
        lines.append(
            '            compatible = "sprd,ums9620-uart", "sprd,sc9836-uart";'
        )
        lines.append(f"            reg = <0x0 {base} 0x0 0x100>;")
        if names:
            q = ", ".join(f'"{n}"' for n in names)
            lines.append(f"            clock-names = {q};")
        if u0.get("clocks_dts"):
            lines.append(f"            {u0['clocks_dts']}")
        else:
            lines.append("            /* clocks: unresolved — see clocks_resolved.json */")
        lines.append("            status = \"okay\";")
        lines.append("        };")
        lines.append("")

    lines.append("    };")
    lines.append("};")
    lines.append("")
    return "\n".join(lines)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--dtb", required=True, type=Path)
    ap.add_argument("-o", "--output", required=True, type=Path, help="output directory")
    ap.add_argument("--uart0-base", default="0x20200000")
    args = ap.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    res = resolve(args.dtb)
    (args.output / "clocks_resolved.json").write_text(
        json.dumps(res, indent=2) + "\n", encoding="utf-8"
    )
    md = ["# Clocks resolvidos (phandle assist)\n", f"fonte: `{res['source']}`\n\n"]
    for u in res["uart_bindings"]:
        md.append(f"## `{u['path']}`\n\n")
        md.append(f"- clock-names: `{u['clock_names']}`\n")
        if u.get("clocks_dts"):
            md.append(f"- **dts:** `{u['clocks_dts']}`\n")
        for d in u.get("resolved") or []:
            if "error" in d:
                md.append(f"- error: {d}\n")
            else:
                md.append(
                    f"- `{d['dts_ref']}` ← `{d['path']}` (#clock-cells={d['clock_cells']})\n"
                )
        md.append("\n")
    md.append("≠ rates verificados · generates_os: false\n")
    (args.output / "CLOCKS_RESOLVED.md").write_text("".join(md), encoding="utf-8")
    dtsi = render_merged_dtsi(res, args.uart0_base)
    (args.output / "board-ums9620-wedge-merged.dtsi").write_text(dtsi, encoding="utf-8")
    ok = sum(1 for u in res["uart_bindings"] if u.get("clocks_dts"))
    print(
        f"resolve OK → {args.output} (uarts={len(res['uart_bindings'])} resolved_lines={ok})"
    )


if __name__ == "__main__":
    main()
