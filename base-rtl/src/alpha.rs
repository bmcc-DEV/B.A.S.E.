use std::fs; use std::path::Path;
use base_recomp::elf::lift_elf_text;
use crate::verilog::VerilogCore;
pub fn generate(elf_path: Option<&Path>, output: &Path) -> anyhow::Result<VerilogCore> {
    let name = "alpha_sir_core".to_string();
    let verilog = include_str!("alpha_core.v").replace("MODULE_NAME", &name);
    let tb = VerilogCore { name, isa: "alpha".into(), width: 64, regs: 32, verilog, testbench: String::new() };
    fs::write(output, &tb.verilog)?;
    Ok(tb)
}
