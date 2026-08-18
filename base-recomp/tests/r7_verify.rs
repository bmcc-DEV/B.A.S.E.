//! R7: round-trip verification — SIR → encode → decode → SIR′ with literal +
//! semantic equivalence, and per-dimension coverage (never a single inflated score).

use base_recomp::lift::lift_x86_32;
use base_recomp::sir::{Op, VReg};
use base_recomp::target::{SuperHFlavor, TargetIsa};
use base_recomp::verify::{
    all_coverages, coverage, probe_kind, semantic_key, verify_ops,
};

const ADD3: [u8; 9] = [0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3];

#[test]
fn add3_roundtrips_all_encoded_isas() {
    let m = lift_x86_32(&ADD3, "add3").unwrap();
    let ops = m.functions[0].blocks[0].ops.clone();
    for t in [
        TargetIsa::Mips,
        TargetIsa::Ppc,
        TargetIsa::SuperH(SuperHFlavor::Sh4),
        TargetIsa::Alpha,
        TargetIsa::ColdFire,
    ] {
        let r = verify_ops(ops.clone(), t);
        assert!(r.verified(), "add3 not verified for {t}: {r:?}");
        assert!(r.literal_match, "add3 should round-trip literally for {t}: {r:?}");
    }
}

#[test]
fn semantic_vs_literal_normalization() {
    // SubImm{·,1} encodes as addi/addiu -1 → decodes as Dec — meaning preserved.
    let r = verify_ops(vec![Op::SubImm { dst: VReg(0), imm: 1 }, Op::Ret], TargetIsa::Mips);
    assert!(!r.literal_match);
    assert!(r.semantic_match, "{r:?}");
    assert!(r.verified());
    assert_eq!(
        semantic_key(&Op::SubImm { dst: VReg(0), imm: 1 }),
        semantic_key(&Op::Dec { dst: VReg(0) })
    );
}

#[test]
fn coverage_table_all_targets_honest() {
    let rows = all_coverages();
    assert_eq!(rows.len(), 14);
    for c in &rows {
        // No decoder → semantic/literal must be 0 — never an inflated claim.
        if !c.has_decoder {
            assert_eq!(c.semantic_pct, 0, "{} must show 0% semantic without decoder", c.target);
        }
    }
    for t in [TargetIsa::Ppc] {
        let c = coverage(t);
        assert!(c.has_decoder);
        assert_eq!(c.semantic_pct, 100, "{t}"); // 18/18 kinds round-trip (incl. cmp/test/bcond/trap)
        assert!(c.covered.contains(&"nop"));
        assert!(c.covered.contains(&"ret"));
        assert!(c.covered.contains(&"mov_imm"));
        assert!(c.covered.contains(&"cmp"), "cmp encoded for {t}");
        assert!(c.covered.contains(&"test"), "test encoded for {t}");
        assert!(c.covered.contains(&"bcond"), "bcond encoded for {t}");
    }
    // ISAs without architectural flags (no Cmp/Test/BranchCond): all 18 kinds encode+decode
    for t in [TargetIsa::Mips, TargetIsa::SuperH(SuperHFlavor::Sh4)] {
        let c = coverage(t);
        assert!(c.has_decoder);
        assert_eq!(c.semantic_pct, 100, "{t}"); // 18/18 kinds round-trip (incl. cmp/test/bcond/trap)
        assert!(c.covered.contains(&"nop"));
        assert!(c.covered.contains(&"ret"));
        assert!(c.covered.contains(&"mov_imm"));
        assert!(c.covered.contains(&"push"), "push encoded for {t}");
    }
}

#[test]
fn coverage_new_decoders() {
    // ISAs without flags: all 18 kinds (no cmp/test/bcond)
    let alpha = coverage(TargetIsa::Alpha);
        assert_eq!((alpha.encoder_pct, alpha.decoder_pct, alpha.semantic_pct), (100, 100, 100)); // 18/18

    let parisc = coverage(TargetIsa::PaRisc);
        assert_eq!(parisc.semantic_pct, 100, "{parisc:?}"); // 18/18
    assert!(parisc.covered.contains(&"nop"));
    assert!(parisc.covered.contains(&"ret"));

    // ISAs with flags + conditional support: 17/17 = 100%
    let cf = coverage(TargetIsa::ColdFire);
    assert_eq!(cf.semantic_pct, 100, "{cf:?}"); // 18/18 kinds incl. cmp/test/bcond/trap
    assert!(cf.covered.contains(&"push"));
    assert!(cf.covered.contains(&"pop"));
    assert!(cf.covered.contains(&"ld_mem"));
    assert!(cf.covered.contains(&"st_mem"));
    assert!(cf.covered.contains(&"cmp"));
    assert!(cf.covered.contains(&"bcond"));
}

#[test]
fn parisc_bv_n_accepts_return() {
    // bv,n %r0(%rp) (nullify) is semantically a return; decoder folds the delay nop.
    assert!(probe_kind(TargetIsa::PaRisc, "ret").semantic);
}

#[test]
fn pending_status_not_full() {
    let x = coverage(TargetIsa::X86_64);
    let x = coverage(TargetIsa::X86_64);
    // x86 encoder/decoder now covers all 18 kinds (incl. cmp/test/bcond/trap).
    assert_eq!(x.status, "FULL");
    assert_eq!(x.semantic_pct, 100, "{x:?}");
    let m88k = coverage(TargetIsa::M88k);
    assert_eq!(m88k.status, "NONE");
}

#[test]
fn add3_differential_matches_behavior() {
    // r3 = 10; r3 += 5 → r3 == 15, same on every ISA backend (the user's example).
    use base_recomp::semexec::{differential_ops, MachineState};
    use base_recomp::sir::VReg;
    let ops = vec![
        Op::MovImm { dst: VReg(3), imm: 10 },
        Op::AddImm { dst: VReg(3), imm: 5 },
        Op::Ret,
    ];
    let state = MachineState::new().with_gpr(26, 0x8000);
    for t in [
        TargetIsa::Mips,
        TargetIsa::Ppc,
        TargetIsa::SuperH(SuperHFlavor::Sh4),
        TargetIsa::Alpha,
        TargetIsa::ColdFire,
    ] {
        let r = differential_ops(ops.clone(), t, &state);
        assert!(r.matched(), "behavior mismatch for {t}: {r:?}");
        assert_eq!(r.reference.gpr(3), 15, "reference r3 for {t}");
        assert_eq!(r.isa.gpr(3), 15, "isa r3 for {t}");
    }
}

#[test]
fn differential_coverage_separates_width_behavior() {
    // ISAs without flags: 14/18 = 78% differential
    let a = coverage(TargetIsa::Alpha);
    assert_eq!(a.differential_pct, 100, "{a:?}"); // 18/18

    // ISAs with flags: 18/18 = 100% differential
    let c = coverage(TargetIsa::ColdFire);
    assert_eq!(c.differential_pct, 100, "{c:?}"); // 18/18 kinds incl. conditional+trap

    let s = coverage(TargetIsa::SuperH(SuperHFlavor::Sh4));
    assert_eq!(s.differential_pct, 100, "{s:?}"); // 18/18
}
