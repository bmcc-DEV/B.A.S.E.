//! MIPS RTL generation — SIR → synthesizable Verilog.

use std::fs;
use std::path::Path;

use base_recomp::decode::decode_ops;
use base_recomp::elf::lift_elf_text;
use base_recomp::sir::Op;
use base_recomp::target::TargetIsa;

use crate::verilog::VerilogCore;

/// Generate a MIPS RTL core, optionally loading an ELF for instruction memory.
pub fn generate(elf_path: Option<&Path>, output: &Path) -> anyhow::Result<VerilogCore> {
    let name = "mips_sir_core".to_string();

    let init_mem = if let Some(path) = elf_path {
        let (text, _module) = lift_elf_text(path, "main")?;
        let words: Vec<u32> = text.bytes
            .chunks(4)
            .map(|c| u32::from_be_bytes(c.try_into().unwrap_or([0; 4])))
            .collect();
        Some(words)
    } else {
        None
    };

    let verilog = gen_mips_core(&name, &init_mem);
    let tb = VerilogCore {
        name: name.clone(),
        isa: "mips".into(),
        width: 32,
        regs: 32,
        verilog,
        testbench: String::new(),
    };
    fs::write(output, &tb.verilog)?;
    Ok(tb)
}

fn gen_mips_core(name: &str, init_mem: &Option<Vec<u32>>) -> String {
    let mut mem_init = String::new();
    if let Some(words) = init_mem {
        mem_init.push_str("    initial begin\n");
        for (i, w) in words.iter().enumerate() {
            mem_init.push_str(&format!("      mem[{}] = 32'h{:08X};\n", i, w));
        }
        mem_init.push_str("    end\n");
    }

    let template = include_str!("mips_core.v");
    template
        .replace("MODULE_NAME", name)
        .replace("MEM_INIT_BLOCK", &mem_init)
}
