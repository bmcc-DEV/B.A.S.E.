//! SPARC RTL generation — SIR → synthesizable Verilog.

use std::fs;
use std::path::Path;
use base_recomp::elf::lift_elf_text;
use crate::verilog::VerilogCore;

pub fn generate(elf_path: Option<&Path>, output: &Path) -> anyhow::Result<VerilogCore> {
    let name = "sparc_sir_core".to_string();
    let init_mem = if let Some(path) = elf_path {
        let (text, _) = lift_elf_text(path, "main")?;
        let words: Vec<u32> = text.bytes
            .chunks(4)
            .map(|c| u32::from_be_bytes(c.try_into().unwrap_or([0; 4])))
            .collect();
        Some(words)
    } else { None };
    let verilog = include_str!("sparc_core.v")
        .replace("MODULE_NAME", &name)
        .replace("MEM_INIT_BLOCK", &init_mem.map(|_| String::new()).unwrap_or_default());
    let tb = VerilogCore { name, isa: "sparc".into(), width: 32, regs: 32, verilog, testbench: String::new() };
    fs::write(output, &tb.verilog)?;
    Ok(tb)
}
