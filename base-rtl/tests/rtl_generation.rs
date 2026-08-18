use base_rtl::{generate_rtl, generate_testbench};
use base_recomp::target::TargetIsa;

fn tmpdir(suffix: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("base_rtl_test_{}", suffix));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn all_isas_generate_valid_verilog() {
    for (isa, expected) in [
        (TargetIsa::Mips, "mips_sir_core"),
        (TargetIsa::Arm, "arm_sir_core"),
        (TargetIsa::AArch64, "aarch64_sir_core"),
        (TargetIsa::Ppc, "ppc_sir_core"),
        (TargetIsa::Sparc, "sparc_sir_core"),
        (TargetIsa::ColdFire, "coldfire_sir_core"),
        (TargetIsa::SuperH(base_recomp::target::SuperHFlavor::Sh4), "superh_sir_core"),
        (TargetIsa::Alpha, "alpha_sir_core"),
        (TargetIsa::PaRisc, "parisc_sir_core"),
        (TargetIsa::M88k, "m88k_sir_core"),
        (TargetIsa::Ia64, "ia64_sir_core"),
        (TargetIsa::I860, "i860_sir_core"),
    ] {
        let dir = tmpdir(&format!("{:?}", isa));
        let out = dir.join("core.v");
        let tb = dir.join("tb.v");
        let core = generate_rtl(isa, None, &out).unwrap();
        generate_testbench(&core, &tb).unwrap();
        let v = std::fs::read_to_string(&out).unwrap();
        assert!(v.contains(&format!("module {}", expected)), "{}: missing module", isa);
        assert!(v.contains("always @(posedge clk)"), "{}: missing clocked logic", isa);
    }
}

#[test]
fn mips_fib_o_generates_verilog_with_init() {
    let elf = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../base-recomp/tests/fixtures/mips_fib.o");
    if !elf.exists() { eprintln!("SKIP: mips_fib.o not found"); return; }
    let dir = tmpdir("mips_fib");
    let out = dir.join("mips_fib_core.v");
    let core = generate_rtl(TargetIsa::Mips, Some(&elf), &out).unwrap();
    let v = std::fs::read_to_string(&out).unwrap();
    assert!(v.contains("mem[0] = 32'h"), "missing memory init");
    assert!(v.contains("module mips_sir_core"));
}

#[test]
fn each_isa_has_unique_architecture() {
    let tests = [
        (TargetIsa::Arm, "gpr [0:15]", "cpsr_nzcv"),
        (TargetIsa::AArch64, "gpr [0:30]", "nzcv"),
        (TargetIsa::Ppc, "gpr [0:31]", "cr0"),
        (TargetIsa::Sparc, "icc", "gpr"),
        (TargetIsa::ColdFire, "dreg [0:7]", "ccr"),
        (TargetIsa::SuperH(base_recomp::target::SuperHFlavor::Sh4), "gpr [0:15]", "t_flag"),
        (TargetIsa::Alpha, "gpr [0:31]", "pc"),
        (TargetIsa::PaRisc, "gpr [0:31]", "pc"),
        (TargetIsa::M88k, "gpr [0:31]", "cr"),
        (TargetIsa::Ia64, "gpr [0:127]", "nat"),
        (TargetIsa::I860, "gpr [0:31]", "psr"),
    ];
    for (isa, feat1, feat2) in tests {
        let dir = tmpdir(&format!("{:?}_arch", isa));
        let out = dir.join("core.v");
        generate_rtl(isa, None, &out).unwrap();
        let v = std::fs::read_to_string(&out).unwrap();
        assert!(v.contains(feat1), "{}: missing feature '{}'", isa, feat1);
        assert!(v.contains(feat2), "{}: missing feature '{}'", isa, feat2);
    }
}
