//! Subset decoders: bytes → SIR ops, inverting the encoders in [`crate::encode`].
//!
//! Coverage is deliberately bounded: each decoder understands *exactly* the encodings
//! our encoders produce for the lifted SIR subset. A word outside that subset becomes
//! `Op::Unknown` (gap), mirroring the lifter's honesty. Full ISA decode is future work
//! (the catalog `encode_status`/decoder availability is the source of truth).

use crate::sir::{Op, VReg};
use crate::target::TargetIsa;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("no decoder for {0} yet (encode-only target)")]
    NoDecoder(String),
    #[error("unexpected end of stream decoding {0} @{1:#x}")]
    Truncated(&'static str, u64),
}

/// Decode `bytes` back into SIR ops for `isa`. Unsupported targets error; unknown
/// words become `Op::Unknown` gaps (never a silent mis-decode).
pub fn decode_ops(bytes: &[u8], isa: TargetIsa) -> Result<Vec<Op>, DecodeError> {
    match isa {
        TargetIsa::Mips => decode_mips(bytes),
        TargetIsa::Ppc => decode_ppc(bytes),
        TargetIsa::SuperH(_) => decode_superh(bytes),
        TargetIsa::Alpha => decode_alpha(bytes),
        TargetIsa::PaRisc => decode_parisc(bytes),
        TargetIsa::ColdFire => decode_coldfire(bytes),
        TargetIsa::Arm => decode_arm(bytes),
        TargetIsa::AArch64 => decode_aarch64(bytes),
        TargetIsa::Sparc => decode_sparc(bytes),
        TargetIsa::X86_64 => decode_x86(bytes),
        other => Err(DecodeError::NoDecoder(other.to_string())),
    }
}

/// True when `decode_ops` understands this ISA's encoded subset.
pub fn has_decoder(isa: TargetIsa) -> bool {
    matches!(
        isa,
        TargetIsa::Mips
            | TargetIsa::Ppc
            | TargetIsa::SuperH(_)
            | TargetIsa::Alpha
            | TargetIsa::PaRisc
            | TargetIsa::ColdFire
            | TargetIsa::Arm
            | TargetIsa::AArch64
            | TargetIsa::Sparc
            | TargetIsa::X86_64
    )
}

fn sext16(w: u32) -> i32 {
    (w as u16 as i16) as i32
}

fn decode_mips(bytes: &[u8]) -> Result<Vec<Op>, DecodeError> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    let v = |reg: u32| VReg(reg.saturating_sub(8)); // encoder maps VReg → $t8..
    while i + 4 <= bytes.len() {
        let w = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
        if w == 0x03E00008 {
            // jr $ra — encoder always appends the delay-slot nop.
            ops.push(Op::Ret);
            i += 4;
            if i + 4 <= bytes.len() {
                let d = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
                if d != 0 {
                    ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: d.to_be_bytes().to_vec(),
                        note: "jr $ra delay slot is not a nop".into(),
                    });
                }
                i += 4;
            }
            continue;
        }
        if w == 0 {
            ops.push(Op::Nop);
            i += 4;
            continue;
        }
        let opc = w >> 26;
        let rs = (w >> 21) & 0x1f;
        let rt = (w >> 16) & 0x1f;
        let imm = sext16(w & 0xffff);
        let funct = w & 0x3f;
        if opc == 0 && funct == 0x25 && rs == 0 && rt == 0 {
            // or $t, $zero, $zero
            ops.push(Op::Clear { dst: v((w >> 11) & 0x1f) });
            i += 4;
            continue;
        }
        if opc == 9 {
            // addiu
            if rs == 0 {
                ops.push(Op::MovImm {
                    dst: v(rt),
                    imm: imm as u32,
                });
            } else if rs == rt {
                ops.push(arith(v(rt), imm));
            } else {
                ops.push(gap(i, w, "mips addiu rs!=0, rs!=rt".into()));
            }
            i += 4;
            continue;
        }
        ops.push(gap(i, w, format!("mips opcode {opc:#x} outside encoder subset")));
        i += 4;
    }
    if i != bytes.len() {
        return Err(DecodeError::Truncated("mips", i as u64));
    }
    Ok(ops)
}

