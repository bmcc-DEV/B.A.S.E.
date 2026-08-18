use std::fs; use std::path::Path;
use crate::verilog::VerilogCore;
pub fn generate(_elf_path: Option<&Path>, output: &Path) -> anyhow::Result<VerilogCore> {
    let name = "i860_sir_core".to_string();
    let verilog = include_str!("i860_core.v").replace("MODULE_NAME", &name);
    let tb = VerilogCore { name, isa: "i860".into(), width: 32, regs: 32, verilog, testbench: String::new() };
    fs::write(output, &tb.verilog)?;
    Ok(tb)
}
