#!/usr/bin/env python3
"""Parse PS-X EXE (Sony PlayStation executable) — deterministic, no AI."""
from __future__ import annotations

import struct
from dataclasses import asdict, dataclass
from pathlib import Path


@dataclass
class PsxExe:
    path: str
    pc: int
    text_addr: int
    text_size: int
    sp: int
    gp: int
    header_bytes: int
    code: bytes
    code_sha256: str
    strings_sample: list[str]

    def to_dict(self) -> dict:
        d = asdict(self)
        d["code"] = None  # too large for JSON by default
        d["code_len"] = len(self.code)
        d["pc_hex"] = hex(self.pc)
        d["text_addr_hex"] = hex(self.text_addr)
        d["sp_hex"] = hex(self.sp)
        return d


def _sha256(b: bytes) -> str:
    import hashlib

    return hashlib.sha256(b).hexdigest()


def _strings(blob: bytes, min_len: int = 4, limit: int = 32) -> list[str]:
    out: list[str] = []
    cur = bytearray()
    for c in blob:
        if 32 <= c < 127:
            cur.append(c)
        else:
            if len(cur) >= min_len:
                out.append(cur.decode("ascii", errors="ignore"))
                if len(out) >= limit:
                    return out
            cur.clear()
    if len(cur) >= min_len and len(out) < limit:
        out.append(cur.decode("ascii", errors="ignore"))
    return out


def parse_psx_exe(path: Path) -> PsxExe:
    data = path.read_bytes()
    if len(data) < 0x800:
        raise ValueError(f"too small for PS-X EXE: {path}")
    if data[0:8] != b"PS-X EXE":
        raise ValueError(f"missing PS-X EXE magic: {path}")

    pc = struct.unpack_from("<I", data, 0x10)[0]
    gp = struct.unpack_from("<I", data, 0x14)[0]
    text_addr = struct.unpack_from("<I", data, 0x18)[0]
    text_size = struct.unpack_from("<I", data, 0x1C)[0]
    sp = struct.unpack_from("<I", data, 0x30)[0]  # some docs use 0x38; 0x30 is SP base in many EXEs
    # Prefer 0x38 if non-zero (initial SP)
    sp38 = struct.unpack_from("<I", data, 0x38)[0]
    if sp38:
        sp = sp38

    header = 0x800
    code = data[header : header + text_size] if text_size else data[header:]
    if text_size and len(code) < text_size:
        # truncated file — take what we have
        pass

    return PsxExe(
        path=str(path),
        pc=pc,
        text_addr=text_addr,
        text_size=text_size or len(code),
        sp=sp,
        gp=gp,
        header_bytes=header,
        code=code,
        code_sha256=_sha256(code),
        strings_sample=_strings(code),
    )


def write_minimal_fixture(path: Path) -> PsxExe:
    """Synthetic PS-X EXE for machine CI (nop; jr ra; nop)."""
    header = bytearray(0x800)
    header[0:8] = b"PS-X EXE"
    # PC = 0x80010000
    struct.pack_into("<I", header, 0x10, 0x80010000)
    struct.pack_into("<I", header, 0x14, 0)  # gp
    struct.pack_into("<I", header, 0x18, 0x80010000)  # text
    # MIPS: nop; jr $ra; nop  (3 words = 12 bytes) — pad to 16
    code = bytes.fromhex(
        "00000000"  # nop
        "03E00008"  # jr ra
        "00000000"  # nop (delay)
        "00000000"  # pad
    )
    struct.pack_into("<I", header, 0x1C, len(code))
    struct.pack_into("<I", header, 0x38, 0x801FFF00)  # SP
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(bytes(header) + code)
    return parse_psx_exe(path)
