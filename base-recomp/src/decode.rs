//! Subset decoders: bytes → SIR ops, inverting the encoders in [`crate::encode`].
//!
//! Coverage is deliberately bounded: each decoder understands *exactly* the encodings
//! our encoders produce for the lifted SIR subset. A word outside that subset becomes
//! `Op::Unknown` (gap), mirroring the lifter's honesty. Full ISA decode is future work
//! (the catalog `encode_status`/decoder availability is the source of truth).

use crate::sir::{Op, VReg, Cond};
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
        // Push/Pop idiom folds: addiu $sp,-4 ; sw rt,0($sp)  /  lw rt,0($sp) ; addiu $sp,+4.
        if w == 0x27BDFFFC && i + 8 <= bytes.len() {
            let w2 = u32::from_be_bytes(bytes[i + 4..i + 8].try_into().unwrap());
            if (w2 >> 26) == 0x2B && ((w2 >> 21) & 0x1f) == 29 && (w2 & 0xffff) == 0 {
                ops.push(Op::Push { src: v((w2 >> 16) & 0x1f) });
                i += 8;
                continue;
            }
        }
        if (w >> 26) == 0x23 && ((w >> 21) & 0x1f) == 29 && (w & 0xffff) == 0 && i + 8 <= bytes.len() {
            let w2 = u32::from_be_bytes(bytes[i + 4..i + 8].try_into().unwrap());
            if w2 == 0x27BD0004 {
                ops.push(Op::Pop { dst: v((w >> 16) & 0x1f) });
                i += 8;
                continue;
            }
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
        if opc == 0x23 {
            // lw $rt, imm($rs) — LdMem (width 4, encoder subset). $sp (29) base is the
            // pop idiom (folded above); a stray sp-based load is outside the subset.
            if rs == 29 {
                ops.push(gap(i, w, "mips lw with $sp base outside push/pop idiom".into()));
            } else {
                ops.push(Op::LdMem {
                    dst: v(rt),
                    base: v(rs),
                    offset: imm,
                    width: 4,
                });
            }
            i += 4;
            continue;
        }
        if opc == 0x2B {
            // sw $rt, imm($rs) — StMem (width 4).
            if rs == 29 {
                ops.push(gap(i, w, "mips sw with $sp base outside push/pop idiom".into()));
            } else {
                ops.push(Op::StMem {
                    src: v(rt),
                    base: v(rs),
                    offset: imm,
                    width: 4,
                });
            }
            i += 4;
            continue;
        }
        if opc == 0x03 {
            // jal — the encoder appends the delay-slot nop (fold it).
            ops.push(Op::CallRel {
                rel: ((w & 0x03FF_FFFF) as i32) << 2,
                target: None,
                symbol: None,
            });
            i += 4;
            if i + 4 <= bytes.len() {
                let d = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
                if d != 0 {
                    ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: d.to_be_bytes().to_vec(),
                        note: "jal delay slot is not a nop".into(),
                    });
                }
                i += 4;
            }
            continue;
        }
        if opc == 0x04 && rs == 0 && rt == 0 {
            // beq $zero,$zero (the unconditional PC-relative jump) + delay nop.
            ops.push(Op::JmpRel {
                rel: (imm << 2) as i32,
                target: None,
                symbol: None,
            });
            i += 4;
            if i + 4 <= bytes.len() {
                let d = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
                if d != 0 {
                    ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: d.to_be_bytes().to_vec(),
                        note: "branch delay slot is not a nop".into(),
                    });
                }
                i += 4;
            }
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
        if w == 0x7FE00008 {
            ops.push(Op::Trap); // trap (tw 31,0,0)
            i += 4;
            continue;
        }
        // stwu rS,-4(r1) → Push; lwzu rS,4(r1) → Pop (single-instruction stack ops).
        if w >> 26 == 37 && ((w >> 16) & 0x1f) == 1 && sext16(w & 0xffff) == -4 {
            ops.push(Op::Push { src: VReg(((w >> 21) & 0x1f).saturating_sub(3)) });
            i += 4;
            continue;
        }
        if w >> 26 == 33 && ((w >> 16) & 0x1f) == 1 && sext16(w & 0xffff) == 4 {
            ops.push(Op::Pop { dst: VReg(((w >> 21) & 0x1f).saturating_sub(3)) });
            i += 4;
            continue;
        }
        if w >> 26 == 18 {
            // b/bl — LI (24-bit signed, scaled by 4) relative to the branch's own PC.
            if w & 2 != 0 {
                ops.push(gap(i, w, "ppc absolute branch (AA=1) outside encoder subset".into()));
            } else {
                let li = ((w & 0x03FF_FFFC) << 8) as i32 >> 8; // sign-extend the 24-bit LI field
                if w & 1 != 0 {
                    ops.push(Op::CallRel { rel: li, target: None, symbol: None });
                } else {
                    ops.push(Op::JmpRel { rel: li, target: None, symbol: None });
                }
            }
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
        if w >> 26 == 32 {
            // lwz rT, d(rA) — LdMem (width 4).
            let rt = (w >> 21) & 0x1f;
            let ra = (w >> 16) & 0x1f;
            let d = VReg(rt.saturating_sub(3));
            let base = VReg(ra.saturating_sub(3));
            ops.push(Op::LdMem { dst: d, base, offset: sext16(w & 0xffff), width: 4 });
            i += 4;
            continue;
        }
        if w >> 26 == 36 {
            // stw rS, d(rA) — StMem (width 4).
            let rs = (w >> 21) & 0x1f;
            let ra = (w >> 16) & 0x1f;
            let src = VReg(rs.saturating_sub(3));
            let base = VReg(ra.saturating_sub(3));
            ops.push(Op::StMem { src, base, offset: sext16(w & 0xffff), width: 4 });
            i += 4;
            continue;
        }
        // CMPW: 0x7C000000 | (RA << 16) | (RB << 11) | 0x0000
        if (w & 0xFC00FFFF) == 0x7C000000 {
            let ra = (w >> 16) & 0x1F;
            let rb = (w >> 11) & 0x1F;
            if ra >= 3 && rb >= 3 {
                ops.push(Op::Cmp { rd: VReg(ra - 3), rs: VReg(rb - 3) });
                i += 4;
                continue;
            }
        }
        // CMPLW: 0x7C000000 | (RA << 16) | (RB << 11) | 0x0020
        if (w & 0xFC00FFFF) == 0x7C000020 {
            let ra = (w >> 16) & 0x1F;
            let rb = (w >> 11) & 0x1F;
            if ra >= 3 && rb >= 3 {
                ops.push(Op::Test { rd: VReg(ra - 3), rs: VReg(rb - 3) });
                i += 4;
                continue;
            }
        }
        // BC: 0x40000000 | (BO << 21) | (BI << 16) | BD
        if (w >> 26) == 16 {
            let bo = (w >> 21) & 0x1F;
            let bi = (w >> 16) & 0x1F;
            let bd = sext16(w & 0xFFFF);
            let cond = match (bo, bi) {
                (12, 2) => Cond::Eq,
                (4, 2) => Cond::Ne,
                (12, 0) => Cond::Lt,
                (4, 0) => Cond::Ge,
                (12, 1) => Cond::Gt,
                (4, 1) => Cond::Le,
                (12, 3) => Cond::Vs,
                (4, 3) => Cond::Vc,
                _ => {
                    ops.push(gap(i, w, format!("ppc unknown bc bo={bo} bi={bi}")));
                    i += 4;
                    continue;
                }
            };
            ops.push(Op::BranchCond { cond, target: (bd as i32) as u64 });
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
        // Memory group: mov.l @r15+,Rn → Pop; mov.l @Rm,Rn → LdMem (offset 0).
        if (w & 0xF000) == 0x5000 {
            let n = ((w >> 8) & 0x0f) as u32;
            let m = ((w >> 4) & 0x0f) as u32;
            let lo = w & 0x000f;
            if m == 15 && lo == 0x6 {
                // mov.l @r15+, Rn (post-increment load — SH-4 manual; binutils SH
                // prints the 0110 form as a displacement — documented quirk).
                ops.push(Op::Pop { dst: VReg(n) });
                i += 2;
                continue;
            }
            if lo == 0 {
                ops.push(Op::LdMem { dst: VReg(n), base: VReg(m), offset: 0, width: 4 });
                i += 2;
                continue;
            }
            ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: w.to_le_bytes().to_vec(),
                        note: "sh mov.l @Rm,Rn with nonzero low nibble".into(),
                    });
            i += 2;
            continue;
        }
        // mov.l Rn,@-r15 → Push; mov.l Rn,@Rm → StMem (offset 0).
        if (w & 0xF000) == 0x2000 {
            let n = ((w >> 8) & 0x0f) as u32; // base register
            let m = ((w >> 4) & 0x0f) as u32; // source register
            let lo = w & 0x000f;
            if n == 15 && lo == 0x6 {
                ops.push(Op::Push { src: VReg(m) });
                i += 2;
                continue;
            }
            if lo == 0x2 {
                ops.push(Op::StMem { src: VReg(m), base: VReg(n), offset: 0, width: 4 });
                i += 2;
                continue;
            }
            ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: w.to_le_bytes().to_vec(),
                        note: "sh mov.l Rn,@Rm with nonzero low nibble".into(),
                    });
            i += 2;
            continue;
        }
        // bra disp12 / bsr disp12 — unconditional branch / call, delay-slot nop appended.
        let group = w & 0xF000;
        if group == 0xA000 || group == 0xB000 {
            let disp = (w & 0x0FFF) as u16 as i16 as i32;
            let disp = if w & 0x0800 != 0 { disp - 0x1000 } else { disp };
            if group == 0xB000 {
                ops.push(Op::CallRel { rel: disp << 1, target: None, symbol: None });
            } else {
                ops.push(Op::JmpRel { rel: disp << 1, target: None, symbol: None });
            }
            i += 2;
            if i + 2 <= bytes.len() {
                let d = u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap());
                if d != 0x0009 {
                    ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: d.to_le_bytes().to_vec(),
                        note: "branch delay slot is not a nop".into(),
                    });
                }
                i += 2;
            }
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
        // Push/Pop idiom folds (8-byte slot on Alpha): lda sp,-8(sp); stq rx,0(sp) /
        // ldq rx,0(sp); lda sp,8(sp). Must precede the general lda/stq/ldq handlers.
        if w == 0x23DEFFF8 && i + 8 <= bytes.len() {
            let w2 = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap());
            if (w2 >> 26) == 0x2D && ((w2 >> 16) & 0x1f) == 30 && (w2 & 0xffff) == 0 {
                ops.push(Op::Push { src: VReg((w2 >> 21) & 0x1f) });
                i += 8;
                continue;
            }
        }
        if (w >> 26) == 0x29 && ((w >> 16) & 0x1f) == 30 && (w & 0xffff) == 0 && i + 8 <= bytes.len() {
            let w2 = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap());
            if w2 == 0x23DE0008 {
                ops.push(Op::Pop { dst: VReg((w >> 21) & 0x1f) });
                i += 8;
                continue;
            }
        }
        if w >> 26 == 0x34 {
            // bsr $26, disp — disp21 signed, scaled by 4.
            let disp = ((w & 0x1F_FFFF) << 11) as i32 >> 11;
            ops.push(Op::CallRel { rel: disp << 2, target: None, symbol: None });
            i += 4;
            continue;
        }
        if w >> 26 == 0x30 {
            // br — unconditional branch.
            let disp = ((w & 0x1F_FFFF) << 11) as i32 >> 11;
            ops.push(Op::JmpRel { rel: disp << 2, target: None, symbol: None });
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
        if w >> 26 == 0x29 {
            // ldq ra, disp(rb) — LdMem (width 8, 64-bit Alpha).
            let ra = (w >> 21) & 0x1f;
            let rb = (w >> 16) & 0x1f;
            ops.push(Op::LdMem {
                dst: VReg(ra),
                base: VReg(rb),
                offset: sext16(w & 0xffff),
                width: 8,
            });
            i += 4;
            continue;
        }
        if w >> 26 == 0x2D {
            // stq ra, disp(rb) — StMem (width 8).
            let ra = (w >> 21) & 0x1f;
            let rb = (w >> 16) & 0x1f;
            ops.push(Op::StMem {
                src: VReg(ra),
                base: VReg(rb),
                offset: sext16(w & 0xffff),
                width: 8,
            });
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
    let sext = |f: u32| (f & 0x7FFF) as i32 - (((f & 0x4000) as i32) << 1); // sign-extend 15-bit
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
        let opc = w >> 26;
        let b = (w >> 21) & 0x1f;
        let t = (w >> 16) & 0x1f;
        let field = w & 0xFFFF;
        let v = |reg: u32| VReg(reg.saturating_sub(3)); // encoder maps VReg → r3..
        // Push/Pop idiom folds (sp = r30): ldo -4(sp),sp; stw t,0(sp) /
        // ldw t,0(sp); ldo 4(sp),sp. Must precede the general ldo/ldw/stw handlers.
        if w == 0x37DEFFF8 && i + 8 <= bytes.len() {
            let w2 = u32::from_be_bytes(bytes[i + 4..i + 8].try_into().unwrap());
            if (w2 >> 26) == 0x1A && ((w2 >> 21) & 0x1f) == 30 && (w2 & 0xFFFF) == 0 {
                ops.push(Op::Push { src: v((w2 >> 16) & 0x1f) });
                i += 8;
                continue;
            }
        }
        if (w >> 26) == 0x12 && b == 30 && (w & 0xFFFF) == 0 && i + 8 <= bytes.len() {
            let w2 = u32::from_be_bytes(bytes[i + 4..i + 8].try_into().unwrap());
            if w2 == 0x37DE0008 {
                ops.push(Op::Pop { dst: v(t) });
                i += 8;
                continue;
            }
        }
        if opc == 0x0D {
            // ldi imm, %rt (base field 0) or ldo disp(%rb), %rt (base != 0).
            if b == 0 {
                ops.push(Op::MovImm { dst: v(t), imm: sext(field >> 1) as u32 });
            } else if t == b {
                ops.push(arith(v(t), sext(field >> 1)));
            } else {
                ops.push(gap(i, w, "parisc ldo with t != b outside encoder subset".into()));
            }
            i += 4;
            continue;
        }
        if opc == 0x12 {
            // ldw disp(%rb), %rt — LdMem (width 4). sp-based stray loads are the pop
            // idiom (folded above); a lone one is outside the subset.
            if b == 30 {
                ops.push(gap(i, w, "parisc ldw with %sp base outside push/pop idiom".into()));
            } else {
                ops.push(Op::LdMem { dst: v(t), base: v(b), offset: sext(field >> 1), width: 4 });
            }
            i += 4;
            continue;
        }
        if opc == 0x1A {
            // stw %rt, disp(%rb) — StMem (width 4).
            if b == 30 {
                ops.push(gap(i, w, "parisc stw with %sp base outside push/pop idiom".into()));
            } else {
                ops.push(Op::StMem { src: v(t), base: v(b), offset: sext(field >> 1), width: 4 });
            }
            i += 4;
            continue;
        }
        if opc == 0x3A {
            // bl/b,l — disp18 at bits 20-3 (displacement LSB), bit 0 = sign (verified
            // against objdump for the rel-0 subset; see encoder). Link register is
            // bits 25-21 (NOT the memory-format t field): rp(r2) = call, r0 = jump.
            let bt = (w >> 21) & 0x1f;
            let disp = ((((w >> 3) & 0x3FFFF) as i32) - (((w & 1) << 18) as i32)) << 2;
            if bt == 2 {
                ops.push(Op::CallRel { rel: disp, target: None, symbol: None });
            } else if bt == 0 {
                ops.push(Op::JmpRel { rel: disp, target: None, symbol: None });
            } else {
                ops.push(gap(i, w, "parisc bl/b,l with link register outside {r0, rp}".into()));
            }
            i += 4;
            if i + 4 <= bytes.len() {
                let d = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
                if d != 0x08000240 {
                    ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: d.to_be_bytes().to_vec(),
                        note: "branch delay slot is not a nop".into(),
                    });
                }
                i += 4;
            }
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
        if w == 0x4AFC {
            ops.push(Op::Trap); // illegal
            i += 2;
            continue;
        }
        if (w & 0xFF00) == 0x6000 || (w & 0xFF00) == 0x6100 {
            // bra.w/bsr.w = 0x60 0x00 / 0x61 0x00 + word16; the .b forms (0x60xx, xx≠0)
            // are outside the encoder subset.
            // Bcc.w: 0x6<cond>00 + word16 (cond in bits 11-8)
            if w == 0x6000 || w == 0x6100 {
                if i + 4 > bytes.len() {
                    return Err(DecodeError::Truncated("coldfire", i as u64));
                }
                let disp = i16::from_be_bytes(bytes[i + 2..i + 4].try_into().unwrap());
                if w == 0x6100 {
                    ops.push(Op::CallRel { rel: disp as i32, target: None, symbol: None });
                } else {
                    ops.push(Op::JmpRel { rel: disp as i32, target: None, symbol: None });
                }
                i += 4;
            } else if (w & 0xF000) == 0x6000 && (w & 0x00FF) == 0x00 {
                // Bcc.w — cond in bits 11-8
                if i + 4 > bytes.len() {
                    return Err(DecodeError::Truncated("coldfire", i as u64));
                }
                let cond_byte = (w >> 8) & 0xF;
                let disp = i16::from_be_bytes(bytes[i + 2..i + 4].try_into().unwrap());
                let cond = match cond_byte {
                    0x7 => Cond::Eq,  // beq
                    0x6 => Cond::Ne,  // bne
                    0xD => Cond::Lt,  // blt
                    0xC => Cond::Ge,  // bge
                    0xF => Cond::Gt,  // bgt
                    0xE => Cond::Le,  // ble
                    0x5 => Cond::Cs,  // bcs/blo
                    0x4 => Cond::Cc,  // bcc/bhs
                    0xB => Cond::Mi,  // bmi
                    0xA => Cond::Pl,  // bpl
                    0x1 => Cond::Vs,  // bvs
                    0x0 => Cond::Vc,  // bvc
                    0x2 => Cond::Hi,  // bhi
                    0x3 => Cond::Ls,  // bls
                    _ => {
                        ops.push(Op::Unknown {
                            offset: i as u64,
                            bytes: w.to_be_bytes().to_vec(),
                            note: format!("coldfire unknown bcc cond {cond_byte:#x}"),
                        });
                        i += 4;
                        continue;
                    }
                };
                ops.push(Op::BranchCond { cond, target: disp as u64 });
                i += 4;
                continue;
            } else {
                ops.push(Op::Unknown {
                    offset: i as u64,
                    bytes: w.to_be_bytes().to_vec(),
                    note: "coldfire bra.b/bsr.b outside encoder subset".into(),
                });
                i += 2;
            }
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
            let dn = ((w >> 9) & 7) as u32;
            ops.push(Op::Pop { dst: VReg(dn) });
            i += 2;
            continue;
        }
        if (w & 0xF1F8) == 0x2010 {
            // move.l (An), Dn — An bits 2-0, Dn bits 11-9 (encoder subset, offset 0).
            ops.push(Op::LdMem {
                dst: VReg(((w >> 9) & 7) as u32),
                base: VReg((w & 7) as u32),
                offset: 0,
                width: 4,
            });
            i += 2;
            continue;
        }
        if (w & 0xF1F8) == 0x2080 {
            // move.l Dn, (An) — An bits 11-9, Dn bits 2-0 (MOVE Dn→mem form).
            ops.push(Op::StMem {
                src: VReg((w & 7) as u32),
                base: VReg(((w >> 9) & 7) as u32),
                offset: 0,
                width: 4,
            });
            i += 2;
            continue;
        }
        // CMP.L Dn, Dm — 0xB140 | (Dn << 9) | Dm
        if (w & 0xFFC0) == 0xB140 {
            let dn = ((w >> 9) & 7) as u32;
            let dm = (w & 7) as u32;
            ops.push(Op::Cmp { rd: VReg(dn), rs: VReg(dm) });
            i += 2;
            continue;
        }
        // TST.L Dn — 0x4A00 | (Dn << 3). Note: ColdFire TST only has one operand.
        // For two-operand Test, we map to CMP with zero (handled by encoder as CMP).
        if (w & 0xFFF8) == 0x4A00 {
            let dn = ((w >> 3) & 7) as u32;
            ops.push(Op::Test { rd: VReg(dn), rs: VReg(0) });
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
            0x2F00 => {
                ops.push(Op::Push { src: VReg((w & 7) as u32) }); // move.l Dn, -(A7)
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
        if w == 0xE1200070 {
            ops.push(Op::Trap); // BKPT #0
            i += 4;
            continue;
        }
        // push {rX} / pop {rX} — single-register STMDB/LDMIA with base sp.
        let single = |reglist: u32| -> Option<VReg> {
            if reglist & (reglist - 1) == 0 && reglist != 0 {
                let bit = reglist.trailing_zeros();
                (bit < 13).then(|| VReg(bit)) // sp/lr/pc registers are outside the subset
            } else {
                None
            }
        };
        if (w & 0xFFFF_0000) == 0xE92D_0000 {
            match single(w & 0xFFFF) {
                Some(reg) => ops.push(Op::Push { src: reg }),
                None => ops.push(gap(i, w, "arm push with multi-register list (outside subset)".into())),
            }
            i += 4;
            continue;
        }
        if (w & 0xFFFF_0000) == 0xE8BD_0000 {
            match single(w & 0xFFFF) {
                Some(reg) => ops.push(Op::Pop { dst: reg }),
                None => ops.push(gap(i, w, "arm pop with multi-register list (outside subset)".into())),
            }
            i += 4;
            continue;
        }
        if (w & 0xFF00_0000) == 0xEB00_0000 {
            // bl +imm24 (cond AL, L=1) — llvm-mc verified.
            let imm = ((w & 0x00FF_FFFF) << 8) as i32 >> 8;
            ops.push(Op::CallRel { rel: imm << 2, target: None, symbol: None });
            i += 4;
            continue;
        }
        if (w & 0xFF00_0000) == 0xEA00_0000 {
            // b +imm24 (cond AL, L=0).
            let imm = ((w & 0x00FF_FFFF) << 8) as i32 >> 8;
            ops.push(Op::JmpRel { rel: imm << 2, target: None, symbol: None });
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
        if (w & 0xFFF0_0F00) == 0xE3E0_0000 && (w & 0x0000_00FF) == 0 {
            // mvn rd, #0 → rd = -1 (encoder's MovImm{0xFFFFFFFF} form, llvm-mc verified).
            let rd = (w >> 12) & 0xf;
            ops.push(Op::MovImm { dst: VReg(rd), imm: 0xFFFF_FFFF });
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
        // LDR/STR rd, [rn] — single data transfer (P=1 U=1 B=0 W=0, L=1/0), offset 0.
        if (w & 0xFFF0_0000) == 0xE590_0000 {
            // ldr rd, [rn] — capstone-verified (0xE5910000 = ldr r0, [r1]).
            if w & 0xfff != 0 {
                ops.push(gap(i, w, "arm ldr with imm offset != 0".into()));
            } else {
                ops.push(Op::LdMem {
                    dst: VReg((w >> 12) & 0xf),
                    base: VReg((w >> 16) & 0xf),
                    offset: 0,
                    width: 4,
                });
            }
            i += 4;
            continue;
        }
        if (w & 0xFFF0_0000) == 0xE580_0000 {
            // str rd, [rn].
            if w & 0xfff != 0 {
                ops.push(gap(i, w, "arm str with imm offset != 0".into()));
            } else {
                ops.push(Op::StMem {
                    src: VReg((w >> 12) & 0xf),
                    base: VReg((w >> 16) & 0xf),
                    offset: 0,
                    width: 4,
                });
            }
            i += 4;
            continue;
        }
        // CMP: 0xE1500000 | (Rn << 16) | Rm — cond=AL, opcode=10101, S=1, Rd=0, Rn=Rn, Rm=Rm
        if (w & 0xFFFF_F0F0) == 0xE150_0000 {
            let rn = (w >> 16) & 0xf;
            let rm = w & 0xf;
            ops.push(Op::Cmp { rd: VReg(rn), rs: VReg(rm) });
            i += 4;
            continue;
        }
        // TST: 0xE1100000 | (Rn << 16) | Rm
        if (w & 0xFFFF_F0F0) == 0xE110_0000 {
            let rn = (w >> 16) & 0xf;
            let rm = w & 0xf;
            ops.push(Op::Test { rd: VReg(rn), rs: VReg(rm) });
            i += 4;
            continue;
        }
        // B<cond>: cond in bits 31-28, 101 in bits 27-25, imm24 in bits 23-0
        if (w & 0x0E00_0000) == 0x0A00_0000 {
            let cond_val = (w >> 28) & 0xF;
            let imm24 = w & 0x00FF_FFFF;
            let cond = match cond_val {
                0x0 => Cond::Eq,
                0x1 => Cond::Ne,
                0x2 => Cond::Cs,
                0x3 => Cond::Cc,
                0x4 => Cond::Mi,
                0x5 => Cond::Pl,
                0x6 => Cond::Vs,
                0x7 => Cond::Vc,
                0x8 => Cond::Hi,
                0x9 => Cond::Ls,
                0xA => Cond::Ge,
                0xB => Cond::Lt,
                0xC => Cond::Gt,
                0xD => Cond::Le,
                _ => {
                    ops.push(gap(i, w, format!("arm unknown cond {cond_val:#x}")));
                    i += 4;
                    continue;
                }
            };
            let target = ((imm24 as i32) << 2) as u64;
            ops.push(Op::BranchCond { cond, target });
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
        if w == 0xD4200000 {
            ops.push(Op::Trap); // BRK #0
            i += 4;
            continue;
        }
        // str wX,[sp,#-4]! / ldr wX,[sp],#4 — single-word stack ops (llvm-mc verified).
        if (w & 0xFFFF_FFE0) == 0xB81F_CFE0 {
            ops.push(Op::Push { src: VReg(w & 0x1f) });
            i += 4;
            continue;
        }
        if (w & 0xFFFF_FFE0) == 0xB840_47E0 {
            ops.push(Op::Pop { dst: VReg(w & 0x1f) });
            i += 4;
            continue;
        }
        if (w >> 26) == 0x05 {
            // b +imm26 — displacement scaled by 4.
            let imm = ((w & 0x03FF_FFFF) << 6) as i32 >> 6;
            ops.push(Op::JmpRel { rel: imm << 2, target: None, symbol: None });
            i += 4;
            continue;
        }
        if (w >> 26) == 0x25 {
            // bl +imm26.
            let imm = ((w & 0x03FF_FFFF) << 6) as i32 >> 6;
            ops.push(Op::CallRel { rel: imm << 2, target: None, symbol: None });
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
        if (w & 0xFFFF_FFE0) == 0x1280_0000 {
            // MOVN Wd, #0 → Wd = -1 (encoder's MovImm{0xFFFFFFFF} form, llvm-mc verified).
            ops.push(Op::MovImm { dst: VReg(w & 0x1f), imm: 0xFFFF_FFFF });
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
        if (w & 0xFFC0_0000) == 0xB940_0000 {
            // ldr wT, [wB] (offset=0, unsigned imm field must be 0 — encoder subset).
            if (w >> 10) & 0xfff != 0 {
                ops.push(gap(i, w, "aarch64 ldr with scaled imm != 0".into()));
            } else {
                ops.push(Op::LdMem {
                    dst: VReg(w & 0x1f),
                    base: VReg((w >> 5) & 0x1f),
                    offset: 0,
                    width: 4,
                });
            }
            i += 4;
            continue;
        }
        if (w & 0xFFC0_0000) == 0xB900_0000 {
            // str wT, [wB] (offset=0).
            if (w >> 10) & 0xfff != 0 {
                ops.push(gap(i, w, "aarch64 str with scaled imm != 0".into()));
            } else {
                ops.push(Op::StMem {
                    src: VReg(w & 0x1f),
                    base: VReg((w >> 5) & 0x1f),
                    offset: 0,
                    width: 4,
                });
            }
            i += 4;
            continue;
        }
        // CMP: SUBS WZR, Wn, Wm — 0x6B000000 | (Rn << 16) | (Rm << 5) | 0x1F
        if (w & 0xFFE0_FFE0) == 0x6B00_001F {
            let rn = (w >> 16) & 0x1f;
            let rm = (w >> 5) & 0x1f;
            ops.push(Op::Cmp { rd: VReg(rn), rs: VReg(rm) });
            i += 4;
            continue;
        }
        // TST: ANDS WZR, Wn, Wm — 0x6A000000 | (Rn << 16) | (Rm << 5) | 0x1F
        if (w & 0xFFE0_FFE0) == 0x6A00_001F {
            let rn = (w >> 16) & 0x1f;
            let rm = (w >> 5) & 0x1f;
            ops.push(Op::Test { rd: VReg(rn), rs: VReg(rm) });
            i += 4;
            continue;
        }
        // B.cond: 0x54000000 | (cond << 24) | (imm19 << 5)
        if (w & 0xFE00_0000) == 0x5400_0000 {
            let cond_val = (w >> 24) & 0xF;
            let imm19 = (w >> 5) & 0x7FFFF;
            let cond = match cond_val {
                0x0 => Cond::Eq,
                0x1 => Cond::Ne,
                0x2 => Cond::Cs,
                0x3 => Cond::Cc,
                0x4 => Cond::Mi,
                0x5 => Cond::Pl,
                0x6 => Cond::Vs,
                0x7 => Cond::Vc,
                0x8 => Cond::Hi,
                0x9 => Cond::Ls,
                0xA => Cond::Ge,
                0xB => Cond::Lt,
                0xC => Cond::Gt,
                0xD => Cond::Le,
                _ => {
                    ops.push(gap(i, w, format!("aarch64 unknown cond {cond_val:#x}")));
                    i += 4;
                    continue;
                }
            };
            let target = ((imm19 as i64) << 2) as u64;
            ops.push(Op::BranchCond { cond, target });
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
            0x0F => {
                // Two-byte escape: 0F 0B = ud2 (trap).
                if i + 1 < bytes.len() && bytes[i + 1] == 0x0B {
                    ops.push(Op::Trap);
                    i += 2;
                } else {
                    ops.push(gap(i, w32(bytes, i), "x86 unknown 0F escape".into()));
                    i += 2;
                }
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
            0x8B => {
                // mov reg, r/m32 — encoder emits mod=00, reg=eax(000), rm=base (LdMem).
                need(i, 2, "x86 8B")?;
                let modrm = bytes[i + 1];
                if modrm >> 6 != 0 || (modrm >> 3) & 7 != 0 {
                    ops.push(gap(i, w32(bytes, i), "x86 8B not [base]→eax".into()));
                } else {
                    ops.push(Op::LdMem {
                        dst: VReg(0),
                        base: VReg((modrm & 7) as u32),
                        offset: 0,
                        width: 4,
                    });
                }
                i += 2;
            }
            0x89 => {
                // mov r/m32, reg — encoder emits mod=00, reg=eax(000), rm=base (StMem).
                need(i, 2, "x86 89")?;
                let modrm = bytes[i + 1];
                if modrm >> 6 != 0 || (modrm >> 3) & 7 != 0 {
                    ops.push(gap(i, w32(bytes, i), "x86 89 not eax→[base]".into()));
                } else {
                    ops.push(Op::StMem {
                        src: VReg(0),
                        base: VReg((modrm & 7) as u32),
                        offset: 0,
                        width: 4,
                    });
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
            0xE8 => {
                // call rel32 — displacement relative to the NEXT instruction.
                need(i, 5, "x86 call")?;
                let rel = i32::from_le_bytes(bytes[i + 1..i + 5].try_into().unwrap());
                ops.push(Op::CallRel { rel, target: None, symbol: None });
                i += 5;
            }
            0xE9 => {
                // jmp rel32.
                need(i, 5, "x86 jmp")?;
                let rel = i32::from_le_bytes(bytes[i + 1..i + 5].try_into().unwrap());
                ops.push(Op::JmpRel { rel, target: None, symbol: None });
                i += 5;
            }
            0x39 => {
                // cmp r/m32, r32 — ModRM follows. Encoder emits mod=11 (reg-reg).
                need(i, 2, "x86 cmp")?;
                let modrm = bytes[i + 1];
                if modrm >> 6 != 3 {
                    ops.push(gap(i, w32(bytes, i), "x86 cmp mod != 11".into()));
                } else {
                    let reg = (modrm >> 3) & 7; // source
                    let rm = modrm & 7; // dest
                    ops.push(Op::Cmp { rd: VReg(rm as u32), rs: VReg(reg as u32) });
                }
                i += 2;
            }
            0x85 => {
                // test r/m32, r32 — ModRM follows. Encoder emits mod=11 (reg-reg).
                need(i, 2, "x86 test")?;
                let modrm = bytes[i + 1];
                if modrm >> 6 != 3 {
                    ops.push(gap(i, w32(bytes, i), "x86 test mod != 11".into()));
                } else {
                    let reg = (modrm >> 3) & 7;
                    let rm = modrm & 7;
                    ops.push(Op::Test { rd: VReg(rm as u32), rs: VReg(reg as u32) });
                }
                i += 2;
            }
            0x0F => {
                // 0F 80-8F: Jcc rel32
                if i + 2 >= bytes.len() {
                    return Err(DecodeError::Truncated("x86", i as u64));
                }
                let b2 = bytes[i + 1];
                if (0x80..=0x8F).contains(&b2) {
                    need(i, 6, "x86 jcc")?;
                    let rel = i32::from_le_bytes(bytes[i + 2..i + 6].try_into().unwrap());
                    let cond = match b2 {
                        0x84 => Cond::Eq,  // je
                        0x85 => Cond::Ne,  // jne
                        0x8C => Cond::Lt,  // jl
                        0x8D => Cond::Ge,  // jge
                        0x8F => Cond::Gt,  // jg
                        0x8E => Cond::Le,  // jle
                        0x82 => Cond::Cs,  // jb/jc
                        0x83 => Cond::Cc,  // jae/jnc
                        0x88 => Cond::Mi,  // js
                        0x89 => Cond::Pl,  // jns
                        0x80 => Cond::Vs,  // jo
                        0x81 => Cond::Vc,  // jno
                        0x87 => Cond::Hi,  // ja
                        0x86 => Cond::Ls,  // jbe
                        _ => {
                            ops.push(Op::Unknown {
                                offset: i as u64,
                                bytes: bytes[i..i + 6].to_vec(),
                                note: format!("x86 unknown jcc {b2:#04x}"),
                            });
                            i += 6;
                            continue;
                        }
                    };
                    ops.push(Op::BranchCond { cond, target: rel as u64 });
                    i += 6;
                    continue;
                }
                ops.push(Op::Unknown {
                    offset: i as u64,
                    bytes: vec![0x0F, b2],
                    note: format!("x86 0F {b2:#04x} outside encoder subset"),
                });
                i += 2;
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
        if w == 0x91D00001 {
            ops.push(Op::Trap); // ta 1
            i += 4;
            continue;
        }
        let op = w >> 30;
        let op3 = (w >> 19) & 0x3f;
        let i_bit = (w >> 13) & 1;
        let rd = (w >> 25) & 0x1f;
        let rs1 = (w >> 14) & 0x1f;
        let imm13 = w & 0x1fff;
        let imm13 = if imm13 & 0x1000 != 0 { (imm13 as i32 - 0x2000) } else { imm13 as i32 };
        // Push/Pop idiom folds (sp = %o6 = 14): add %sp,-4,%sp; st rd,[%sp] /
        // ld [%sp],rd; add %sp,4,%sp. Must precede the general add/sub handler.
        if w == 0x9C03BFFC && i + 8 <= bytes.len() {
            let w2 = u32::from_be_bytes(bytes[i + 4..i + 8].try_into().unwrap());
            if (w2 >> 30) == 3 && (w2 >> 19) & 0x3f == 4
                && ((w2 >> 14) & 0x1f) == 14 && (w2 >> 13) & 1 == 1 && (w2 & 0x1fff) == 0
            {
                let r = (w2 >> 25) & 0x1f;
                if (16..24).contains(&r) {
                    ops.push(Op::Push { src: VReg(r - 16) });
                    i += 8;
                    continue;
                }
            }
        }
        if (w >> 30) == 3 && op3 == 0 && rs1 == 14 && i_bit == 1 && imm13 == 0
            && i + 8 <= bytes.len()
        {
            let w2 = u32::from_be_bytes(bytes[i + 4..i + 8].try_into().unwrap());
            if w2 == 0x9C03A004 && (16..24).contains(&rd) {
                ops.push(Op::Pop { dst: VReg(rd - 16) });
                i += 8;
                continue;
            }
        }
        if op == 2 && (op3 == 0 || op3 == 4) && i_bit == 1 && rd == rs1 && (16..24).contains(&rd) {
            // add/sub %rd, imm13, %rd — SIR arith on the %l0..%l7 window.
            if op3 == 4 {
                ops.push(arith(VReg(rd - 16), -imm13));
            } else {
                ops.push(arith(VReg(rd - 16), imm13));
            }
            i += 4;
            continue;
        }
        if op == 1 {
            // call disp30 — encoder appends the delay-slot nop (fold it).
            let disp = (w & 0x3FFF_FFFF) as i32;
            let disp = if disp & 0x2000_0000 != 0 { disp - 0x4000_0000 } else { disp };
            ops.push(Op::CallRel { rel: disp << 2, target: None, symbol: None });
            i += 4;
            if i + 4 <= bytes.len() {
                let d = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
                if d != 0x01000000 {
                    ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: d.to_be_bytes().to_vec(),
                        note: "call delay slot is not a nop".into(),
                    });
                }
                i += 4;
            }
            continue;
        }
        if op == 0 && ((w >> 22) & 7) == 2 {
            // b<cond> disp22 or ba disp22 — conditional/unconditional branch on icc.
            let cond = (w >> 25) & 0xf;
            let annul = (w >> 29) & 1;
            let disp = (w & 0x3F_FFFF) as i32;
            let disp = if disp & 0x20_0000 != 0 { disp - 0x40_0000 } else { disp };
            // Only accept annul=0 (delay-slot nop always executed).
            if cond == 8 && annul == 0 {
                // ba — unconditional branch.
                ops.push(Op::JmpRel { rel: disp << 2, target: None, symbol: None });
            } else if annul == 0 {
                // b<cond> — conditional branch on icc.
                let sir_cond = match cond {
                    1 => Some(Cond::Eq),   // be
                    2 => Some(Cond::Le),   // ble
                    3 => Some(Cond::Lt),   // bl
                    4 => Some(Cond::Ls),   // bleu
                    5 => Some(Cond::Cs),   // bcs
                    6 => Some(Cond::Mi),   // bneg
                    7 => Some(Cond::Vs),   // bvs
                    9 => Some(Cond::Ne),   // bne
                    10 => Some(Cond::Gt),  // bg
                    11 => Some(Cond::Ge),  // bge
                    12 => Some(Cond::Hi),  // bgu
                    13 => Some(Cond::Cc),  // bcc
                    14 => Some(Cond::Pl),  // bpos
                    15 => Some(Cond::Vc),  // bvc
                    _ => None,
                };
                if let Some(c) = sir_cond {
                    ops.push(Op::BranchCond { cond: c, target: (disp << 2) as u64 });
                } else {
                    ops.push(gap(i, w, format!("sparc unknown bcond cond={cond}")));
                    i += 4;
                    continue;
                }
            } else {
                ops.push(gap(i, w, format!("sparc branch annul bit set, cond={cond}")));
                i += 4;
                continue;
            }
            i += 4;
            if i + 4 <= bytes.len() {
                let d = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
                if d != 0x01000000 {
                    ops.push(Op::Unknown {
                        offset: i as u64,
                        bytes: d.to_be_bytes().to_vec(),
                        note: "branch delay slot is not a nop".into(),
                    });
                }
                i += 4;
            }
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
        if (w & 0xC1F0_3FFF) == 0xC000_2000 {
            // ld [%rs], %rd — op=3, op3=0, i=1, imm13=0 (capstone-verified).
            let rd = (w >> 25) & 0x1f;
            let rs = (w >> 14) & 0x1f;
            if !(16..24).contains(&rd) || !(16..24).contains(&rs) {
                ops.push(gap(i, w, "sparc ld outside %l0..%l7".into()));
            } else {
                ops.push(Op::LdMem {
                    dst: VReg(rd - 16),
                    base: VReg(rs - 16),
                    offset: 0,
                    width: 4,
                });
            }
            i += 4;
            continue;
        }
        if (w & 0xC1F0_3FFF) == 0xC020_2000 {
            // st %rd, [%rs] — op=3, op3=0x04, i=1, imm13=0.
            let rd = (w >> 25) & 0x1f;
            let rs = (w >> 14) & 0x1f;
            if !(16..24).contains(&rd) || !(16..24).contains(&rs) {
                ops.push(gap(i, w, "sparc st outside %l0..%l7".into()));
            } else {
                ops.push(Op::StMem {
                    src: VReg(rd - 16),
                    base: VReg(rs - 16),
                    offset: 0,
                    width: 4,
                });
            }
            i += 4;
            continue;
        }
        // subcc %rs1, %rs2, %g0 → Cmp (sets icc from rs1-rs2, discards result).
        // op=2, op3=0x14, rd=0, i=0.
        if (w & 0xC1F7_FFFF) == 0x80A0_0000 {
            let rs1 = (w >> 14) & 0x1f;
            let rs2 = w & 0x1f;
            if (16..24).contains(&rs1) && (16..24).contains(&rs2) {
                ops.push(Op::Cmp { rd: VReg(rs1 - 16), rs: VReg(rs2 - 16) });
                i += 4;
                continue;
            }
        }
        // andcc %rs1, %rs2, %g0 → Test (sets icc from rs1&rs2, discards result).
        // op=2, op3=0x11, rd=0, i=0.
        if (w & 0xC1F7_FFFF) == 0x8088_0000 {
            let rs1 = (w >> 14) & 0x1f;
            let rs2 = w & 0x1f;
            if (16..24).contains(&rs1) && (16..24).contains(&rs2) {
                ops.push(Op::Test { rd: VReg(rs1 - 16), rs: VReg(rs2 - 16) });
                i += 4;
                continue;
            }
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
        // add eax,2 now encodes (add %l0,2,%l0) and round-trips.
        let ops = roundtrip(&[0x83, 0xC0, 0x02, 0xC3], TargetIsa::Sparc).unwrap();
        assert_eq!(ops, vec![Op::AddImm { dst: VReg(0), imm: 2 }, Op::Ret]);
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
