//! Encode SIR → machine bytes (portable / WASM-friendly). No host binutils required.
//!
//! Covers the **subset** of ops we lift. Call/Jmp with unresolved symbols → error gap bytes skipped /
//! return EncodeError. SuperH included (Keystone does not).

use thiserror::Error;

use crate::sir::{Module, Op, VReg};
use crate::target::{SuperHFlavor, TargetIsa};

#[derive(Debug, Error)]
pub enum EncodeError {
    #[error("cannot encode unresolved call/jmp (need symbol)")]
    UnresolvedBranch,
    #[error("unsupported op for encode on {0}: {1}")]
    Unsupported(TargetIsa, String),
}

/// Encode first function's entry block to raw machine code for `target`.
pub fn encode_module(module: &Module, target: TargetIsa) -> Result<Vec<u8>, EncodeError> {
    let ops = &module.functions[0].blocks[0].ops;
    let mut out = Vec::new();
    for op in ops {
        out.extend(encode_op(op, target)?);
    }
    Ok(out)
}

fn encode_op(op: &Op, target: TargetIsa) -> Result<Vec<u8>, EncodeError> {
    match target {
        TargetIsa::X86_64 => encode_x86(op),
        TargetIsa::Arm => encode_arm(op),
        TargetIsa::AArch64 => encode_aarch64(op),
        TargetIsa::Mips => encode_mips(op),
        TargetIsa::Ppc => encode_ppc(op),
        TargetIsa::Sparc => encode_sparc(op),
        TargetIsa::SuperH(_) => encode_superh(op),
        TargetIsa::Alpha => encode_alpha(op),
        TargetIsa::PaRisc => encode_parisc(op),
        TargetIsa::M88k => encode_m88k(op),
        TargetIsa::Ia64 => encode_ia64(op),
        TargetIsa::I860 => encode_i860(op),
        TargetIsa::ColdFire => encode_coldfire(op),
    }
}

fn encode_x86(op: &Op) -> Result<Vec<u8>, EncodeError> {
    Ok(match op {
        Op::Nop => vec![0x90],
        Op::Ret => vec![0xC3],
        Op::MovImm { dst, imm } => {
            let mut v = vec![0xB8 + (dst.0 as u8 & 7)];
            v.extend_from_slice(&imm.to_le_bytes());
            v
        }
        Op::AddImm { dst, imm } if dst.0 == 0 && *imm > 0x7f => {
            let mut v = vec![0x05];
            v.extend_from_slice(&imm.to_le_bytes());
            v
        }
        Op::AddImm { dst, imm } => {
            vec![0x83, 0xC0 | (dst.0 as u8 & 7), *imm as u8]
        }
        Op::SubImm { dst, imm } if dst.0 == 0 && *imm > 0x7f => {
            let mut v = vec![0x2D];
            v.extend_from_slice(&imm.to_le_bytes());
            v
        }
        Op::SubImm { dst, imm } => {
            vec![0x83, 0xE8 | (dst.0 as u8 & 7), *imm as u8]
        }
        Op::Clear { dst } => vec![0x31, 0xC0 | ((dst.0 as u8 & 7) << 3) | (dst.0 as u8 & 7)],
        Op::Inc { dst } => vec![0x40 + (dst.0 as u8 & 7)],
        Op::Dec { dst } => vec![0x48 + (dst.0 as u8 & 7)],
        Op::Push { src } => vec![0x50 + (src.0 as u8 & 7)],
        Op::Pop { dst } => vec![0x58 + (dst.0 as u8 & 7)],
        Op::LdMem { .. } | Op::StMem { .. } => {
            return Err(EncodeError::Unsupported(
                TargetIsa::X86_64,
                "LdMem/StMem encoder pending (x86 addressing)".into(),
            ))
        }
        Op::CallRel { symbol: Some(_), .. } | Op::JmpRel { symbol: Some(_), .. } => {
            return Err(EncodeError::UnresolvedBranch); // need linker for reloc
        }
        Op::CallRel { .. } | Op::JmpRel { .. } => return Err(EncodeError::UnresolvedBranch),
        Op::Unknown { .. } => return Err(EncodeError::Unsupported(TargetIsa::X86_64, "Unknown".into())),
    })
}

