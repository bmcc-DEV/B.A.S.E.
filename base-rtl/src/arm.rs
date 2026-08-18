//! ARM (A32) RTL generation — SIR → synthesizable Verilog.

use std::fs;
use std::path::Path;

use base_recomp::elf::lift_elf_text;

use crate::verilog::VerilogCore;

/// Generate an ARM (A32) RTL core.
pub fn generate(elf_path: Option<&Path>, output: &Path) -> anyhow::Result<VerilogCore> {
    let name = "arm_sir_core".to_string();
    let init_mem = if let Some(path) = elf_path {
        let (text, _) = lift_elf_text(path, "main")?;
        let words: Vec<u32> = text.bytes
            .chunks(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap_or([0; 4])))
            .collect();
        Some(words)
    } else {
        None
    };
    let verilog = gen_arm_core(&name, &init_mem);
    let tb = VerilogCore {
        name: name.clone(),
        isa: "arm".into(),
        width: 32,
        regs: 16,
        verilog,
        testbench: String::new(),
    };
    fs::write(output, &tb.verilog)?;
    Ok(tb)
}

fn gen_arm_core(name: &str, init_mem: &Option<Vec<u32>>) -> String {
    let mut mem_init = String::new();
    if let Some(words) = init_mem {
        mem_init.push_str("    initial begin\n");
        for (i, w) in words.iter().enumerate() {
            mem_init.push_str(&format!("      imem[{}] = 32'h{:08X};\n", i, w));
        }
        mem_init.push_str("    end\n");
    }
    include_str!("arm_core.v")
        .replace("MODULE_NAME", name)
        .replace("MEM_INIT_BLOCK", &mem_init)
}
