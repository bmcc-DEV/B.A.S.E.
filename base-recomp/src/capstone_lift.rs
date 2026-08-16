//! Optional Capstone-assisted x86 decode (feature = "capstone").

use crate::lift::LiftError;
use crate::sir::Module;

#[cfg(feature = "capstone")]
mod imp {
    use super::*;
    use crate::sir::{BasicBlock, Function, Op, VReg};
    use capstone::prelude::*;

    pub fn lift_x86_capstone(bytes: &[u8], name: &str, base_vma: u64) -> Result<Module, LiftError> {
        if bytes.is_empty() {
            return Err(LiftError::Empty);
        }
        let cs = Capstone::new()
            .x86()
            .mode(arch::x86::ArchMode::Mode32)
            .detail(true)
            .build()
            .map_err(|_| LiftError::Empty)?;

        let insns = cs
            .disasm_all(bytes, base_vma)
            .map_err(|_| LiftError::Empty)?;

        let mut ops = Vec::new();
        let mut gaps = 0usize;
        for insn in insns.iter() {
            let offset = insn.address().saturating_sub(base_vma);
            if let Some(op) = map_insn(&insn) {
                ops.push(op);
            } else {
                gaps += 1;
                ops.push(Op::Unknown {
                    offset,
                    bytes: insn.bytes().to_vec(),
                    note: format!(
                        "capstone unmapped {} {}",
                        insn.mnemonic().unwrap_or("?"),
                        insn.op_str().unwrap_or("")
                    ),
                });
            }
        }

        Ok(Module {
            name: name.to_string(),
            source_isa: "x86_32+capstone".into(),
            functions: vec![Function {
                name: name.to_string(),
                blocks: vec![BasicBlock {
                    label: "entry".into(),
                    ops,
                }],
            }],
            lift_gaps: gaps,
            source: None,
            text_vma: if base_vma != 0 { Some(base_vma) } else { None },
        })
    }

    fn map_insn(insn: &capstone::Insn<'_>) -> Option<Op> {
        let m = insn.mnemonic()?.to_ascii_lowercase();
        let ops = insn.op_str().unwrap_or("").to_ascii_lowercase();
        match m.as_str() {
            "nop" => Some(Op::Nop),
            "ret" | "retn" => Some(Op::Ret),
            "xor" if ops.contains("eax, eax") || ops.contains("eax,eax") => {
                Some(Op::Clear { dst: VReg(0) })
            }
            "mov" if ops.starts_with("eax") => {
                Some(Op::MovImm {
                    dst: VReg(0),
                    imm: parse_imm(&ops)?,
                })
            }
            "add" if ops.starts_with("eax") => {
                Some(Op::AddImm {
                    dst: VReg(0),
                    imm: parse_imm(&ops)?,
                })
            }
            "sub" if ops.starts_with("eax") => {
                Some(Op::SubImm {
                    dst: VReg(0),
                    imm: parse_imm(&ops)?,
                })
            }
            "inc" if ops.contains("eax") => Some(Op::Inc { dst: VReg(0) }),
            "dec" if ops.contains("eax") => Some(Op::Dec { dst: VReg(0) }),
            "push" if ops.contains("eax") => Some(Op::Push { src: VReg(0) }),
            "pop" if ops.contains("eax") => Some(Op::Pop { dst: VReg(0) }),
            _ => None,
        }
    }

    fn parse_imm(ops: &str) -> Option<u32> {
        let part = ops.split(',').nth(1)?.trim();
        if let Some(h) = part.strip_prefix("0x") {
            u32::from_str_radix(h.trim_end_matches('h'), 16).ok()
        } else {
            part.trim_end_matches('h').parse().ok()
        }
    }
}

#[cfg(feature = "capstone")]
pub use imp::lift_x86_capstone;

#[cfg(not(feature = "capstone"))]
pub fn lift_x86_capstone(
    _bytes: &[u8],
    _name: &str,
    _base_vma: u64,
) -> Result<Module, LiftError> {
    Err(LiftError::CapstoneDisabled)
}