fn encode_arm(op: &Op) -> Result<Vec<u8>, EncodeError> {
    // ARM condition AL=0xE
    let enc = |opc: u32| opc.to_le_bytes().to_vec();
    Ok(match op {
        Op::Nop => enc(0xE320F000), // NOP
        Op::Ret => enc(0xE12FFF1E), // BX LR
        Op::Clear { dst } => {
            // MOV Rd, #0
            let rd = dst.0.min(12);
            enc(0xE3A00000 | (rd << 12))
        }
        Op::MovImm { dst, imm } if *imm <= 0xff => {
            let rd = dst.0.min(12);
            enc(0xE3A00000 | (rd << 12) | (*imm & 0xff))
        }
        Op::AddImm { dst, imm } if *imm <= 0xff => {
            let rd = dst.0.min(12);
            enc(0xE2800000 | (rd << 12) | (rd << 16) | (*imm & 0xff))
        }
        Op::SubImm { dst, imm } if *imm <= 0xff => {
            let rd = dst.0.min(12);
            enc(0xE2400000 | (rd << 12) | (rd << 16) | (*imm & 0xff))
        }
        Op::Inc { dst } => {
            let rd = dst.0.min(12);
            enc(0xE2800000 | (rd << 12) | (rd << 16) | 1)
        }
        Op::Dec { dst } => {
            let rd = dst.0.min(12);
            enc(0xE2400000 | (rd << 12) | (rd << 16) | 1)
        }
        other => {
            return Err(EncodeError::Unsupported(
                TargetIsa::Arm,
                format!("{other:?}"),
            ))
        }
    })
}

fn encode_aarch64(op: &Op) -> Result<Vec<u8>, EncodeError> {
    let enc = |opc: u32| opc.to_le_bytes().to_vec();
    Ok(match op {
        Op::Nop => enc(0xD503201F),
        Op::Ret => enc(0xD65F03C0),
        Op::Clear { dst } => {
            let wd = dst.0.min(30);
            // MOV Wd, WZR
            enc(0x2A1F03E0 | wd)
        }
        Op::MovImm { dst, imm } if *imm <= 0xffff => {
            let wd = dst.0.min(30);
            enc(0x52800000 | ((*imm & 0xffff) << 5) | wd)
        }
        Op::AddImm { dst, imm } if *imm <= 0xfff => {
            let wd = dst.0.min(30);
            enc(0x11000000 | ((*imm & 0xfff) << 10) | (wd << 5) | wd)
        }
        Op::SubImm { dst, imm } if *imm <= 0xfff => {
            let wd = dst.0.min(30);
            enc(0x51000000 | ((*imm & 0xfff) << 10) | (wd << 5) | wd)
        }
        Op::Inc { dst } => {
            let wd = dst.0.min(30);
            enc(0x11000400 | (wd << 5) | wd) // add wd, wd, #1
        }
        Op::Dec { dst } => {
            let wd = dst.0.min(30);
            enc(0x51000400 | (wd << 5) | wd)
        }
        other => {
            return Err(EncodeError::Unsupported(
                TargetIsa::AArch64,
                format!("{other:?}"),
            ))
        }
    })
}

fn encode_mips(op: &Op) -> Result<Vec<u8>, EncodeError> {
    let enc = |opc: u32| opc.to_be_bytes().to_vec(); // classic big-endian MIPS
    let treg = |v: VReg| 8 + (v.0.min(7)); // $t0..
    Ok(match op {
        Op::Nop => enc(0),
        Op::Ret => {
            let mut v = enc(0x03E00008); // jr $ra
            v.extend(enc(0));
            v
        }
        Op::Clear { dst } => {
            let t = treg(*dst);
            // or $t, $zero, $zero
            enc(0x00000025 | (t << 11))
        }
        Op::MovImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            let t = treg(*dst);
            // addiu $t, $zero, imm
            enc(0x24000000 | (t << 16) | (*imm as u16 as u32))
        }
        Op::AddImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            let t = treg(*dst);
            enc(0x24000000 | (t << 21) | (t << 16) | (*imm as u16 as u32))
        }
        Op::SubImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            let t = treg(*dst);
            let neg = (-(*imm as i32)) as u16 as u32;
            enc(0x24000000 | (t << 21) | (t << 16) | neg)
        }
        Op::Inc { dst } => {
            let t = treg(*dst);
            enc(0x24000000 | (t << 21) | (t << 16) | 1)
        }
        Op::Dec { dst } => {
            let t = treg(*dst);
            enc(0x24000000 | (t << 21) | (t << 16) | 0xffff)
        }
        other => {
            return Err(EncodeError::Unsupported(
                TargetIsa::Mips,
                format!("{other:?}"),
            ))
        }
    })
}

