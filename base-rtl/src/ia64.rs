use std::fs; use std::path::Path;
use crate::verilog::VerilogCore;
pub fn generate(_elf_path: Option<&Path>, output: &Path) -> anyhow::Result<VerilogCore> {
    let name = "ia64_sir_core".to_string();
    let verilog = include_str!("ia64_core.v").replace("MODULE_NAME", &name);
    let tb = VerilogCore { name, isa: "ia64".into(), width: 64, regs: 128, verilog, testbench: String::new() };
    fs::write(output, &tb.verilog)?;
    Ok(tb)
}
