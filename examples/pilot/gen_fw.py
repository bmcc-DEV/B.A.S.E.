#!/usr/bin/env python3
"""Generate a minimal AArch64 little-endian raw FW blob for the UART pilot.

Sequence (PC base 0):
  adrp x0, #0x40034000
  movz w1, #1
  str  w1, [x0]
  ldr  w2, [x0, #4]
  movz w1, #0
  str  w1, [x0, #0x18]
  movz w1, #0x41
  str  w1, [x0]
  ret

No external ARM toolchain required. Regenerate:
  python3 examples/pilot/gen_fw.py
  (cd examples/pilot && sha256sum fw.bin mmio.json … > SHA256SUMS)
"""
from __future__ import annotations

import pathlib
import struct


def u32(x: int) -> bytes:
    return struct.pack("<I", x & 0xFFFFFFFF)


def adrp(rd: int, imm: int) -> bytes:
    """ADRP Xd, #imm — imm must be page-aligned; PC assumed 0."""
    page = imm >> 12
    immlo = page & 0x3
    immhi = (page >> 2) & 0x7FFFF
    enc = (1 << 31) | (immlo << 29) | (0b10000 << 24) | (immhi << 5) | rd
    return u32(enc)


def movz_w(rd: int, imm16: int) -> bytes:
    """MOVZ Wd, #imm16"""
    enc = 0x52800000 | ((imm16 & 0xFFFF) << 5) | rd
    return u32(enc)


def str_w_imm(rt: int, rn: int, offset: int) -> bytes:
    """STR Wt, [Xn, #offset] — offset multiple of 4"""
    imm12 = offset // 4
    enc = 0xB9000000 | ((imm12 & 0xFFF) << 10) | (rn << 5) | rt
    return u32(enc)


def ldr_w_imm(rt: int, rn: int, offset: int) -> bytes:
    """LDR Wt, [Xn, #offset] — offset multiple of 4"""
    imm12 = offset // 4
    enc = 0xB9400000 | ((imm12 & 0xFFF) << 10) | (rn << 5) | rt
    return u32(enc)


def ret() -> bytes:
    return u32(0xD65F03C0)


FW = b"".join(
    [
        adrp(0, 0x40034000),
        movz_w(1, 1),
        str_w_imm(1, 0, 0),
        ldr_w_imm(2, 0, 4),
        movz_w(1, 0),
        str_w_imm(1, 0, 0x18),
        movz_w(1, 0x41),
        str_w_imm(1, 0, 0),
        ret(),
    ]
)

HERE = pathlib.Path(__file__).resolve().parent


def main() -> None:
    out = HERE / "fw.bin"
    out.write_bytes(FW)
    print(f"wrote {out} ({len(FW)} bytes)")
    print("hex:", FW.hex())
    print("arch=AArch64-LE format=raw-bin UART_page=0x40034000")


if __name__ == "__main__":
    main()
