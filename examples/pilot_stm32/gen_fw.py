#!/usr/bin/env python3
"""Generate a minimal AArch64-LE raw FW blob for the STM32 USART1 Capstone wedge (V1).

USART1 lives at 0x40013800 (APB2). Capstone ADRP uses the 4K page 0x40013000;
register accesses use offsets +0x800 (SR), +0x804 (DR), +0x80C (CR1).

Honest claim: synthetic AArch64 blob for Capstone — NOT Cortex-M3 Thumb silicon image.

Sequence (PC base 0):
  adrp x0, #0x40013000
  movz w1, #0x2004          ; CR1 enable-ish value from mmio.json
  str  w1, [x0, #0x80c]     ; CR1
  ldr  w2, [x0, #0x800]     ; SR
  movz w1, #0x41
  str  w1, [x0, #0x804]     ; DR
  ret

Regenerate:
  python3 examples/pilot_stm32/gen_fw.py
  (cd examples/pilot_stm32 && sha256sum fw.bin mmio.json contracts.yaml trace.csv pilot.bsl > SHA256SUMS)
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
    enc = 0x52800000 | ((imm16 & 0xFFFF) << 5) | rd
    return u32(enc)


def str_w_imm(rt: int, rn: int, offset: int) -> bytes:
    imm12 = offset // 4
    enc = 0xB9000000 | ((imm12 & 0xFFF) << 10) | (rn << 5) | rt
    return u32(enc)


def ldr_w_imm(rt: int, rn: int, offset: int) -> bytes:
    imm12 = offset // 4
    enc = 0xB9400000 | ((imm12 & 0xFFF) << 10) | (rn << 5) | rt
    return u32(enc)


def ret() -> bytes:
    return u32(0xD65F03C0)


# Page containing USART1 @ 0x40013800
PAGE = 0x40013000
OFF_SR = 0x800
OFF_DR = 0x804
OFF_CR1 = 0x80C

FW = b"".join(
    [
        adrp(0, PAGE),
        movz_w(1, 0x2004),
        str_w_imm(1, 0, OFF_CR1),
        ldr_w_imm(2, 0, OFF_SR),
        movz_w(1, 0x41),
        str_w_imm(1, 0, OFF_DR),
        ret(),
    ]
)

HERE = pathlib.Path(__file__).resolve().parent


def main() -> None:
    out = HERE / "fw.bin"
    out.write_bytes(FW)
    print(f"wrote {out} ({len(FW)} bytes)")
    print("hex:", FW.hex())
    print(
        "arch=AArch64-LE format=raw-bin "
        f"USART1_page=0x{PAGE:x} regs=0x40013800/04/0c (synthetic, not Thumb)"
    )


if __name__ == "__main__":
    main()
