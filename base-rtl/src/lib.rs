//! B.A.S.E. RTL Generator — SIR → synthesizable Verilog.

mod mips;
mod arm;
mod aarch64;
mod ppc;
mod sparc;
mod coldfire;
mod superh;
mod alpha;
mod parisc;
mod m88k;
mod ia64;
mod i860;
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
        TargetIsa::Ppc => ppc::generate(elf_path, output),
        TargetIsa::Sparc => sparc::generate(elf_path, output),
        TargetIsa::ColdFire => coldfire::generate(elf_path, output),
        TargetIsa::SuperH(_) => superh::generate(elf_path, output),
        TargetIsa::Alpha => alpha::generate(elf_path, output),
        TargetIsa::PaRisc => parisc::generate(elf_path, output),
        TargetIsa::M88k => m88k::generate(elf_path, output),
        TargetIsa::Ia64 => ia64::generate(elf_path, output),
        TargetIsa::I860 => i860::generate(elf_path, output),
        TargetIsa::X86_64 => anyhow::bail!("x86_64 is the source ISA, not a target for RTL"),
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
