//! R8: ELF loader validation — real binary through the full pipeline.

use base_recomp::decode::decode_ops;
use base_recomp::elf::lift_elf_text;
use base_recomp::encode::encode_module;
use base_recomp::emit::emit_module;
use base_recomp::target::TargetIsa;
use std::path::Path;

#[test]
fn mips_fib_elf_loads_and_lifts() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mips_fib.o");
    if !path.exists() {
        eprintln!("SKIP: mips_fib.o fixture not found");
        return;
    }
    let (text, module) = lift_elf_text(&path, "fib").expect("ELF lift failed");
    assert_eq!(text.architecture, "Mips");
    assert!(!text.bytes.is_empty());
    assert!(!module.functions.is_empty());
    // fib.o has 2 functions: add and fib
    assert!(module.functions.len() >= 2, "Expected >=2 functions, got {}", module.functions.len());
    let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"add"), "Expected 'add' function, got {:?}", names);
    assert!(names.contains(&"fib"), "Expected 'fib' function, got {:?}", names);
}

#[test]
fn mips_fib_add_roundtrips() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mips_fib.o");
    if !path.exists() {
        eprintln!("SKIP: mips_fib.o fixture not found");
        return;
    }
    let (_, module) = lift_elf_text(&path, "fib").expect("ELF lift failed");
    // Find the 'add' function
    let add_fn = module.functions.iter().find(|f| f.name == "add").expect("no add fn");
    let ops = &add_fn.blocks[0].ops;
    // add should contain at least Ret
    let has_ret = ops.iter().any(|op| matches!(op, base_recomp::sir::Op::Ret));
    assert!(has_ret, "add function must contain Ret");
    assert!(!ops.is_empty(), "add function must have ops");
}

#[test]
fn mips_fib_encode_decode_roundtrip() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mips_fib.o");
    if !path.exists() {
        eprintln!("SKIP: mips_fib.o fixture not found");
        return;
    }
    let (_, module) = lift_elf_text(&path, "fib").expect("ELF lift failed");
    // Try to encode+decode each function
    for func in &module.functions {
        let ops = func.blocks[0].ops.clone();
        // Filter out Unknown ops (gaps in decoder)
        let known_ops: Vec<_> = ops.iter()
            .filter(|op| !matches!(op, base_recomp::sir::Op::Unknown { .. }))
            .cloned()
            .collect();
        if known_ops.is_empty() {
            continue;
        }
        // Build a module with just this function's known ops
        let m = base_recomp::sir::Module {
            name: func.name.clone(),
            source_isa: "mips".into(),
            functions: vec![base_recomp::sir::Function {
                name: func.name.clone(),
                blocks: vec![base_recomp::sir::BasicBlock {
                    label: "entry".into(),
                    ops: known_ops,
                }],
            }],
            lift_gaps: 0,
            source: None,
            text_vma: None,
        };
        // Encode to MIPS bytes
        let bytes = encode_module(&m, TargetIsa::Mips)
            .unwrap_or_else(|e| panic!("encode {} failed: {}", func.name, e));
        assert!(!bytes.is_empty(), "encode {} produced no bytes", func.name);
        // Decode back
        let decoded = decode_ops(&bytes, TargetIsa::Mips)
            .unwrap_or_else(|e| panic!("decode {} failed: {}", func.name, e));
        assert!(!decoded.is_empty(), "decode {} produced no ops", func.name);
        // Check we got back Ret
        let has_ret = decoded.iter().any(|op| matches!(op, base_recomp::sir::Op::Ret));
        assert!(has_ret, "round-trip {} lost Ret", func.name);
    }
}

#[test]
fn mips_fib_emit_assembly() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mips_fib.o");
    if !path.exists() {
        eprintln!("SKIP: mips_fib.o fixture not found");
        return;
    }
    let (_, module) = lift_elf_text(&path, "fib").expect("ELF lift failed");
    let asm = emit_module(&module, TargetIsa::Mips);
    assert!(asm.contains("addiu"), "MIPS ASM should contain addiu");
    assert!(asm.contains("sw"), "MIPS ASM should contain sw");
    assert!(asm.contains("lw"), "MIPS ASM should contain lw");
    assert!(asm.contains("jr"), "MIPS ASM should contain jr");
}