fn decode_ppc(bytes: &[u8]) -> Result<Vec<Op>, DecodeError> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        let w = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
        if w == 0x4E800020 {
            ops.push(Op::Ret); // blr
            i += 4;
            continue;
        }
        if w == 0x60000000 {
            ops.push(Op::Nop); // ori r0, r0, 0
            i += 4;
            continue;
        }
        if w >> 26 == 14 {
            // addi — encoder maps VReg → r3..
            let rt = (w >> 21) & 0x1f;
            let ra = (w >> 16) & 0x1f;
            let imm = sext16(w & 0xffff);
            let d = VReg(rt.saturating_sub(3));
            if ra == 0 {
                ops.push(Op::MovImm { dst: d, imm: imm as u32 });
            } else if ra == rt {
                ops.push(arith(d, imm));
            } else {
                ops.push(gap(i, w, "ppc addi ra!=0, ra!=rt".into()));
            }
            i += 4;
            continue;
        }
        ops.push(gap(i, w, format!("ppc opcode {:#x} outside encoder subset", w >> 26)));
        i += 4;
    }
    if i != bytes.len() {
        return Err(DecodeError::Truncated("ppc", i as u64));
    }
    Ok(ops)
}

fn decode_superh(bytes: &[u8]) -> Result<Vec<Op>, DecodeError> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    while i + 2 <= bytes.len() {
        let w = u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap());
        if w == 0x000B {
            // rts — encoder always appends the delay-slot nop.
            ops.push(Op::Ret);
            i += 2;
            if i + 2 <= bytes.len() {
                let d = u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap());
                if d != 0x0009 {
                    ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: d.to_le_bytes().to_vec(),
                        note: "rts delay slot is not a nop".into(),
                    });
                }
                i += 2;
            }
            continue;
        }
        if w == 0x0009 {
            ops.push(Op::Nop);
            i += 2;
            continue;
        }
        let n = ((w >> 8) & 0x0f) as u32;
        let imm = (w & 0xff) as u8 as i8 as i32;
        match w & 0xF000 {
            0xE000 => {
                // mov #imm, Rn
                ops.push(Op::MovImm { dst: VReg(n), imm: imm as u32 });
                i += 2;
                continue;
            }
            0x7000 => {
                // add #imm, Rn
                ops.push(arith(VReg(n), imm));
                i += 2;
                continue;
            }
            _ => {
                ops.push(Op::Unknown {
                    offset: i as u64,
                    bytes: w.to_le_bytes().to_vec(),
                    note: format!("sh halfword {w:#06x} outside encoder subset"),
                });
                i += 2;
            }
        }
    }
    if i != bytes.len() {
        return Err(DecodeError::Truncated("superh", i as u64));
    }
    Ok(ops)
}

/// `dst += imm` → Add/Sub/Inc/Dec, matching the encoders' imm canonicalization.
fn arith(dst: VReg, imm: i32) -> Op {
    match imm {
        1 => Op::Inc { dst },
        -1 => Op::Dec { dst },
        _ if imm < 0 => Op::SubImm { dst, imm: (-imm) as u32 },
        _ => Op::AddImm { dst, imm: imm as u32 },
    }
}

fn decode_alpha(bytes: &[u8]) -> Result<Vec<Op>, DecodeError> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        let w = u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
        if w == 0x6BFA8001 {
            ops.push(Op::Ret); // ret r31, (r26) — no delay slot on Alpha
            i += 4;
            continue;
        }
        if w == 0x23FF0000 {
            ops.push(Op::Nop); // lda r31, 0(r31)
            i += 4;
            continue;
        }
        if w >> 26 == 0x08 {
            // lda ra, disp(rb)
            let ra = (w >> 21) & 0x1f;
            let rb = (w >> 16) & 0x1f;
            let disp = sext16(w & 0xffff);
            let d = VReg(ra);
            if rb == 31 && ra != 31 {
                ops.push(Op::MovImm { dst: d, imm: disp as u32 });
            } else if rb == ra && ra != 31 {
                ops.push(arith(d, disp));
            } else {
                ops.push(gap(i, w, "alpha lda with rb outside {r31, ra}".into()));
            }
            i += 4;
            continue;
        }
        ops.push(gap(i, w, format!("alpha opcode {:#x} outside encoder subset", w >> 26)));
        i += 4;
    }
    if i != bytes.len() {
        return Err(DecodeError::Truncated("alpha", i as u64));
    }
    Ok(ops)
}

