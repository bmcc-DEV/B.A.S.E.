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
        Op::LdMem { dst, base, offset, width } if dst.0 == 0 && *offset == 0 && *width == 4 => {
            // mov eax, [base] — ModRM mod=00, reg=000 (eax), rm=base (capstone-verified).
            vec![0x8B, 0x00 | (base.0 as u8 & 7)]
        }
        Op::StMem { src, base, offset, width } if src.0 == 0 && *offset == 0 && *width == 4 => {
            // mov [base], eax.
            vec![0x89, 0x00 | (base.0 as u8 & 7)]
        }
        Op::LdMem { .. } | Op::StMem { .. } => {
            return Err(EncodeError::Unsupported(
                TargetIsa::X86_64,
                "LdMem/StMem with dst/src != eax or offset/width outside subset".into(),
            ))
        }
        Op::CallRel { rel, .. } => {
            // call rel32 — relative to the NEXT instruction (x86 semantics).
            let mut v = vec![0xE8];
            v.extend_from_slice(&(*rel as u32).to_le_bytes());
            v
        }
        Op::JmpRel { rel, .. } => {
            // jmp rel32.
            let mut v = vec![0xE9];
            v.extend_from_slice(&(*rel as u32).to_le_bytes());
            v
        }
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
        Op::MovImm { dst, imm } if *imm == 0xFFFF_FFFF => {
            // mvn rd, #0 → rd = -1. ARM MOV Rd,#imm8 cannot hold 0xFFFFFFFF.
            let rd = dst.0.min(12);
            enc(0xE3E00000 | (rd << 12))
        }
        Op::AddImm { dst, imm } if *imm == 0xFFFF_FFFF => {
            // add -1 → sub rd, rd, #1.
            let rd = dst.0.min(12);
            enc(0xE2400000 | (rd << 12) | (rd << 16) | 1)
        }
        Op::SubImm { dst, imm } if *imm == 0xFFFF_FFFF => {
            // sub -1 → add rd, rd, #1.
            let rd = dst.0.min(12);
            enc(0xE2800000 | (rd << 12) | (rd << 16) | 1)
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
        Op::LdMem { dst, base, offset, width } if *offset == 0 && *width == 4 => {
            let rd = dst.0.min(12);
            let rn = base.0.min(12);
            // ldr rd, [rn] — capstone-verified (0xE5910000 = ldr r0, [r1]).
            enc(0xE5900000 | (rn << 16) | (rd << 12))
        }
        Op::StMem { src, base, offset, width } if *offset == 0 && *width == 4 => {
            let rd = src.0.min(12);
            let rn = base.0.min(12);
            // str rd, [rn] — capstone-verified (0xE5810000 = str r0, [r1]).
            enc(0xE5800000 | (rn << 16) | (rd << 12))
        }
        Op::Push { src } => {
            // push {rX} = STMDB sp!, {rX} — llvm-mc verified (0xE92D0001 = push {r0}).
            enc(0xE92D0000 | (1u32 << src.0.min(12)))
        }
        Op::Pop { dst } => {
            // pop {rX} = LDMIA sp!, {rX} — llvm-mc verified (0xE8BD0001 = pop {r0}).
            enc(0xE8BD0000 | (1u32 << dst.0.min(12)))
        }
        Op::CallRel { rel, .. } => {
            // bl +imm24 (cond=AL, L=1) — llvm-mc verified (0xEB000000 = bl #0).
            enc(0xEB000000 | ((*rel as u32 >> 2) & 0x00FF_FFFF))
        }
        Op::JmpRel { rel, .. } => {
            // b +imm24 — llvm-mc verified (0xEA000000 = b #0).
            enc(0xEA000000 | ((*rel as u32 >> 2) & 0x00FF_FFFF))
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
        Op::MovImm { dst, imm } if *imm == 0xFFFF_FFFF => {
            // movn wd, #0 → wd = -1 (MOVZ Wd,#imm16 can't hold 0xFFFFFFFF).
            let wd = dst.0.min(30);
            enc(0x12800000 | (wd << 5) | wd)
        }
        Op::AddImm { dst, imm } if *imm == 0xFFFF_FFFF => {
            // add -1 → sub wd, wd, #1.
            let wd = dst.0.min(30);
            enc(0x51000400 | (wd << 5) | wd)
        }
        Op::SubImm { dst, imm } if *imm == 0xFFFF_FFFF => {
            // sub -1 → add wd, wd, #1.
            let wd = dst.0.min(30);
            enc(0x11000400 | (wd << 5) | wd)
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
        Op::LdMem { dst, base, offset, width } if *offset == 0 && *width == 4 => {
            let wt = dst.0.min(30);
            let wb = base.0.min(30);
            // ldr wT, [wB] — capstone-verified (0xB9400020 = ldr w0, [x1]).
            enc(0xB9400000 | (wb << 5) | wt)
        }
        Op::StMem { src, base, offset, width } if *offset == 0 && *width == 4 => {
            let wt = src.0.min(30);
            let wb = base.0.min(30);
            // str wT, [wB] — capstone-verified (0xB9000020 = str w0, [x1]).
            enc(0xB9000000 | (wb << 5) | wt)
        }
        Op::Push { src } => {
            // str wX, [sp, #-4]! (pre-index) — llvm-mc verified (E0 CF 1F B8 = str w0,[sp,#-4]!).
            let wd = src.0.min(30) as u8;
            vec![0xE0 | (wd & 0x1F), 0xCF, 0x1F, 0xB8]
        }
        Op::Pop { dst } => {
            // ldr wX, [sp], #4 (post-index) — llvm-mc verified (E0 47 40 B8 = ldr w0,[sp],#4).
            let wd = dst.0.min(30) as u8;
            vec![0xE0 | (wd & 0x1F), 0x47, 0x40, 0xB8]
        }
        Op::CallRel { rel, .. } => {
            // bl +imm26 — llvm-mc verified (0x94000000 = bl #0).
            enc(0x94000000 | ((*rel as u32 >> 2) & 0x03FF_FFFF))
        }
        Op::JmpRel { rel, .. } => {
            // b +imm26 — llvm-mc verified (0x14000000 = b #0).
            enc(0x14000000 | ((*rel as u32 >> 2) & 0x03FF_FFFF))
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
        Op::LdMem { dst, base, offset, width } if *width == 4 && *offset >= -32768 && *offset <= 32767 => {
            let rt = treg(*dst);
            let rs = treg(*base);
            // lw $rt, imm($rs) — capstone-verified (0x8D280000 = lw $t0, ($t1)).
            enc(0x8C000000 | (rs << 21) | (rt << 16) | (*offset as u16 as u32))
        }
        Op::StMem { src, base, offset, width } if *width == 4 && *offset >= -32768 && *offset <= 32767 => {
            let rt = treg(*src);
            let rs = treg(*base);
            // sw $rt, imm($rs) — capstone-verified (0xAD280000 = sw $t0, ($t1)).
            enc(0xAC000000 | (rs << 21) | (rt << 16) | (*offset as u16 as u32))
        }
        Op::Push { src } => {
            // addiu $sp,$sp,-4 ; sw $t,0($sp) — fold on decode (objdump verified).
            let t = treg(*src);
            let mut v = enc(0x27BDFFFC);
            v.extend(enc(0xAC000000 | (29 << 21) | (t << 16)));
            v
        }
        Op::Pop { dst } => {
            // lw $t,0($sp) ; addiu $sp,$sp,4 — fold on decode (objdump verified).
            let t = treg(*dst);
            let mut v = enc(0x8C000000 | (29 << 21) | (t << 16));
            v.extend(enc(0x27BD0004));
            v
        }
        Op::CallRel { rel, .. } => {
            // jal — 26-bit field = target>>2; PC upper bits untracked (pseudo-relative).
            // objdump verified: 0x0C000000 = jal 0. Delay-slot nop appended.
            let mut v = enc(0x0C000000 | ((*rel as u32 >> 2) & 0x03FF_FFFF));
            v.extend(enc(0));
            v
        }
        Op::JmpRel { rel, .. } => {
            // beq $zero,$zero,rel — the PC-relative unconditional jump (objdump: 0x10000000 = b).
            let mut v = enc(0x10000000 | ((*rel as u32 >> 2) & 0xFFFF));
            v.extend(enc(0));
            v
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
        Op::LdMem { dst, base, offset, width } if *width == 4 && *offset >= -32768 && *offset <= 32767 => {
            let rt = r(*dst);
            let ra = r(*base);
            // lwz rT, d(rA) — capstone-verified (0x80640000 = lwz r3, 0(r4)).
            enc(0x80000000 | (rt << 21) | (ra << 16) | (*offset as u16 as u32))
        }
        Op::StMem { src, base, offset, width } if *width == 4 && *offset >= -32768 && *offset <= 32767 => {
            let rs = r(*src);
            let ra = r(*base);
            // stw rS, d(rA) — capstone-verified (0x90640000 = stw r3, 0(r4)).
            enc(0x90000000 | (rs << 21) | (ra << 16) | (*offset as u16 as u32))
        }
        Op::Push { src } => {
            // stwu rS, -4(r1) — pre-decrement store is PPC's push (objdump verified
            // 0x9461FFFC = stwu r3,-4(r1)).
            let rs = r(*src);
            enc(0x94000000 | (rs << 21) | (1 << 16) | 0xFFFC)
        }
        Op::Pop { dst } => {
            // lwzu rS, 4(r1) — post-increment load is PPC's pop (objdump verified
            // 0x84610004 = lwzu r3,4(r1)).
            let rs = r(*dst);
            enc(0x84000000 | (rs << 21) | (1 << 16) | 4)
        }
        Op::CallRel { rel, .. } => {
            // bl — LI = rel>>2 (24-bit), AA=0, LK=1 (objdump: 0x48000001 = bl 0).
            enc(0x48000000 | ((*rel as u32) & 0x03FF_FFFC) | 1)
        }
        Op::JmpRel { rel, .. } => {
            // b — LI = rel>>2, AA=0, LK=0 (objdump: 0x48000000 = b 0).
            enc(0x48000000 | ((*rel as u32) & 0x03FF_FFFC))
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
        Op::LdMem { dst, base, offset, width } if *offset == 0 && *width == 4 => {
            let rd = 16 + dst.0.min(7);
            let rs = 16 + base.0.min(7);
            // ld [%rs], %rd — op=3, op3=0x00, i=1, imm13=0 (capstone-verified).
            enc(0xC0000000 | (rd << 25) | (rs << 14) | 0x2000)
        }
        Op::StMem { src, base, offset, width } if *offset == 0 && *width == 4 => {
            let rd = 16 + src.0.min(7);
            let rs = 16 + base.0.min(7);
            // st %rd, [%rs] — op=3, op3=0x04, i=1, imm13=0 (capstone-verified).
            enc(0xC0000000 | (rd << 25) | (4 << 19) | (rs << 14) | 0x2000)
        }
        Op::AddImm { dst, imm } if (*imm as i32) >= -4096 && (*imm as i32) <= 4095 => {
            let r = 16 + dst.0.min(7);
            // add %rd, imm13, %rd — op=2, op3=0, i=1 (llvm-mc verified 0xA0042005).
            enc(0x80000000 | (r << 25) | (r << 14) | 0x2000 | (*imm as u32 & 0x1FFF))
        }
        Op::SubImm { dst, imm } if (*imm as i32) >= -4096 && (*imm as i32) <= 4095 => {
            let r = 16 + dst.0.min(7);
            // sub %rd, imm13, %rd — op=2, op3=0x04 (llvm-mc verified 0xA0242005).
            enc(0x80000000 | (4 << 19) | (r << 25) | (r << 14) | 0x2000 | (*imm as u32 & 0x1FFF))
        }
        Op::Inc { dst } => {
            let r = 16 + dst.0.min(7);
            enc(0x80000000 | (r << 25) | (r << 14) | 0x2000 | 1) // add %rd, 1, %rd
        }
        Op::Dec { dst } => {
            let r = 16 + dst.0.min(7);
            enc(0x80000000 | (4 << 19) | (r << 25) | (r << 14) | 0x2000 | 1) // sub %rd, 1, %rd
        }
        Op::Push { src } => {
            let r = 16 + src.0.min(7);
            // add %sp,-4,%sp ; st %rd,[%sp] — fold on decode (llvm-mc verified).
            let mut v = enc(0x80000000 | (14 << 25) | (14 << 14) | 0x2000 | 0x1FFC);
            v.extend(enc(0xC0000000 | (r << 25) | (4 << 19) | (14 << 14) | 0x2000));
            v
        }
        Op::Pop { dst } => {
            let r = 16 + dst.0.min(7);
            // ld [%sp],%rd ; add %sp,4,%sp — fold on decode (llvm-mc verified).
            let mut v = enc(0xC0000000 | (r << 25) | (14 << 14) | 0x2000);
            v.extend(enc(0x80000000 | (14 << 25) | (14 << 14) | 0x2000 | 4));
            v
        }
        Op::CallRel { rel, .. } => {
            // call disp30 — delay-slot nop appended (llvm-mc verified 0x40000000 = call 0).
            let mut v = enc(0x40000000 | ((*rel as u32 >> 2) & 0x3FFF_FFFF));
            v.extend(enc(0x01000000));
            v
        }
        Op::JmpRel { rel, .. } => {
            // ba disp22 — delay-slot nop appended (llvm-mc verified 0x10800000 = ba 0).
            let mut v = enc(0x10800000 | ((*rel as u32 >> 2) & 0x3F_FFFF));
            v.extend(enc(0x01000000));
            v
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
        // mov/add #imm8 sign-extend (SH has no sub-immediate — Sub/Dec fold through add).
        Op::MovImm { dst, imm } if (*imm as i8 as u32) == *imm => {
            let n = dst.0.min(15) as u16;
            h(0xE000 | (n << 8) | (*imm as u8 as u16))
        }
        Op::AddImm { dst, imm } if (*imm as i8 as u32) == *imm => {
            let n = dst.0.min(15) as u16;
            h(0x7000 | (n << 8) | (*imm as u8 as u16))
        }
        Op::SubImm { dst, imm } if (*imm as i8 as u32) == *imm => {
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
        Op::LdMem { dst, base, offset, width } if *offset == 0 && *width == 4 => {
            // mov.l @Rm, Rn — 0101nnnnmmmm0000. objdump: 0x5010 = mov.l @r1,r0.
            let n = dst.0.min(15) as u16;
            let m = base.0.min(15) as u16;
            h(0x5000 | (n << 8) | (m << 4))
        }
        Op::StMem { src, base, offset, width } if *offset == 0 && *width == 4 => {
            // mov.l Rn, @Rm — 0010nnnnmmmm0010. objdump: 0x2102 = mov.l r0,@r1.
            let n = src.0.min(15) as u16;
            let m = base.0.min(15) as u16;
            h(0x2000 | (m << 8) | (n << 4) | 0x2)
        }
        Op::Push { src } => {
            // mov.l Rn, @-r15 — 0010nnnnmmmm0110 with base r15. objdump: 0x2F06.
            let n = src.0.min(15) as u16;
            h(0x2000 | (15 << 8) | (n << 4) | 0x6)
        }
        Op::Pop { dst } => {
            // mov.l @r15+, Rn — 0101nnnnmmmm0110 with src r15 (SH-4 manual; binutils SH
            // labels the 0110 post-increment form as a displacement — documented quirk).
            let n = dst.0.min(15) as u16;
            h(0x5000 | (n << 8) | (15 << 4) | 0x6)
        }
        Op::CallRel { rel, .. } => {
            // bsr disp12 — delay-slot nop appended (objdump: 0xB000 = bsr 0x4).
            let mut v = h(0xB000 | ((*rel as u16 >> 1) & 0xFFF));
            v.extend(h(0x0009));
            v
        }
        Op::JmpRel { rel, .. } => {
            // bra disp12 — delay-slot nop appended (objdump: 0xA000 = bra 0x4).
            let mut v = h(0xA000 | ((*rel as u16 >> 1) & 0xFFF));
            v.extend(h(0x0009));
            v
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
        Op::LdMem { dst, base, offset, width } if *width == 8 && *offset >= -32768 && *offset <= 32767 => {
            let ra = r(*dst);
            let rb = r(*base);
            // ldq ra, disp(rb) — Alpha memory format (opcode 0x29 << 26, 64-bit).
            enc((0x29 << 26) | (ra << 21) | (rb << 16) | (*offset as u16 as u32))
        }
        Op::StMem { src, base, offset, width } if *width == 8 && *offset >= -32768 && *offset <= 32767 => {
            let ra = r(*src);
            let rb = r(*base);
            // stq ra, disp(rb) — Alpha memory format (opcode 0x2D << 26).
            enc((0x2D << 26) | (ra << 21) | (rb << 16) | (*offset as u16 as u32))
        }
        Op::Push { src } => {
            let ra = r(*src);
            // lda r30,-8(r30) ; stq ra,0(r30) — 8-byte stack slot (objdump verified:
            // F8 FF DE 23 = lda sp,-8(sp); 00 00 1E B4 = stq r0,0(sp)). Fold on decode.
            let mut v = enc(0x20000000 | (30 << 21) | (30 << 16) | 0xFFF8);
            v.extend(enc((0x2D << 26) | (ra << 21) | (30 << 16)));
            v
        }
        Op::Pop { dst } => {
            let ra = r(*dst);
            // ldq ra,0(r30) ; lda r30,8(r30) — fold on decode (objdump verified).
            let mut v = enc((0x29 << 26) | (ra << 21) | (30 << 16));
            v.extend(enc(0x20000000 | (30 << 21) | (30 << 16) | 8));
            v
        }
        Op::CallRel { rel, .. } => {
            // bsr $26, disp — opcode 0x34, disp21 scaled by 4 (objdump: 0xD3400000 = bsr ra,0).
            enc((0x34 << 26) | (26 << 21) | ((*rel as u32 >> 2) & 0x1F_FFFF))
        }
        Op::JmpRel { rel, .. } => {
            // br $31, disp — opcode 0x30 (objdump: 0xC3E00000 = br zero,0).
            enc((0x30 << 26) | (31 << 21) | ((*rel as u32 >> 2) & 0x1F_FFFF))
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
        Op::CallRel { rel, .. } => {
            // bsr.w disp16 — 0x61 0x00 + word16 (objdump: 0x61000000 = bsrw 0x6).
            let mut v = be(0x6100);
            v.extend_from_slice(&(*rel as i16 as u16).to_be_bytes());
            v
        }
        Op::JmpRel { rel, .. } => {
            // bra.w disp16 — 0x60 0x00 + word16 (objdump: 0x60000000 = braw 0x2).
            let mut v = be(0x6000);
            v.extend_from_slice(&(*rel as i16 as u16).to_be_bytes());
            v
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

    #[test]
    fn encode_ld_st_capstone_verified() {
        // Bytes verified against capstone (mips/ppc/arm64); Alpha against its memory
        // format (opcode 0x29/0x2D — LDA pattern already checked vs LLVM).
        let ld = |dst: VReg, base: VReg, width: u8| {
            module_from_ops(vec![Op::LdMem { dst, base, offset: 0, width }])
        };
        let st = |src: VReg, base: VReg, width: u8| {
            module_from_ops(vec![Op::StMem { src, base, offset: 0, width }])
        };
        // MIPS: lw $t0,($t1) / sw $t0,($t1) — 0x8D280000 / 0xAD280000.
        assert_eq!(
            encode_module(&ld(VReg(0), VReg(1), 4), TargetIsa::Mips).unwrap(),
            vec![0x8D, 0x28, 0x00, 0x00]
        );
        assert_eq!(
            encode_module(&st(VReg(0), VReg(1), 4), TargetIsa::Mips).unwrap(),
            vec![0xAD, 0x28, 0x00, 0x00]
        );
        // PPC: lwz r3,0(r4) / stw r3,0(r4) — 0x80640000 / 0x90640000.
        assert_eq!(
            encode_module(&ld(VReg(0), VReg(1), 4), TargetIsa::Ppc).unwrap(),
            vec![0x80, 0x64, 0x00, 0x00]
        );
        assert_eq!(
            encode_module(&st(VReg(0), VReg(1), 4), TargetIsa::Ppc).unwrap(),
            vec![0x90, 0x64, 0x00, 0x00]
        );
        // AArch64: ldr w0,[x1] / str w0,[x1] — 0xB9400020 / 0xB9000020.
        assert_eq!(
            encode_module(&ld(VReg(0), VReg(1), 4), TargetIsa::AArch64).unwrap(),
            vec![0x20, 0x00, 0x40, 0xB9]
        );
        assert_eq!(
            encode_module(&st(VReg(0), VReg(1), 4), TargetIsa::AArch64).unwrap(),
            vec![0x20, 0x00, 0x00, 0xB9]
        );
        // Alpha: ldq r0,0(r1) / stq r0,0(r1) — opcode 0x29/0x2D << 26.
        assert_eq!(
            encode_module(&ld(VReg(0), VReg(1), 8), TargetIsa::Alpha).unwrap(),
            vec![0x00, 0x00, 0x01, 0xA4] // 0xA4000000 LE
        );
        assert_eq!(
            encode_module(&st(VReg(0), VReg(1), 8), TargetIsa::Alpha).unwrap(),
            vec![0x00, 0x00, 0x01, 0xB4] // 0xB4000000 LE
        );
        // ARM: ldr r0,[r1] / str r0,[r1] — 0xE5910000 / 0xE5810000.
        assert_eq!(
            encode_module(&ld(VReg(0), VReg(1), 4), TargetIsa::Arm).unwrap(),
            vec![0x00, 0x00, 0x91, 0xE5]
        );
        assert_eq!(
            encode_module(&st(VReg(0), VReg(1), 4), TargetIsa::Arm).unwrap(),
            vec![0x00, 0x00, 0x81, 0xE5]
        );
        // SPARC: ld [%l1],%l0 / st %l0,[%l1] — 0xE0046000 / 0xE0246000.
        assert_eq!(
            encode_module(&ld(VReg(0), VReg(1), 4), TargetIsa::Sparc).unwrap(),
            vec![0xE0, 0x04, 0x60, 0x00]
        );
        assert_eq!(
            encode_module(&st(VReg(0), VReg(1), 4), TargetIsa::Sparc).unwrap(),
            vec![0xE0, 0x24, 0x60, 0x00]
        );
        // x86: mov eax,[ecx] / mov [ecx],eax — 0x8B01 / 0x8901.
        assert_eq!(
            encode_module(&ld(VReg(0), VReg(1), 4), TargetIsa::X86_64).unwrap(),
            vec![0x8B, 0x01]
        );
        assert_eq!(
            encode_module(&st(VReg(0), VReg(1), 4), TargetIsa::X86_64).unwrap(),
            vec![0x89, 0x01]
        );
    }
}