fn encode_ppc(op: &Op) -> Result<Vec<u8>, EncodeError> {
    let enc = |opc: u32| opc.to_be_bytes().to_vec();
    // VReg → r3..r31: r0 reads as 0 in addi (RA field) and r1/r2 are ABI-special.
    let r = |v: VReg| 3 + v.0.min(28);
    Ok(match op {
        Op::Nop => enc(0x60000000), // ori 0,0,0
        Op::Ret => enc(0x4E800020), // blr
        Op::Clear { dst } => {
            let rr = r(*dst);
            // li rr,0 = addi rr,0,0
            enc(0x38000000 | (rr << 21))
        }
        Op::MovImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            let rr = r(*dst);
            enc(0x38000000 | (rr << 21) | (*imm as u16 as u32))
        }
        Op::AddImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            let rr = r(*dst);
            enc(0x38000000 | (rr << 21) | (rr << 16) | (*imm as u16 as u32))
        }
        Op::SubImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            let rr = r(*dst);
            let neg = (-(*imm as i32)) as u16 as u32;
            enc(0x38000000 | (rr << 21) | (rr << 16) | neg)
        }
        Op::Inc { dst } => {
            let rr = r(*dst);
            enc(0x38000000 | (rr << 21) | (rr << 16) | 1)
        }
        Op::Dec { dst } => {
            let rr = r(*dst);
            enc(0x38000000 | (rr << 21) | (rr << 16) | 0xffff)
        }
        other => {
            return Err(EncodeError::Unsupported(
                TargetIsa::Ppc,
                format!("{other:?}"),
            ))
        }
    })
}

fn encode_sparc(op: &Op) -> Result<Vec<u8>, EncodeError> {
    let enc = |opc: u32| opc.to_be_bytes().to_vec();
    Ok(match op {
        Op::Nop => enc(0x01000000),
        Op::Ret => {
            // retl = jmpl %o7+8, %g0 ; nop — simplified: restore; ret style
            // retl: 0x81C3E008
            let mut v = enc(0x81C3E008);
            v.extend(enc(0x01000000));
            v
        }
        Op::Clear { dst } => {
            let r = 16 + dst.0.min(7); // %l0..
            // clr = or %g0, %g0, rd
            enc(0x80100000 | (r << 25))
        }
        Op::MovImm { dst, imm } if (*imm as i32) >= -4096 && (*imm as i32) <= 4095 => {
            let r = 16 + dst.0.min(7);
            // or %g0, imm, rd
            enc(0x80102000 | (r << 25) | (*imm as u32 & 0x1fff))
        }
        other => {
            return Err(EncodeError::Unsupported(
                TargetIsa::Sparc,
                format!("{other:?}"),
            ))
        }
    })
}

fn encode_superh(op: &Op) -> Result<Vec<u8>, EncodeError> {
    let h = |w: u16| w.to_le_bytes().to_vec();
    Ok(match op {
        Op::Nop => h(0x0009),
        Op::Ret => {
            let mut v = h(0x000B); // rts
            v.extend(h(0x0009)); // delay nop
            v
        }
        Op::MovImm { dst, imm } if *imm <= 0x7f => {
            let n = dst.0.min(15) as u16;
            h(0xE000 | (n << 8) | (*imm as u16 & 0xff))
        }
        Op::AddImm { dst, imm } if *imm <= 0x7f => {
            let n = dst.0.min(15) as u16;
            h(0x7000 | (n << 8) | (*imm as u16 & 0xff))
        }
        Op::SubImm { dst, imm } if *imm <= 0x7f => {
            let n = dst.0.min(15) as u16;
            let neg = (-(*imm as i8)) as u8 as u16;
            h(0x7000 | (n << 8) | neg)
        }
        Op::Clear { dst } => {
            let n = dst.0.min(15) as u16;
            h(0xE000 | (n << 8)) // mov #0, Rn
        }
        Op::Inc { dst } => {
            let n = dst.0.min(15) as u16;
            h(0x7000 | (n << 8) | 1)
        }
        Op::Dec { dst } => {
            let n = dst.0.min(15) as u16;
            h(0x7000 | (n << 8) | 0xff)
        }
        other => {
            return Err(EncodeError::Unsupported(
                TargetIsa::SuperH(SuperHFlavor::Sh2),
                format!("{other:?}"),
            ))
        }
    })
}

