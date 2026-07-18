//! Path v1.8: ELF .text → lift → emit.

use base_recomp::elf::{lift_elf_text, load_elf_text};
use base_recomp::emit::emit_module;
use base_recomp::sir::Op;
use base_recomp::target::TargetIsa;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn load_add3_elf_text() {
    let text = load_elf_text(&fixture("add3.o")).expect("load elf");
    assert_eq!(text.section_name, ".text");
    assert!(!text.bytes.is_empty());
    // gas: b8 01 00 00 00 ; 83 c0 02 ; c3
    assert_eq!(text.bytes[0], 0xb8);
    assert_eq!(*text.bytes.last().unwrap(), 0xc3);
}

#[test]
fn lift_add3_elf_no_gaps() {
    let (text, module) = lift_elf_text(&fixture("add3.o"), "add3").unwrap();
    assert_eq!(module.lift_gaps, 0, "bytes={:02x?}", text.bytes);
    let ops = &module.functions[0].blocks[0].ops;
    assert!(matches!(ops[0], Op::MovImm { imm: 1, .. }));
    assert!(matches!(ops[1], Op::AddImm { imm: 2, .. }));
    assert!(matches!(ops[2], Op::Ret));
    assert!(module.source.as_ref().unwrap().contains("add3.o"));
}

#[test]
fn emit_add3_elf_to_sh2() {
    let (_, module) = lift_elf_text(&fixture("add3.o"), "add3").unwrap();
    let asm = emit_module(&module, "sh2".parse::<TargetIsa>().unwrap());
    assert!(asm.contains("mov #1, r0"));
    assert!(asm.contains("add #2, r0"));
    assert!(asm.contains("rts"));
}
