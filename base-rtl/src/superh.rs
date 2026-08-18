use std::fs; use std::path::Path;
use base_recomp::elf::lift_elf_text;
use crate::verilog::VerilogCore;
pub fn generate(elf_path: Option<&Path>, output: &Path) -> anyhow::Result<VerilogCore> {
    let name = "superh_sir_core".to_string();
    let init_mem = elf_path.map(|p| {
        let (t, _) = lift_elf_text(p, "main").unwrap();
        t.bytes.chunks(4).map(|c| u16::from_le_bytes(c.try_into().unwrap_or([0;2]))).collect::<Vec<_>>()
    });
    let verilog = include_str!("superh_core.v").replace("MODULE_NAME", &name);
    let tb = VerilogCore { name, isa: "superh".into(), width: 32, regs: 16, verilog, testbench: String::new() };
    fs::write(output, &tb.verilog)?;
    Ok(tb)
}