fn encode_alpha(op: &Op) -> Result<Vec<u8>, EncodeError> {
    // DEC Alpha: 32-bit LE words, [opcode(6) | RA(5) | RB(5) | ...].
    // Immediate moves/adds via LDA (opcode 0x08, 16-bit signed disp); RET is the
    // JMP-family (0x1A) with function 2 — see LLVM AlphaInstrInfo + GNU alpha-opc.
    let enc = |w: u32| w.to_le_bytes().to_vec();
    let r = |v: VReg| v.0.min(31);
    let lda = |rd: u32, disp: u32, rb: u32| 0x20000000 | (rd << 21) | (rb << 16) | disp;
    Ok(match op {
        Op::Nop => enc(0x23FF0000), // lda r31, 0(r31)
        Op::Ret => enc(0x6BFA8001), // ret r31, (r26) — hint 1
        Op::Clear { dst } => {
            let rd = r(*dst);
            enc(lda(rd, 0, 31)) // lda rd, 0(r31)
        }
        Op::MovImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            let rd = r(*dst);
            enc(lda(rd, *imm as u16 as u32, 31)) // lda rd, imm(r31)
        }
        Op::AddImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            let rd = r(*dst);
            enc(lda(rd, *imm as u16 as u32, rd)) // lda rd, imm(rd)
        }
        Op::SubImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            let rd = r(*dst);
            let neg = (-(*imm as i32)) as u16 as u32;
            enc(lda(rd, neg, rd))
        }
        Op::Inc { dst } => {
            let rd = r(*dst);
            enc(lda(rd, 1, rd))
        }
        Op::Dec { dst } => {
            let rd = r(*dst);
            enc(lda(rd, 0xFFFF, rd))
        }
        other => {
            return Err(EncodeError::Unsupported(
                TargetIsa::Alpha,
                format!("{other:?}"),
            ))
        }
    })
}

fn encode_parisc(op: &Op) -> Result<Vec<u8>, EncodeError> {
    // HP PA-RISC: 32-bit BE words; one branch delay slot.
    let enc = |w: u32| w.to_be_bytes().to_vec();
    Ok(match op {
        Op::Nop => enc(0x08000240),          // or %r0, %r0, %r0
        Op::Ret => {
            let mut v = enc(0xE840C000);     // bv %r0(%rp) — return via rp (r2)
            v.extend(enc(0x08000240));       // delay-slot nop
            v
        }
        other => {
            return Err(EncodeError::Unsupported(
                TargetIsa::PaRisc,
                format!("{other:?}"),
            ))
        }
    })
}

fn encode_m88k(_op: &Op) -> Result<Vec<u8>, EncodeError> {
    // Motorola 88000 encoder pending — semantic catalog only (emit text works).
    Err(EncodeError::Unsupported(
        TargetIsa::M88k,
        "encoder pending — semantic catalog only".into(),
    ))
}

fn encode_ia64(_op: &Op) -> Result<Vec<u8>, EncodeError> {
    // IA-64 needs EPIC bundle construction (3×41-bit slots + template) — pending.
    Err(EncodeError::Unsupported(
        TargetIsa::Ia64,
        "EPIC bundle encoder pending — semantic catalog only".into(),
    ))
}

fn encode_i860(_op: &Op) -> Result<Vec<u8>, EncodeError> {
    // Intel i860 encoder pending — semantic catalog only (emit text works).
    Err(EncodeError::Unsupported(
        TargetIsa::I860,
        "encoder pending — semantic catalog only".into(),
    ))
}