fn decode_parisc(bytes: &[u8]) -> Result<Vec<Op>, DecodeError> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        let w = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
        if w == 0xE840C000 || w == 0xE840C002 {
            // bv %r0(%rp) / bv,n — encoder appends the delay-slot nop.
            ops.push(Op::Ret);
            i += 4;
            if i + 4 <= bytes.len() {
                let d = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
                if d != 0x08000240 {
                    ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: d.to_be_bytes().to_vec(),
                        note: "bv delay slot is not a nop".into(),
                    });
                }
                i += 4;
            }
            continue;
        }
        if w == 0x08000240 {
            ops.push(Op::Nop); // or %r0, %r0, %r0
            i += 4;
            continue;
        }
        ops.push(gap(i, w, "parisc word outside encoder subset".into()));
        i += 4;
    }
    if i != bytes.len() {
        return Err(DecodeError::Truncated("parisc", i as u64));
    }
    Ok(ops)
}

fn decode_coldfire(bytes: &[u8]) -> Result<Vec<Op>, DecodeError> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    while i + 2 <= bytes.len() {
        let w = u16::from_be_bytes(bytes[i..i + 2].try_into().unwrap());
        if w == 0x4E71 {
            ops.push(Op::Nop);
            i += 2;
            continue;
        }
        if w == 0x4E75 {
            ops.push(Op::Ret); // rts
            i += 2;
            continue;
        }
        if (w & 0xF100) == 0x7000 {
            // moveq #imm, Dn
            let dn = ((w >> 9) & 7) as u32;
            let imm = (w & 0xff) as u8 as i8 as i32;
            ops.push(Op::MovImm { dst: VReg(dn), imm: imm as u32 });
            i += 2;
            continue;
        }
        if (w & 0xF83F) == 0x203C {
            // move.l #imm, Dn
            if i + 6 > bytes.len() {
                return Err(DecodeError::Truncated("coldfire", i as u64));
            }
            let dn = ((w >> 6) & 7) as u32;
            let imm = u32::from_be_bytes(bytes[i + 2..i + 6].try_into().unwrap());
            ops.push(Op::MovImm { dst: VReg(dn), imm });
            i += 6;
            continue;
        }
        if (w & 0xF83F) == 0x201F {
            // move.l (A7)+, Dn
            let dn = ((w >> 6) & 7) as u32;
            ops.push(Op::Pop { dst: VReg(dn) });
            i += 2;
            continue;
        }
        let d = |v: u16| VReg((v >> 3) as u32 & 7);
        let mut imm32 = || -> Result<u32, DecodeError> {
            if i + 6 > bytes.len() {
                Err(DecodeError::Truncated("coldfire", i as u64))
            } else {
                Ok(u32::from_be_bytes(bytes[i + 2..i + 6].try_into().unwrap()))
            }
        };
        match w & 0xFFC0 {
            0x0680 => {
                let imm = imm32()?;
                ops.push(Op::AddImm { dst: d(w), imm }); // addi.l #imm, Dn
                i += 6;
            }
            0x0480 => {
                let imm = imm32()?;
                ops.push(Op::SubImm { dst: d(w), imm }); // subi.l #imm, Dn
                i += 6;
            }
            0x4280 => {
                ops.push(Op::Clear { dst: d(w) }); // clr.l Dn
                i += 2;
            }
            0x5280 => {
                ops.push(Op::Inc { dst: d(w) }); // addq.l #1, Dn
                i += 2;
            }
            0x5380 => {
                ops.push(Op::Dec { dst: d(w) }); // subq.l #1, Dn
                i += 2;
            }
            0x29C0 => {
                ops.push(Op::Push { src: d(w) }); // move.l Dn, -(A7)
                i += 2;
            }
            _ => {
                ops.push(Op::Unknown {
                    offset: i as u64,
                    bytes: w.to_be_bytes().to_vec(),
                    note: format!("coldfire word {w:#06x} outside encoder subset"),
                });
                i += 2;
            }
        }
    }
    if i != bytes.len() {
        return Err(DecodeError::Truncated("coldfire", i as u64));
    }
    Ok(ops)
}

