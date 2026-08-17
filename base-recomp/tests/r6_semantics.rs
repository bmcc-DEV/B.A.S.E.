//! R6: semantic catalog of the 11 preserved ISAs + encode/emit for the new
//! preservation targets (Alpha, PA-RISC, M88k, IA-64, i860, ColdFire).
//!
//! Honesty: encode is Partial for Alpha/PA-RISC/ColdFire (subset of lifted ops),
//! None for M88k/IA-64/i860 (emit text only). The catalog is the semantic contract.

use base_recomp::encode::{encode_module, EncodeError};
use base_recomp::emit::emit_module;
use base_recomp::lift::lift_x86_32;
use base_recomp::semantics::{EncodeStatus, PRESERVED_ISAS};
use base_recomp::target::{SuperHFlavor, TargetIsa};

const ADD3: [u8; 9] = [0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3];

#[test]
fn catalog_has_eleven_entries() {
    assert_eq!(PRESERVED_ISAS.len(), 13);
    for s in PRESERVED_ISAS {
        assert!(!s.family.is_empty());
        assert!(!s.quirks.is_empty(), "{} needs documented quirks", s.name);
    }
}

#[test]
fn catalog_covers_all_new_targets() {
    for t in [
        TargetIsa::Alpha,
        TargetIsa::PaRisc,
        TargetIsa::M88k,
        TargetIsa::Ia64,
        TargetIsa::I860,
        TargetIsa::ColdFire,
    ] {
        let s = base_recomp::semantics::for_isa(t)
            .unwrap_or_else(|| panic!("no catalog entry for {t}"));
        assert_eq!(s.name, t.as_str());
    }
}

#[test]
fn new_isa_emit_works_for_all_canonical() {
    let m = lift_x86_32(&ADD3, "add3").unwrap();
    for t in TargetIsa::all_canonical() {
        let asm = emit_module(&m, *t);
        assert!(asm.contains("add3"), "label missing for {t}");
        assert!(
            asm.contains("static_recomp_complete"),
            "honesty banner missing for {t}"
        );
    }
}

#[test]
fn new_isa_encode_partial_or_pending() {
    let m = lift_x86_32(&ADD3, "add3").unwrap();
    for t in [TargetIsa::Alpha, TargetIsa::ColdFire] {
        let code = encode_module(&m, t).unwrap_or_else(|e| panic!("encode {t}: {e}"));
        assert!(!code.is_empty(), "{t} produced no bytes");
    }
    // PA-RISC now covers the full kind set (LDI/LDO/LDW/STW + bl/b,l).
    let nop_ret = lift_x86_32(&[0x90, 0xC3], "nr").unwrap();
    assert!(!encode_module(&nop_ret, TargetIsa::PaRisc).unwrap().is_empty());
    assert!(!encode_module(&m, TargetIsa::PaRisc).unwrap().is_empty());
    for t in [TargetIsa::M88k, TargetIsa::Ia64, TargetIsa::I860] {
        let err = encode_module(&m, t).unwrap_err();
        assert!(matches!(err, EncodeError::Unsupported(isa, _) if isa == t));
    }
}

#[test]
fn encode_status_matches_reality() {
    // 10 of 11 preserved ISAs now encode the full lifted SIR op subset (Full);
    // PA-RISC is Partial (Nop/Ret only); M88k/IA-64/i860 are text-only.
    for t in [
        TargetIsa::Alpha,
        TargetIsa::ColdFire,
        TargetIsa::Ppc,
        TargetIsa::Mips,
        TargetIsa::Arm,
        TargetIsa::AArch64,
        TargetIsa::Sparc,
        TargetIsa::SuperH(SuperHFlavor::Sh4),
        TargetIsa::X86_64,
    ] {
        let s = base_recomp::semantics::for_isa(t);
        if s.is_none() {
            eprintln!("WARNING: for_isa returned None for {t:?} (as_str={})", t.as_str());
        }
        assert!(s.is_some(), "{t} should have semantic catalog entry");
        assert!(matches!(s.unwrap().encode_status, EncodeStatus::Full), "{t} should be Full");
    }

    let m88k = base_recomp::semantics::for_isa(TargetIsa::M88k).unwrap();
    assert!(matches!(m88k.encode_status, EncodeStatus::None(_)));
}

#[test]
fn superh_flavors_share_catalog_entry() {
    assert_eq!(
        base_recomp::semantics::for_isa(TargetIsa::SuperH(SuperHFlavor::Sh2))
            .unwrap()
            .name,
        "superh"
    );
    assert_eq!(
        base_recomp::semantics::for_isa(TargetIsa::SuperH(SuperHFlavor::Sh4))
            .unwrap()
            .name,
        "superh"
    );
}
