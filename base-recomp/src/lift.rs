//! x86-32 static lifter (Path v1.7 subset + v1.8 wider decode).

use crate::sir::{BasicBlock, Function, Module, Op, VReg};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LiftError {
    #[error("empty input")]
    Empty,
}

/// GP encoding: eax=0 … edi=7.
fn gp(r: u8) -> VReg {
    VReg(u32::from(r & 7))
}

/// Lift a flat x86-32 instruction stream into one function (linear block).
///
/// Supported (v1.7 + v1.8):
/// - `90` nop · `C3` ret
/// - `B8+r iv` mov r32, imm32
/// - `05 iv` add eax, imm32 · `2D iv` sub eax, imm32
/// - `83 /0 ib` add r32, imm8 · `83 /5 ib` sub r32, imm8
/// - `31 /r` / `33 /r` xor r32,r32 when src==dst → Clear
/// - `40+r` inc · `48+r` dec
/// - `50+r` push · `58+r` pop
/// - `E8 cd` call rel32 · `E9 cd` jmp rel32 · `EB cb` jmp rel8
pub fn lift_x86_32(bytes: &[u8], name: &str) -> Result<Module, LiftError> {
    lift_x86_32_at(bytes, name, 0)
}

/// Like [`lift_x86_32`], with base VMA for resolving relative call/jmp targets.
pub fn lift_x86_32_at(bytes: &[u8], name: &str, base_vma: u64) -> Result<Module, LiftError> {
    if bytes.is_empty() {
        return Err(LiftError::Empty);
    }

    let mut ops = Vec::new();
    let mut i = 0usize;
    let mut gaps = 0usize;

    while i < bytes.len() {
        let before = i;
        match decode_one(bytes, i, base_vma) {
            Some((op, len)) => {
                if matches!(op, Op::Unknown { .. }) {
                    gaps += 1;
                }
                ops.push(op);
                i += len;
            }
            None => {
                gaps += 1;
                ops.push(Op::Unknown {
                    offset: i as u64,
                    bytes: vec![bytes[i]],
                    note: format!("unsupported opcode 0x{:02x}", bytes[i]),
                });
                i += 1;
            }
        }
        if i == before {
            i += 1;
        }
    }

    Ok(Module {
        name: name.to_string(),
        source_isa: "x86_32".into(),
        functions: vec![Function {
            name: name.to_string(),
            blocks: vec![BasicBlock {
                label: "entry".into(),
                ops,
            }],
        }],
        lift_gaps: gaps,
        source: None,
        text_vma: if base_vma != 0 {
            Some(base_vma)
        } else {
            None
        },
    })
}