fn encode_coldfire(op: &Op) -> Result<Vec<u8>, EncodeError> {
    // ColdFire (68k-derived): big-endian, variable length. D0–D7 ↔ SIR VReg.
    // Dn sits in ea bits 5-3 for the fixed-opcode group (addi/subi/clr/addq/subq/push).
    let d = |v: VReg| v.0.min(7) as u16;
    let be = |w: u16| w.to_be_bytes().to_vec();
    Ok(match op {
        Op::Nop => be(0x4E71),
        Op::Ret => be(0x4E75), // rts
        Op::Clear { dst } => be(0x4280 | (d(*dst) << 3)), // clr.l Dn
        Op::MovImm { dst, imm } => {
            let dn = d(*dst);
            if (*imm as i32) >= -128 && (*imm as i32) <= 127 {
                be(0x7000 | (dn << 9) | (*imm as u8 as u16)) // moveq #imm, Dn
            } else {
                let mut v = be(0x203C | (dn << 6)); // move.l #imm, Dn
                v.extend_from_slice(&imm.to_be_bytes());
                v
            }
        }
        Op::AddImm { dst, imm } => {
            let dn = d(*dst);
            let mut v = be(0x0680 | (dn << 3)); // addi.l #imm, Dn
            v.extend_from_slice(&imm.to_be_bytes());
            v
        }
        Op::SubImm { dst, imm } => {
            let dn = d(*dst);
            let mut v = be(0x0480 | (dn << 3)); // subi.l #imm, Dn
            v.extend_from_slice(&imm.to_be_bytes());
            v
        }
        Op::Inc { dst } => be(0x5280 | (d(*dst) << 3)),  // addq.l #1, Dn
        Op::Dec { dst } => be(0x5380 | (d(*dst) << 3)),  // subq.l #1, Dn
        Op::Push { src } => be(0x2F00 | d(*src)), // move.l Dn, -(A7) — Dn in bits 2-0
        Op::Pop { dst } => be(0x201F | (d(*dst) << 9)), // move.l (A7)+, Dn — Dn in bits 11-9
        Op::LdMem { dst, base, offset, width } if *offset == 0 && *width == 4 => {
            // move.l (An), Dn — An in bits 2-0, Dn in bits 11-9 (capstone-verified).
            let an = base.0.min(7) as u16;
            let dn = d(*dst);
            be(0x2010 | (dn << 9) | an)
        }
        Op::StMem { src, base, offset, width } if *offset == 0 && *width == 4 => {
            // move.l Dn, (An) — An in bits 11-9, Dn in bits 2-0 (MOVE Dn→mem form,
            // capstone-verified: 0x2280 = move.l d0,(a1)).
            let an = base.0.min(7) as u16;
            let dn = d(*src);
            be(0x2080 | (an << 9) | dn)
        }
        other => {
            return Err(EncodeError::Unsupported(
                TargetIsa::ColdFire,
                format!("{other:?}"),
            ))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lift::lift_x86_32;

    fn module_from_ops(ops: Vec<Op>) -> Module {
        use crate::sir::{BasicBlock, Function};
        Module {
            name: "t".into(),
            source_isa: "sir".into(),
            functions: vec![Function {
                name: "t".into(),
                blocks: vec![BasicBlock {
                    label: "b0".into(),
                    ops,
                }],
            }],
            lift_gaps: 0,
            source: None,
            text_vma: None,
        }
    }

    #[test]
    fn encode_x86_nop_ret() {
        let m = lift_x86_32(&[0x90, 0xC3], "f").unwrap();
        let b = encode_module(&m, TargetIsa::X86_64).unwrap();
        assert_eq!(b, vec![0x90, 0xC3]);
    }

    #[test]
    fn encode_sh2_add3() {
        let bytes = [
            0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3,
        ];
        let m = lift_x86_32(&bytes, "add3").unwrap();
        let b = encode_module(&m, TargetIsa::SuperH(SuperHFlavor::Sh2)).unwrap();
        // mov #1,r0 ; add #2,r0 ; rts ; nop
        assert_eq!(b, vec![0x01, 0xE0, 0x02, 0x70, 0x0B, 0x00, 0x09, 0x00]);
    }

    #[test]
    fn encode_mips_nop_ret() {
        let m = lift_x86_32(&[0x90, 0xC3], "f").unwrap();
        let b = encode_module(&m, TargetIsa::Mips).unwrap();
        assert!(b.len() >= 4);
    }

    #[test]
    fn encode_alpha_known_bytes() {
        let m = lift_x86_32(&[0x90, 0xC3], "f").unwrap();
        // lda r31, 0(r31) ; ret r31, (r26)
        assert_eq!(encode_module(&m, TargetIsa::Alpha).unwrap(), [
            0x00, 0x00, 0xFF, 0x23, 0x01, 0x80, 0xFA, 0x6B,
        ]);
    }

    #[test]
    fn encode_alpha_add3() {
        let bytes = [0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3];
        let m = lift_x86_32(&bytes, "add3").unwrap();
        let b = encode_module(&m, TargetIsa::Alpha).unwrap();
        // mov $1 -> lda r0, 1(r31) ; add $2 -> lda r0, 2(r0) ; ret
        assert_eq!(
            b,
            [0x01, 0x00, 0x1F, 0x20, 0x02, 0x00, 0x00, 0x20, 0x01, 0x80, 0xFA, 0x6B]
        );
    }

    #[test]
    fn encode_parisc_nop_ret() {
        let m = lift_x86_32(&[0x90, 0xC3], "f").unwrap();
        // or %r0,%r0,%r0 ; bv %r0(%rp) ; nop
        assert_eq!(
            encode_module(&m, TargetIsa::PaRisc).unwrap(),
            [0x08, 0x00, 0x02, 0x40, 0xE8, 0x40, 0xC0, 0x00, 0x08, 0x00, 0x02, 0x40]
        );
    }

    #[test]
    fn encode_coldfire_add3() {
        let bytes = [0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3];
        let m = lift_x86_32(&bytes, "add3").unwrap();
        let b = encode_module(&m, TargetIsa::ColdFire).unwrap();
        // moveq #1,d0 ; addi.l #2,d0 ; rts
        assert_eq!(b, [0x70, 0x01, 0x06, 0x80, 0x00, 0x00, 0x00, 0x02, 0x4E, 0x75]);
    }

    #[test]
    fn encode_coldfire_nonzero_regs() {
        // Regression (found by differential test): Dn sits in ea bits 5-3 for the
        // fixed-opcode group — `clr.l d3` must be 0x4298, not 0x4283 (clr.l (a3)+).
        let ops = vec![
            Op::Clear { dst: VReg(3) },
            Op::AddImm { dst: VReg(3), imm: 5 },
            Op::Inc { dst: VReg(3) },
            Op::Dec { dst: VReg(3) },
            Op::Push { src: VReg(3) },
            Op::Pop { dst: VReg(3) },
        ];
        for (op, want) in [
            (&ops[0], vec![0x42, 0x98]),
            (&ops[1], vec![0x06, 0x98, 0x00, 0x00, 0x00, 0x05]),
            (&ops[2], vec![0x52, 0x98]),
            (&ops[3], vec![0x53, 0x98]),
            (&ops[4], vec![0x2F, 0x03]), // move.l d3, -(a7)
            (&ops[5], vec![0x26, 0x1F]), // move.l (a7)+, d3
        ] {
            let m = module_from_ops(vec![op.clone()]);
            assert_eq!(encode_module(&m, TargetIsa::ColdFire).unwrap(), *want, "{op:?}");
        }
    }

    #[test]
    fn encode_coldfire_push_pop_reg_field_capstone_verified() {
        // Regression: the old push/pop placed Dn wrong (0x29C0 = move.l d0,(a4)+ —
        // only D0/VReg0 ever passed, so roundtrip was consistent-but-wrong).
        // Capstone-verified: 0x2F01 = move.l d1,-(a7); 0x221F = move.l (a7)+,d1.
        let ops = vec![Op::Push { src: VReg(1) }, Op::Pop { dst: VReg(1) }];
        let m = module_from_ops(ops.clone());
        let bytes = encode_module(&m, TargetIsa::ColdFire).unwrap();
        assert_eq!(bytes, vec![0x2F, 0x01, 0x22, 0x1F]);
        let ops2 = vec![
            Op::LdMem { dst: VReg(3), base: VReg(3), offset: 0, width: 4 },
            Op::StMem { src: VReg(3), base: VReg(3), offset: 0, width: 4 },
        ];
        let m2 = module_from_ops(ops2.clone());
        let b2 = encode_module(&m2, TargetIsa::ColdFire).unwrap();
        assert_eq!(b2, vec![0x26, 0x13, 0x26, 0x83]); // move.l (a3),d3 / move.l d3,(a3)
    }

    #[test]
    fn encode_m88k_ia64_i860_pending() {
        let m = lift_x86_32(&[0x90, 0xC3], "f").unwrap();
        for t in [TargetIsa::M88k, TargetIsa::Ia64, TargetIsa::I860] {
            assert!(
                matches!(encode_module(&m, t), Err(EncodeError::Unsupported(isa, _)) if isa == t),
                "expected Unsupported for {t}"
            );
        }
    }
}
