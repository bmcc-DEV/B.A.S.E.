use base_rtl::{generate_rtl, generate_testbench};
use base_recomp::target::TargetIsa;

fn tmpdir(suffix: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("base_rtl_test_{}", suffix));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn mips_core_generates_valid_verilog() {
    let dir = tmpdir("mips");
    let out = dir.join("mips_core.v");
    let tb = dir.join("tb.v");
    let core = generate_rtl(TargetIsa::Mips, None, &out).unwrap();
    generate_testbench(&core, &tb).unwrap();
    let v = std::fs::read_to_string(&out).unwrap();
    assert!(v.contains("module mips_sir_core"));
    assert!(v.contains("always @(posedge clk)"));
    assert!(v.contains("wire [31:0]"));
}

#[test]
fn arm_core_generates_valid_verilog() {
    let dir = tmpdir("arm");
    let out = dir.join("arm_core.v");
    let tb = dir.join("tb.v");
    let core = generate_rtl(TargetIsa::Arm, None, &out).unwrap();
    generate_testbench(&core, &tb).unwrap();
    let v = std::fs::read_to_string(&out).unwrap();
    assert!(v.contains("module arm_sir_core"));
    assert!(v.contains("gpr [0:15]"), "ARM should have 16 GPRs");
    assert!(v.contains("cpsr_nzcv"), "ARM should have NZCV flags");
}

#[test]
fn aarch64_core_generates_valid_verilog() {
    let dir = tmpdir("aarch64");
    let out = dir.join("aarch64_core.v");
    let tb = dir.join("tb.v");
    let core = generate_rtl(TargetIsa::AArch64, None, &out).unwrap();
    generate_testbench(&core, &tb).unwrap();
    let v = std::fs::read_to_string(&out).unwrap();
    assert!(v.contains("module aarch64_sir_core"));
    assert!(v.contains("gpr [0:30]"), "AArch64 should have 31 GPRs");
    assert!(v.contains("wire [63:0] pc"), "AArch64 should have 64-bit PC");
}

#[test]
fn mips_fib_o_generates_verilog_with_init() {
    let elf = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../base-recomp/tests/fixtures/mips_fib.o");
    if !elf.exists() {
        eprintln!("SKIP: mips_fib.o not found");
        return;
    }
    let dir = tmpdir("mips_fib");
    let out = dir.join("mips_fib_core.v");
    let core = generate_rtl(TargetIsa::Mips, Some(&elf), &out).unwrap();
    let v = std::fs::read_to_string(&out).unwrap();
    assert!(v.contains("mem[0] = 32'h"), "missing memory init");
    assert!(v.contains("module mips_sir_core"));
}

#[test]
fn all_isas_compile_check() {
    for (isa, expected) in [
        (TargetIsa::Mips, "mips_sir_core"),
        (TargetIsa::Arm, "arm_sir_core"),
        (TargetIsa::AArch64, "aarch64_sir_core"),
    ] {
        let dir = tmpdir(&format!("{:?}", isa));
        let out = dir.join("core.v");
        let core = generate_rtl(isa, None, &out).unwrap();
        let v = std::fs::read_to_string(&out).unwrap();
        assert!(v.contains(&format!("module {}", expected)),
            "{}: missing module {}", isa, expected);
        assert!(v.contains("always @(posedge clk)"),
            "{}: missing clocked logic", isa);
    }
}
