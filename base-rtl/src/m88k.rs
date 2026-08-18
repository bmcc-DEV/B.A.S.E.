use std::fs; use std::path::Path;
use crate::verilog::VerilogCore;
pub fn generate(_elf_path: Option<&Path>, output: &Path) -> anyhow::Result<VerilogCore> {
    let name = "m88k_sir_core".to_string();
    let verilog = include_str!("m88k_core.v").replace("MODULE_NAME", &name);
    let tb = VerilogCore { name, isa: "m88k".into(), width: 32, regs: 32, verilog, testbench: String::new() };
    fs::write(output, &tb.verilog)?;
    Ok(tb)
}
