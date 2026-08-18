//! B.A.S.E. RTL Generator — SIR → synthesizable Verilog.

mod mips;
pub mod verilog;

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