fn decode_arm(bytes: &[u8]) -> Result<Vec<Op>, DecodeError> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        let w = u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
        if w == 0xE320F000 {
            ops.push(Op::Nop); // MOV r0, r0 (alias NOP)
            i += 4;
            continue;
        }
        if w == 0xE12FFF1E {
            ops.push(Op::Ret); // BX LR
            i += 4;
            continue;
        }
        if (w & 0xFFFF_0F00) == 0xE3A0_0000 {
            // MOV Rd, #imm8 (cond=AL, opcode 1101, Rn=0000, rotate=0000) — encoder MovImm/Clear.
            let rd = (w >> 12) & 0xf;
            let imm = w & 0xff;
            ops.push(Op::MovImm { dst: VReg(rd), imm });
            i += 4;
            continue;
        }
        let is_sub = (w & 0xFFF0_0F00) == 0xE240_0000;
        if (w & 0xFFF0_0F00) == 0xE280_0000 || is_sub {
            // ADD/SUB Rd, Rd, #imm8 (cond=AL, opcode 0100/0010, S=0, Rn==Rd).
            let rd = (w >> 12) & 0xf;
            let rn = (w >> 16) & 0xf;
            let imm = w & 0xff;
            if rd != rn {
                ops.push(gap(i, w, "arm add/sub with rn != rd".into()));
            } else if is_sub {
                ops.push(arith(VReg(rd), -(imm as i32)));
            } else {
                ops.push(arith(VReg(rd), imm as i32));
            }
            i += 4;
            continue;
        }
        ops.push(gap(i, w, format!("arm word {w:#010x} outside encoder subset")));
        i += 4;
    }
    if i != bytes.len() {
        return Err(DecodeError::Truncated("arm", i as u64));
    }
    Ok(ops)
}

fn decode_aarch64(bytes: &[u8]) -> Result<Vec<Op>, DecodeError> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        let w = u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
        if w == 0xD503201F {
            ops.push(Op::Nop); // NOP
            i += 4;
            continue;
        }
        if w == 0xD65F03C0 {
            ops.push(Op::Ret); // RET
            i += 4;
            continue;
        }
        if (w & 0xFFFF_FFE0) == 0x2A1F_03E0 {
            // ORR Wd, WZR, WZR (MOV Wd, WZR) — encoder Clear.
            ops.push(Op::Clear { dst: VReg(w & 0x1f) });
            i += 4;
            continue;
        }
        if (w & 0xFFE0_001F) == 0x5280_0000 {
            // MOVZ Wd, #imm16 — encoder MovImm.
            let imm = (w >> 5) & 0xffff;
            ops.push(Op::MovImm { dst: VReg(w & 0x1f), imm });
            i += 4;
            continue;
        }
        let is_sub = (w & 0xFFC0_0000) == 0x5100_0000;
        if (w & 0xFFC0_0000) == 0x1100_0000 || is_sub {
            // ADD/SUB Wd, Wd, #imm12 — bits 31-22 fixed (sf=0 op=0/1 S=0 10001 shift=00),
            // imm12 in 21-10, rn in 9-5, rd in 4-0. Encoder emits rn == rd.
            let wd = w & 0x1f;
            let wn = (w >> 5) & 0x1f;
            let imm = (w >> 10) & 0xfff;
            if wd != wn {
                ops.push(gap(i, w, "aarch64 add/sub with rn != rd".into()));
            } else if is_sub {
                ops.push(arith(VReg(wd), -(imm as i32)));
            } else {
                ops.push(arith(VReg(wd), imm as i32));
            }
            i += 4;
            continue;
        }
        ops.push(gap(i, w, format!("aarch64 word {w:#010x} outside encoder subset")));
        i += 4;
    }
    if i != bytes.len() {
        return Err(DecodeError::Truncated("aarch64", i as u64));
    }
    Ok(ops)
}

