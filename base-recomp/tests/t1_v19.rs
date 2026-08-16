//! Path v1.9 smoke: PE reject ELF, encode SH/MIPS, symbols, runtime stub.

use base_recomp::encode::encode_module;
use base_recomp::lift::lift_x86_32;
use base_recomp::pe::{load_pe_text, PeError};
use base_recomp::runtime::{runtime_status, ConsoleTarget};
use base_recomp::symbols::{resolve_symbols, SymbolMap};
use base_recomp::target::{SuperHFlavor, TargetIsa};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn pe_rejects_elf_object() {
    let err = load_pe_text(&fixture("add3.o")).unwrap_err();
    assert!(matches!(err, PeError::NotPe));
}

#[test]
fn encode_portable_sh_and_mips() {
    let bytes = [
        0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3,
    ];
    let m = lift_x86_32(&bytes, "add3").unwrap();
    let sh = encode_module(&m, TargetIsa::SuperH(SuperHFlavor::Sh2)).unwrap();
    assert!(!sh.is_empty());
    let mi = encode_module(&m, TargetIsa::Mips).unwrap();
    assert!(!mi.is_empty());
    let pp = encode_module(&m, TargetIsa::Ppc).unwrap();
    assert!(!pp.is_empty());
}

#[test]
fn symbol_resolve_emit_call() {
    use base_recomp::emit::emit_module;
    use base_recomp::lift::lift_x86_32_at;
    let mut m = lift_x86_32_at(&[0xE8, 0x00, 0x00, 0x00, 0x00, 0xC3], "c", 0x1000).unwrap();
    let mut map = SymbolMap::new();
    map.insert(0x1005, "helper".into());
    resolve_symbols(&mut m, &map);
    let asm = emit_module(&m, TargetIsa::X86_64);
    assert!(asm.contains("call helper"));
    assert!(!asm.contains("ud2"));
}

#[test]
fn runtime_stub_false() {
    assert!(!runtime_status(ConsoleTarget::SaturnSh2).runs);
    assert!(!runtime_status(ConsoleTarget::DreamcastSh4).runs);
}
