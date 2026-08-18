//! B.A.S.E. RTL Generator — SIR → synthesizable Verilog.
//!
//! Generates synthesizable Verilog cores from SIR/ELF input.
//! Supports MIPS, ARM, AArch64 targets.

mod mips;
mod arm;
mod aarch64;
pub mod verilog;
mod pipeline;

pub use verilog::VerilogCore;

use std::path::Path;
use base_recomp::target::TargetIsa;

/// Generate RTL for a target ISA.
pub fn generate_rtl(
    isa: TargetIsa,
    elf_path: Option<&Path>,
    output: &Path,
) -> anyhow::Result<VerilogCore> {
    match isa {
        TargetIsa::Mips => mips::generate(elf_path, output),
        TargetIsa::Arm => arm::generate(elf_path, output),
        TargetIsa::AArch64 => aarch64::generate(elf_path, output),
        _ => anyhow::bail!("RTL generation not yet implemented for {:?}", isa),
    }
}

/// Generate a testbench for a given core.
pub fn generate_testbench(
    core: &VerilogCore,
    output: &Path,
) -> anyhow::Result<()> {
    let tb = core.testbench();
    std::fs::write(output, tb)?;
    Ok(())
}