fn decode_x86(bytes: &[u8]) -> Result<Vec<Op>, DecodeError> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    let need = |i: usize, n: usize, label: &'static str| -> Result<(), DecodeError> {
        if i + n > bytes.len() {
            Err(DecodeError::Truncated(label, i as u64))
        } else {
            Ok(())
        }
    };
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            0x90 => {
                ops.push(Op::Nop);
                i += 1;
            }
            0xC3 => {
                ops.push(Op::Ret);
                i += 1;
            }
            0xB8..=0xBF => {
                need(i, 5, "x86 mov")?;
                let imm = u32::from_le_bytes(bytes[i + 1..i + 5].try_into().unwrap());
                ops.push(Op::MovImm { dst: VReg((b & 7) as u32), imm });
                i += 5;
            }
            0x05 => {
                need(i, 5, "x86 add eax")?;
                let imm = u32::from_le_bytes(bytes[i + 1..i + 5].try_into().unwrap());
                ops.push(Op::AddImm { dst: VReg(0), imm });
                i += 5;
            }
            0x2D => {
                need(i, 5, "x86 sub eax")?;
                let imm = u32::from_le_bytes(bytes[i + 1..i + 5].try_into().unwrap());
                ops.push(Op::SubImm { dst: VReg(0), imm });
                i += 5;
            }
            0x83 => {
                // add/sub r/m32, imm8 — encoder emits mod=11 (reg) with reg field 0=add/5=sub.
                need(i, 3, "x86 83")?;
                let modrm = bytes[i + 1];
                let reg = (modrm >> 3) & 7;
                let rm = modrm & 7;
                if modrm >> 6 != 3 {
                    ops.push(gap(i, w32(bytes, i), "x86 83 with mod != 11".into()));
                } else {
                    let imm = bytes[i + 2] as i8 as i32;
                    match reg {
                        0 => ops.push(arith(VReg(rm as u32), imm)),
                        5 => ops.push(arith(VReg(rm as u32), -imm)),
                        _ => ops.push(gap(i, w32(bytes, i), format!("x86 83 reg={reg}"))),
                    }
                }
                i += 3;
            }
            0x31 => {
                // xor r/m32, reg — encoder emits reg==rm (Clear).
                need(i, 2, "x86 xor")?;
                let modrm = bytes[i + 1];
                let reg = (modrm >> 3) & 7;
                let rm = modrm & 7;
                if modrm >> 6 == 3 && reg == rm {
                    ops.push(Op::Clear { dst: VReg(rm as u32) });
                } else {
                    ops.push(gap(i, w32(bytes, i), "x86 xor not reg==reg".into()));
                }
                i += 2;
            }
            0x40..=0x47 => {
                ops.push(Op::Inc { dst: VReg((b & 7) as u32) });
                i += 1;
            }
            0x48..=0x4F => {
                ops.push(Op::Dec { dst: VReg((b & 7) as u32) });
                i += 1;
            }
            0x50..=0x57 => {
                ops.push(Op::Push { src: VReg((b & 7) as u32) });
                i += 1;
            }
            0x58..=0x5F => {
                ops.push(Op::Pop { dst: VReg((b & 7) as u32) });
                i += 1;
            }
            other => {
                ops.push(Op::Unknown {
                    offset: i as u64,
                    bytes: vec![other],
                    note: format!("x86 byte {other:#04x} outside encoder subset"),
                });
                i += 1;
            }
        }
    }
    Ok(ops)
}

fn w32(bytes: &[u8], i: usize) -> u32 {
    let mut w = [0u8; 4];
    let n = bytes.len().min(i + 4);
    w[..n - i].copy_from_slice(&bytes[i..n]);
    u32::from_le_bytes(w)
}

fn decode_sparc(bytes: &[u8]) -> Result<Vec<Op>, DecodeError> {
    let mut ops = Vec::new();
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        let w = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
        if w == 0x01000000 {
            ops.push(Op::Nop); // sethi %hi(0), %g0
            i += 4;
            continue;
        }
        if w == 0x81C3E008 {
            // retl = jmpl %o7+8, %g0 — encoder appends the delay-slot nop.
            ops.push(Op::Ret);
            i += 4;
            if i + 4 <= bytes.len() {
                let d = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
                if d != 0x01000000 {
                    ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: d.to_be_bytes().to_vec(),
                        note: "retl delay slot is not a nop".into(),
                    });
                }
                i += 4;
            }
            continue;
        }
        // or %g0, %g0, rd → Clear (i=0, asi=0, rs2=0). rd in bits 29-25 → %l0..%l7 (16..23).
        if (w & 0xC1F7_FFFF) == 0x8010_0000 {
            let rd = (w >> 25) & 0x1f;
            if (16..24).contains(&rd) {
                ops.push(Op::Clear { dst: VReg(rd - 16) });
            } else {
                ops.push(gap(i, w, "sparc clr rd outside %l0..%l7".into()));
            }
            i += 4;
            continue;
        }
        // or %g0, imm13, rd → MovImm (i=1, imm13 sign-extended).
        if (w & 0xC1F7_E000) == 0x8010_2000 {
            let rd = (w >> 25) & 0x1f;
            if !(16..24).contains(&rd) {
                ops.push(gap(i, w, "sparc mov rd outside %l0..%l7".into()));
                i += 4;
                continue;
            }
            let imm13 = w & 0x1fff;
            let imm = if imm13 & 0x1000 != 0 {
                (imm13 as i32 - 0x2000) as u32
            } else {
                imm13
            };
            ops.push(Op::MovImm { dst: VReg(rd - 16), imm });
            i += 4;
            continue;
        }
        ops.push(gap(i, w, format!("sparc word {w:#010x} outside encoder subset")));
        i += 4;
    }
    if i != bytes.len() {
        return Err(DecodeError::Truncated("sparc", i as u64));
    }
    Ok(ops)
}