fn decode_one(bytes: &[u8], i: usize, base_vma: u64) -> Option<(Op, usize)> {
    let b0 = *bytes.get(i)?;
    match b0 {
        0x90 => Some((Op::Nop, 1)),
        0xC3 => Some((Op::Ret, 1)),
        0x05 if i + 5 <= bytes.len() => {
            let imm = u32::from_le_bytes(bytes[i + 1..i + 5].try_into().ok()?);
            Some((Op::AddImm { dst: gp(0), imm }, 5))
        }
        0x2D if i + 5 <= bytes.len() => {
            let imm = u32::from_le_bytes(bytes[i + 1..i + 5].try_into().ok()?);
            Some((Op::SubImm { dst: gp(0), imm }, 5))
        }
        0xB8..=0xBF if i + 5 <= bytes.len() => {
            let imm = u32::from_le_bytes(bytes[i + 1..i + 5].try_into().ok()?);
            Some((Op::MovImm { dst: gp(b0 - 0xB8), imm }, 5))
        }
        0x40..=0x47 => Some((Op::Inc { dst: gp(b0 - 0x40) }, 1)),
        0x48..=0x4F => Some((Op::Dec { dst: gp(b0 - 0x48) }, 1)),
        0x50..=0x57 => Some((Op::Push { src: gp(b0 - 0x50) }, 1)),
        0x58..=0x5F => Some((Op::Pop { dst: gp(b0 - 0x58) }, 1)),
        0x31 | 0x33 if i + 2 <= bytes.len() => {
            let modrm = bytes[i + 1];
            let mod_ = modrm >> 6;
            let reg = (modrm >> 3) & 7;
            let rm = modrm & 7;
            if mod_ == 0b11 && reg == rm {
                Some((Op::Clear { dst: gp(rm) }, 2))
            } else {
                Some((
                    Op::Unknown {
                        offset: i as u64,
                        bytes: bytes[i..i + 2].to_vec(),
                        note: format!("xor form not clear-self (modrm={modrm:#x})"),
                    },
                    2,
                ))
            }
        }
        0x83 if i + 3 <= bytes.len() => {
            let modrm = bytes[i + 1];
            let imm = bytes[i + 2] as i8 as i32 as u32; // sign-extend to 32
            let mod_ = modrm >> 6;
            let reg = (modrm >> 3) & 7;
            let rm = modrm & 7;
            if mod_ != 0b11 {
                return Some((
                    Op::Unknown {
                        offset: i as u64,
                        bytes: bytes[i..i + 3].to_vec(),
                        note: "83 /r memory form unsupported".into(),
                    },
                    3,
                ));
            }
            match reg {
                0 => Some((Op::AddImm { dst: gp(rm), imm }, 3)),
                5 => Some((Op::SubImm { dst: gp(rm), imm }, 3)),
                _ => Some((
                    Op::Unknown {
                        offset: i as u64,
                        bytes: bytes[i..i + 3].to_vec(),
                        note: format!("83 /{reg} unsupported"),
                    },
                    3,
                )),
            }
        }
        0xE8 if i + 5 <= bytes.len() => {
            let rel = i32::from_le_bytes(bytes[i + 1..i + 5].try_into().ok()?);
            let next = base_vma + (i as u64) + 5;
            let target = next.wrapping_add(rel as i64 as u64);
            Some((
                Op::CallRel {
                    rel,
                    target: Some(target),
                },
                5,
            ))
        }
        0xE9 if i + 5 <= bytes.len() => {
            let rel = i32::from_le_bytes(bytes[i + 1..i + 5].try_into().ok()?);
            let next = base_vma + (i as u64) + 5;
            let target = next.wrapping_add(rel as i64 as u64);
            Some((
                Op::JmpRel {
                    rel,
                    target: Some(target),
                },
                5,
            ))
        }
        0xEB if i + 2 <= bytes.len() => {
            let rel8 = bytes[i + 1] as i8 as i32;
            let next = base_vma + (i as u64) + 2;
            let target = next.wrapping_add(rel8 as i64 as u64);
            Some((
                Op::JmpRel {
                    rel: rel8,
                    target: Some(target),
                },
                2,
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lift_nop_ret() {
        let m = lift_x86_32(&[0x90, 0xC3], "f").unwrap();
        assert_eq!(m.lift_gaps, 0);
    }

    #[test]
    fn lift_xor_clear_ret() {
        let m = lift_x86_32(&[0x31, 0xC0, 0xC3], "z").unwrap();
        assert_eq!(m.lift_gaps, 0);
        assert!(matches!(
            m.functions[0].blocks[0].ops[0],
            Op::Clear { dst: VReg(0) }
        ));
    }

    #[test]
    fn lift_mov_ecx_imm() {
        let bytes = [0xB9, 0x07, 0x00, 0x00, 0x00, 0xC3]; // mov ecx,7; ret
        let m = lift_x86_32(&bytes, "c").unwrap();
        assert!(matches!(
            m.functions[0].blocks[0].ops[0],
            Op::MovImm {
                dst: VReg(1),
                imm: 7
            }
        ));
    }

    #[test]
    fn lift_sub_push_pop() {
        // sub eax, 1 ; push eax ; pop ecx ; ret
        let bytes = [
            0x2D, 0x01, 0x00, 0x00, 0x00, 0x50, 0x59, 0xC3,
        ];
        let m = lift_x86_32(&bytes, "s").unwrap();
        assert_eq!(m.lift_gaps, 0);
        assert!(matches!(
            m.functions[0].blocks[0].ops[0],
            Op::SubImm { imm: 1, .. }
        ));
    }

    #[test]
    fn lift_call_rel() {
        // e8 00 00 00 00 = call +0 (next insn)
        let m = lift_x86_32_at(&[0xE8, 0x00, 0x00, 0x00, 0x00, 0xC3], "c", 0x1000).unwrap();
        assert!(matches!(
            m.functions[0].blocks[0].ops[0],
            Op::CallRel {
                rel: 0,
                target: Some(0x1005)
            }
        ));
    }
}
