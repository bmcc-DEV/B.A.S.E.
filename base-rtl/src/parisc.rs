use std::fs; use std::path::Path;
use base_recomp::elf::lift_elf_text;
use crate::verilog::VerilogCore;
pub fn generate(elf_path: Option<&Path>, output: &Path) -> anyhow::Result<VerilogCore> {
    let name = "parisc_sir_core".to_string();
    let verilog = include_str!("parisc_core.v").replace("MODULE_NAME", &name);
    let tb = VerilogCore { name, isa: "parisc".into(), width: 32, regs: 32, verilog, testbench: String::new() };
    fs::write(output, &tb.verilog)?;
    Ok(tb)
}
