//! Static Intermediate Representation (SIR).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VReg(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    Nop,
    Ret,
    /// `dst := imm` (32-bit).
    MovImm { dst: VReg, imm: u32 },
    /// `dst := dst + imm`
    AddImm { dst: VReg, imm: u32 },
    /// `dst := dst - imm`
    SubImm { dst: VReg, imm: u32 },
    /// `dst := 0` (from `xor dst,dst`)
    Clear { dst: VReg },
    Inc { dst: VReg },
    Dec { dst: VReg },
    Push { src: VReg },
    Pop { dst: VReg },
    /// Relative call; optional resolved symbol name for emit.
    CallRel {
        rel: i32,
        target: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        symbol: Option<String>,
    },
    JmpRel {
        rel: i32,
        target: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        symbol: Option<String>,
    },
    /// Unliftable / unsupported opcode — wedge for `base-reason`.
    Unknown { offset: u64, bytes: Vec<u8>, note: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicBlock {
    pub label: String,
    pub ops: Vec<Op>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub source_isa: String,
    pub functions: Vec<Function>,
    pub lift_gaps: usize,
    /// Optional provenance (ELF path / section).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_vma: Option<u64>,
}

impl Module {
    pub fn count_gaps(&self) -> usize {
        self.functions
            .iter()
            .flat_map(|f| f.blocks.iter())
            .flat_map(|b| b.ops.iter())
            .filter(|o| matches!(o, Op::Unknown { .. }))
            .count()
    }
}
