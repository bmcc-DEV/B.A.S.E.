//! x86_64 RTL generation — SIR → synthesizable Verilog.

use std::fs;
use std::path::Path;

use base_recomp::elf::lift_elf_text;
use crate::verilog::VerilogCore;

/// Generate an x86_64 RTL core.
pub fn generate(elf_path: Option<&Path>, output: &Path) -> anyhow::Result<VerilogCore> {
    let name = "x86_sir_core".to_string();
    let init_mem = if let Some(path) = elf_path {
        let (text, _) = lift_elf_text(path, "main")?;
        let words: Vec<u64> = text.bytes
            .chunks(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap_or([0; 8])))
            .collect();
        Some(words)
    } else { None };
    let verilog = include_str!("x86_core.v")
        .replace("MODULE_NAME", &name)
        .replace("MEM_INIT_BLOCK", &init_mem.map(|_| String::new()).unwrap_or_default());
    let tb = VerilogCore { name, isa: "x86_64".into(), width: 64, regs: 16, verilog, testbench: String::new() };
    fs::write(output, &tb.verilog)?;
    Ok(tb)
}
