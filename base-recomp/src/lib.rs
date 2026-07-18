//! Static x86 → multi-ISA recompilation (SIR).
//!
//! Honesty: not Wine, not a complete Win32 ABI, not “runs any PE”.

pub mod elf;
pub mod emit;
pub mod gaps;
pub mod honesty;
pub mod lift;
pub mod roundtrip;
pub mod sir;
pub mod target;

pub use honesty::{
    STATIC_RECOMP_COMPLETE, WIN32_ABI_COMPLETE, RUNS_ANY_PE, markdown_section,
};
pub use sir::{BasicBlock, Function, Module, Op, VReg};
pub use target::TargetIsa;

/// Lift raw x86-32 bytes into a SIR module, then emit assembly for `target`.
pub fn recompile_bytes(bytes: &[u8], name: &str, target: TargetIsa) -> anyhow::Result<String> {
    let module = lift::lift_x86_32(bytes, name)?;
    Ok(emit::emit_module(&module, target))
}