fn gap(offset: usize, word: u32, note: String) -> Op {
    Op::Unknown {
        offset: offset as u64,
        bytes: word.to_be_bytes().to_vec(),
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::{encode_module, EncodeError};
    use crate::lift::lift_x86_32;
    use crate::target::{SuperHFlavor, TargetIsa};

    fn roundtrip(bytes: &[u8], isa: TargetIsa) -> Result<Vec<Op>, EncodeError> {
        let m = lift_x86_32(bytes, "f").unwrap();
        let enc = encode_module(&m, isa)?;
        Ok(decode_ops(&enc, isa).expect("decode"))
    }

    #[test]
    fn mips_decodes_what_it_encodes() {
        for (bytes, expect) in [
            (&[0x90, 0xC3][..], vec![Op::Nop, Op::Ret]),
            (
                &[0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3][..],
                vec![Op::MovImm { dst: VReg(0), imm: 1 }, Op::AddImm { dst: VReg(0), imm: 2 }, Op::Ret],
            ),
        ] {
            assert_eq!(roundtrip(bytes, TargetIsa::Mips).unwrap(), expect);
        }
    }

    #[test]
    fn ppc_decodes_what_it_encodes() {
        let ops = roundtrip(&[0x90, 0xC3], TargetIsa::Ppc).unwrap();
        assert_eq!(ops, vec![Op::Nop, Op::Ret]);
        let ops = roundtrip(&[0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3], TargetIsa::Ppc)
            .unwrap();
        assert_eq!(
            ops,
            vec![Op::MovImm { dst: VReg(0), imm: 1 }, Op::AddImm { dst: VReg(0), imm: 2 }, Op::Ret]
        );
    }

    #[test]
    fn superh_decodes_what_it_encodes() {
        let ops = roundtrip(&[0x90, 0xC3], TargetIsa::SuperH(SuperHFlavor::Sh2)).unwrap();
        assert_eq!(ops, vec![Op::Nop, Op::Ret]);
    }

    #[test]
    fn sh_add3_clear_normalizes_via_semantics() {
        // xor eax,eax (Clear) → mov #0,r0 on SH; decode gives MovImm{0,0}.
        let m = lift_x86_32(&[0x31, 0xC0, 0xC3], "z").unwrap();
        let enc = encode_module(&m, TargetIsa::SuperH(SuperHFlavor::Sh2)).unwrap();
        let ops = decode_ops(&enc, TargetIsa::SuperH(SuperHFlavor::Sh2)).unwrap();
        assert_eq!(ops[0], Op::MovImm { dst: VReg(0), imm: 0 });
    }

    #[test]
    fn alpha_decodes_what_it_encodes() {
        let ops = roundtrip(&[0x90, 0xC3], TargetIsa::Alpha).unwrap();
        assert_eq!(ops, vec![Op::Nop, Op::Ret]);
        let ops = roundtrip(
            &[0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3],
            TargetIsa::Alpha,
        )
        .unwrap();
        assert_eq!(
            ops,
            vec![Op::MovImm { dst: VReg(0), imm: 1 }, Op::AddImm { dst: VReg(0), imm: 2 }, Op::Ret]
        );
    }

    #[test]
    fn parisc_decodes_what_it_encodes() {
        let ops = roundtrip(&[0x90, 0xC3], TargetIsa::PaRisc).unwrap();
        // nop ; bv %r0(%rp) (+ folded delay nop)
        assert_eq!(ops, vec![Op::Nop, Op::Ret]);
    }

    #[test]
    fn coldfire_decodes_what_it_encodes() {
        let ops = roundtrip(&[0x90, 0xC3], TargetIsa::ColdFire).unwrap();
        assert_eq!(ops, vec![Op::Nop, Op::Ret]);
        let ops = roundtrip(
            &[0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3],
            TargetIsa::ColdFire,
        )
        .unwrap();
        assert_eq!(
            ops,
            vec![Op::MovImm { dst: VReg(0), imm: 1 }, Op::AddImm { dst: VReg(0), imm: 2 }, Op::Ret]
        );
    }

    #[test]
    fn coldfire_push_pop_roundtrip() {
        // push eax ; pop eax (x86 push/pop lift)
        let ops = roundtrip(&[0x50, 0x58, 0xC3], TargetIsa::ColdFire).unwrap();
        assert_eq!(ops, vec![Op::Push { src: VReg(0) }, Op::Pop { dst: VReg(0) }, Op::Ret]);
    }

    #[test]
    fn arm_decodes_what_it_encodes() {
        let ops = roundtrip(&[0x90, 0xC3], TargetIsa::Arm).unwrap();
        assert_eq!(ops, vec![Op::Nop, Op::Ret]);
        let ops = roundtrip(
            &[0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3],
            TargetIsa::Arm,
        )
        .unwrap();
        assert_eq!(
            ops,
            vec![Op::MovImm { dst: VReg(0), imm: 1 }, Op::AddImm { dst: VReg(0), imm: 2 }, Op::Ret]
        );
    }

    #[test]
    fn arm_clear_normalizes_via_semantics() {
        // xor eax,eax (Clear) → mov r0, #0 → decodes as MovImm{·,0}.
        let m = crate::lift::lift_x86_32(&[0x31, 0xC0, 0xC3], "z").unwrap();
        let enc = crate::encode::encode_module(&m, TargetIsa::Arm).unwrap();
        let ops = decode_ops(&enc, TargetIsa::Arm).unwrap();
        assert_eq!(ops[0], Op::MovImm { dst: VReg(0), imm: 0 });
    }

    #[test]
    fn arm_rotated_imm_is_a_gap_not_misdecode() {
        // MOV r0, #0x100 = 0xE3A00101 (rotate field 0x10) — outside encoder's imm8 subset.
        let w = 0xE3A0_0101u32;
        let ops = decode_ops(&w.to_le_bytes(), TargetIsa::Arm).unwrap();
        assert!(matches!(ops[0], Op::Unknown { .. }), "rotated form must be a gap: {ops:?}");
    }

    #[test]
    fn arm_adds_sets_flag_is_a_gap_not_misdecode() {
        // ADDS r0, r0, #1 (S=1) — encoder emits ADD with S=0; must not decode as AddImm.
        let w = 0xE290_0001u32;
        let ops = decode_ops(&w.to_le_bytes(), TargetIsa::Arm).unwrap();
        assert!(matches!(ops[0], Op::Unknown { .. }), "S=1 form must be a gap: {ops:?}");
    }

    #[test]
    fn sparc_decodes_what_it_encodes() {
        let ops = roundtrip(&[0x90, 0xC3], TargetIsa::Sparc).unwrap();
        assert_eq!(ops, vec![Op::Nop, Op::Ret]);
        // mov eax,1 ; xor eax,eax (Clear) — only mov/clear are encodable on sparc.
        let ops = roundtrip(&[0xB8, 0x01, 0x00, 0x00, 0x00, 0x31, 0xC0, 0xC3], TargetIsa::Sparc).unwrap();
        assert_eq!(
            ops,
            vec![Op::MovImm { dst: VReg(0), imm: 1 }, Op::Clear { dst: VReg(0) }, Op::Ret]
        );
        // add eax,2 is an encode gap, not a pass.
        assert!(encode_module(&lift_x86_32(&[0x83, 0xC0, 0x02], "g").unwrap(), TargetIsa::Sparc).is_err());
    }

    #[test]
    fn sparc_mov_clear_roundtrip() {
        // mov eax,1 ; xor eax,eax (Clear) — both encodable on sparc.
        let ops = roundtrip(&[0xB8, 0x01, 0x00, 0x00, 0x00, 0x31, 0xC0, 0xC3], TargetIsa::Sparc).unwrap();
        assert_eq!(
            ops,
            vec![Op::MovImm { dst: VReg(0), imm: 1 }, Op::Clear { dst: VReg(0) }, Op::Ret]
        );
    }

    #[test]
    fn sparc_imm13_sign_extends() {
        // or %g0, -1, %l0 → mov eax,-1 (i32 0xFFFFFFFF round-trips).
        let w = 0xA010_3FFFu32; // rd=16 (%l0), imm13 = 0x1FFF = -1
        let ops = decode_ops(&w.to_be_bytes(), TargetIsa::Sparc).unwrap();
        assert_eq!(ops[0], Op::MovImm { dst: VReg(0), imm: 0xFFFF_FFFF });
    }

    #[test]
    fn sparc_non_l_register_rd_is_a_gap() {
        // or %g0, %g0, %g0 (rd=0) — outside encoder's %l0..%l7 window.
        let w = 0x8010_0000u32;
        let ops = decode_ops(&w.to_be_bytes(), TargetIsa::Sparc).unwrap();
        assert!(matches!(ops[0], Op::Unknown { .. }), "{ops:?}");
    }

    #[test]
    fn x86_decodes_what_it_encodes() {
        let ops = roundtrip(&[0x90, 0xC3], TargetIsa::X86_64).unwrap();
        assert_eq!(ops, vec![Op::Nop, Op::Ret]);
        let ops = roundtrip(
            &[0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3],
            TargetIsa::X86_64,
        )
        .unwrap();
        assert_eq!(
            ops,
            vec![Op::MovImm { dst: VReg(0), imm: 1 }, Op::AddImm { dst: VReg(0), imm: 2 }, Op::Ret]
        );
    }

    #[test]
    fn x86_full_op_subset_roundtrips() {
        // xor eax,eax ; inc eax ; dec eax ; push eax ; pop eax ; sub eax,5
        let ops = roundtrip(&[0x31, 0xC0, 0x40, 0x48, 0x50, 0x58, 0x2D, 0x05, 0x00, 0x00, 0x00, 0xC3], TargetIsa::X86_64).unwrap();
        assert_eq!(
            ops,
            vec![
                Op::Clear { dst: VReg(0) },
                Op::Inc { dst: VReg(0) },
                Op::Dec { dst: VReg(0) },
                Op::Push { src: VReg(0) },
                Op::Pop { dst: VReg(0) },
                Op::SubImm { dst: VReg(0), imm: 5 },
                Op::Ret,
            ]
        );
    }

    #[test]
    fn x86_add_imm32_form_decodes() {
        // add eax, 0x7FFFFFFF → 0x05 form (dst==0, imm>0x7f).
        let ops = decode_ops(&[0x05, 0xFF, 0xFF, 0xFF, 0x7F], TargetIsa::X86_64).unwrap();
        assert_eq!(ops[0], Op::AddImm { dst: VReg(0), imm: 0x7FFF_FFFF });
    }

    #[test]
    fn x86_unknown_byte_is_a_gap() {
        // 0x66 is a prefix outside the encoder subset — gap, not misdecode.
        let ops = decode_ops(&[0x66, 0xC3], TargetIsa::X86_64).unwrap();
        assert!(matches!(ops[0], Op::Unknown { .. }));
        assert_eq!(ops[1], Op::Ret);
    }

    #[test]
    fn aarch64_decodes_what_it_encodes() {
        let ops = roundtrip(&[0x90, 0xC3], TargetIsa::AArch64).unwrap();
        assert_eq!(ops, vec![Op::Nop, Op::Ret]);
        let ops = roundtrip(
            &[0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3],
            TargetIsa::AArch64,
        )
        .unwrap();
        assert_eq!(
            ops,
            vec![Op::MovImm { dst: VReg(0), imm: 1 }, Op::AddImm { dst: VReg(0), imm: 2 }, Op::Ret]
        );
    }

    #[test]
    fn aarch64_clear_inc_dec_roundtrip() {
        // xor eax,eax (Clear) ; inc eax ; dec eax
        let ops = roundtrip(&[0x31, 0xC0, 0x40, 0x48, 0xC3], TargetIsa::AArch64).unwrap();
        assert_eq!(
            ops,
            vec![Op::Clear { dst: VReg(0) }, Op::Inc { dst: VReg(0) }, Op::Dec { dst: VReg(0) }, Op::Ret]
        );
    }

    #[test]
    fn aarch64_shifted_add_is_a_gap_not_misdecode() {
        // ADD W0, W0, #0x1234, lsl #12 — outside the encoder's shift=00 subset.
        let w = 0x1100_0000 | (0x1234 << 10) | (1u32 << 23) | 0;
        let ops = decode_ops(&w.to_le_bytes(), TargetIsa::AArch64).unwrap();
        assert!(matches!(ops[0], Op::Unknown { .. }), "shifted form must be a gap: {ops:?}");
    }

    #[test]
    fn no_decoder_for_encode_only_targets() {
        for t in [TargetIsa::M88k, TargetIsa::Ia64, TargetIsa::I860] {
            assert!(!has_decoder(t), "{t} should not have a decoder yet");
            assert!(matches!(decode_ops(&[], t), Err(DecodeError::NoDecoder(_))));
        }
        assert!(has_decoder(TargetIsa::Alpha));
        assert!(has_decoder(TargetIsa::PaRisc));
        assert!(has_decoder(TargetIsa::ColdFire));
    }
}
