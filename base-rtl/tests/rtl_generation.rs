use base_rtl::{generate_rtl, generate_testbench};
use base_recomp::target::TargetIsa;

#[test]
fn mips_core_generates_valid_verilog() {
    let dir = std::env::temp_dir().join("base_rtl_test");
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("mips_core.v");
    let tb = dir.join("tb_mips_core.v");
    
    let core = generate_rtl(TargetIsa::Mips, None, &out).unwrap();
    generate_testbench(&core, &tb).unwrap();
    
    assert!(out.exists(), "Verilog core not written");
    assert!(tb.exists(), "Testbench not written");
    
    let verilog = std::fs::read_to_string(&out).unwrap();
    assert!(verilog.contains("module mips_sir_core"), "missing module declaration");
    assert!(verilog.contains("wire [31:0]"), "missing wire declarations");
    assert!(verilog.contains("always @(posedge clk)"), "missing clocked logic");
    
    let testbench = std::fs::read_to_string(&tb).unwrap();
    assert!(testbench.contains("module tb_mips_sir_core"), "missing testbench module");
}

#[test]
fn mips_fib_o_generates_verilog_with_init() {
    let elf = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../base-recomp/tests/fixtures/mips_fib.o");
    if !elf.exists() {
        eprintln!("SKIP: mips_fib.o not found");
        return;
    }
    let dir = std::env::temp_dir().join("base_rtl_test_fib");
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("mips_fib_core.v");
    
    let core = generate_rtl(TargetIsa::Mips, Some(&elf), &out).unwrap();
    let verilog = std::fs::read_to_string(&out).unwrap();
    assert!(verilog.contains("mem[0] = 32'h"), "missing memory init for fib.o");
    assert!(verilog.contains("module mips_sir_core"), "missing module");
}
