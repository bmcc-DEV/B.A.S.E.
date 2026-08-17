//! Semantic ISA catalog (Path v1.9): *preserve the semantics of an architecture,
//! not just execute its binaries.*
//!
//! Data-driven, serde-serializable description of each preserved architecture:
//! registers, endianness, delay slots, flags, ABI and quirks — the "what does the
//! hardware do" model that B.A.S.E. keeps even when no physical part exists.
//!
//! This is the source of truth for the 11 preserved ISAs. `TargetIsa::all_canonical()`
//! is the machine target set; this catalog is the semantic contract behind it.

use serde::Serialize;

use crate::target::TargetIsa;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Endianness {
    Little,
    Big,
}

/// How complete the SIR→machine-code encoder is for this ISA (`base recomp encode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EncodeStatus {
    /// The whole lifted SIR op subset encodes.
    Full,
    /// Only the listed ops encode; the rest return `EncodeError::Unsupported`.
    Partial(&'static str),
    /// No encoder yet — emit text only.
    None(&'static str),
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct IsaSemantics {
    /// Canonical target name (`TargetIsa::as_str`).
    pub name: &'static str,
    pub family: &'static str,
    pub word_bits: u8,
    pub endianness: Endianness,
    pub gpr_count: u8,
    pub zero_register: Option<&'static str>,
    pub stack_register: &'static str,
    /// Return-address / link register, when the ISA has one.
    pub link_register: Option<&'static str>,
    pub branch_delay_slots: u8,
    pub load_delay_slots: u8,
    /// Explicit architectural condition flags (CCR/PSR/CPSR…) vs compare-and-branch.
    pub architectural_flags: bool,
    pub flags: &'static [&'static str],
    pub register_windows: bool,
    pub predication: bool,
    pub vector_extensions: &'static [&'static str],
    pub calling_convention: Option<&'static str>,
    pub quirks: &'static [&'static str],
    pub encode_status: EncodeStatus,
}

#[allow(clippy::too_many_arguments)] // static data constructor for the catalog
const fn s(
    name: &'static str,
    family: &'static str,
    word_bits: u8,
    endianness: Endianness,
    gpr_count: u8,
    zero_register: Option<&'static str>,
    stack_register: &'static str,
    link_register: Option<&'static str>,
    branch_delay_slots: u8,
    load_delay_slots: u8,
    architectural_flags: bool,
    flags: &'static [&'static str],
    register_windows: bool,
    predication: bool,
    vector_extensions: &'static [&'static str],
    calling_convention: Option<&'static str>,
    quirks: &'static [&'static str],
    encode_status: EncodeStatus,
) -> IsaSemantics {
    IsaSemantics {
        name,
        family,
        word_bits,
        endianness,
        gpr_count,
        zero_register,
        stack_register,
        link_register,
        branch_delay_slots,
        load_delay_slots,
        architectural_flags,
        flags,
        register_windows,
        predication,
        vector_extensions,
        calling_convention,
        quirks,
        encode_status,
    }
}

/// The 11 preserved architectures (Path v1.9). Order mirrors the user's target list.
pub const PRESERVED_ISAS: &[IsaSemantics] = &[
    s(
        "ppc",
        "Power / PowerPC",
        32,
        Endianness::Big,
        32,
        None,
        "r1",
        Some("lr"),
        0,
        0,
        true,
        &["cr", "xer", "so", "ov", "ca"],
        false,
        false,
        &["AltiVec (VMX)"],
        Some("SysV PPC32"),
        &[
            "bi-endian (default big; POWER9+ per-process)",
            "r0 is real, but reads as 0 in base+offset (addi) addressing",
            "branch via CTR/LR; no branch delay slot",
            "exceptions/MMU via MSR + BAT/SLB",
        ],
        EncodeStatus::Full,
    ),
    s(
        "mips",
        "MIPS",
        32,
        Endianness::Big,
        32,
        Some("$zero"),
        "$sp",
        Some("$ra"),
        1,
        1,
        false,
        &[],
        false,
        false,
        &["MIPS DSP ASE", "MDMX", "MIPS-3D"],
        Some("o32"),
        &[
            "branch delay slot (architectural)",
            "load delay slot (MIPS I)",
            "no integer condition flags — compare-and-branch",
            "$zero reads 0; hi/lo for multiply/divide",
            "CP0 for MMU/exceptions; bi-endian (big default)",
        ],
        EncodeStatus::Full,
    ),
    s(
        "alpha",
        "DEC Alpha (AXP)",
        64,
        Endianness::Little,
        32,
        Some("r31"),
        "r30",
        Some("r26 (ra)"),
        0,
        0,
        false,
        &[],
        false,
        false,
        &[],
        Some("Tru64 UNIX (differs from Linux on r27/gp)"),
        &[
            "little-endian from day one — unusual for 1992 64-bit RISC",
            "no byte addressing — loads/stores are 32/64-bit; ldq_u/stq_u for unaligned",
            "no integer condition flags — compare + conditional branch",
            "r31 reads as 0; writes to r31 ignored",
            "PALcode (call_pal) for privileged/arch-specific ops",
        ],
        EncodeStatus::Full,
    ),
    s(
        "parisc",
        "HP PA-RISC (HPPA)",
        32,
        Endianness::Big,
        32,
        Some("r0"),
        "%sp (r30)",
        Some("%rp (r2)"),
        1,
        0,
        false,
        &[],
        false,
        false,
        &["PA-RISC 2.0 multimedia (MAX)"],
        Some("HP-UX ELF/PA-RISC"),
        &[
            "branch delay slot with nullification",
            "r0 hardwired to 0",
            "space registers sr0–sr7; address space via space IDs",
            "1.1 is 32-bit, 2.0 is 64-bit",
            "no general flags — compare-and-branch / carry via DC registers",
        ],
        EncodeStatus::Full,
    ),
    s(
        "arm",
        "ARM (A32/A64)",
        32,
        Endianness::Little,
        16,
        None,
        "sp (r13)",
        Some("lr (r14)"),
        0,
        0,
        true,
        &["n", "z", "c", "v"],
        false,
        false,
        &["NEON", "VFP"],
        Some("AAPCS (AAPCS32)"),
        &[
            "r15 = PC visible as a GPR",
            "A32 conditional execution; Thumb/Thumb-2 as alternate encodings",
            "bi-endian; newer cores BE8",
            "v4–v8 spanned: M-profile, A-profile, R-profile",
        ],
        EncodeStatus::Full,
    ),
    s(
        "m88k",
        "Motorola 88000 (88100/88110)",
        32,
        Endianness::Big,
        32,
        Some("r0"),
        "r31",
        Some("r1"),
        1,
        0,
        true,
        &["z", "n", "c", "v"],
        false,
        false,
        &[],
        Some("m88k System V"),
        &[
            "branch delay slot",
            "r0 hardwired to 0",
            "unified RISC GPR file, RISC-1 style",
            "88110 dual-issue (two independent 88100 cores on one die)",
            "MMU (88200) / FPU (88100/88110) as separate parts",
        ],
        EncodeStatus::None("encoder pending — emit text only"),
    ),
    s(
        "ia64",
        "Intel Itanium (EPIC)",
        64,
        Endianness::Little,
        128,
        Some("r0"),
        "r12 (sp)",
        Some("b0 (rp)"),
        0,
        0,
        false,
        &[],
        false,
        true,
        &["registers rotating fr32+ (Itanium 2)"],
        Some("Itanium psABI"),
        &[
            "EPIC: 128-bit bundles, 3 slots + 5-bit template",
            "predication via p0–p63",
            "register stack engine (RSE) for stacked registers",
            "no branch delay slots — stop bits (;;) separate groups",
            "speculative loads (ld.s) and advanced loads (ld.a)",
        ],
        EncodeStatus::None("bundle encoder pending — emit text only"),
    ),
    s(
        "sparc",
        "Sun SPARC",
        32,
        Endianness::Big,
        32,
        Some("%g0"),
        "%sp (%o6)",
        Some("%o7 (call) / %i7 (fp)"),
        1,
        0,
        true,
        &["icc/xcc: z", "n", "c", "v"],
        true,
        false,
        &["VIS (V9)"],
        Some("SysV SPARC o32 / V9"),
        &[
            "register windows with save/restore (V8: 8 windows)",
            "branch delay slot with annul (a) on delayed branches",
            "%g0 hardwired to 0",
            "traps are the exception mechanism (ta/tret)",
            "V8 32-bit; V9 64-bit with %asi address spaces",
        ],
        EncodeStatus::Full,
    ),
    s(
        "i860",
        "Intel i860 (80860)",
        32,
        Endianness::Little,
        32,
        Some("r0"),
        "r30 (SysV; none dedicated in hw)",
        Some("r1 (br link)"),
        1,
        0,
        true,
        &["cc (integer compare)", "fcc (FP compare)"],
        false,
        false,
        &["core/FPU dual pipelines (instruction pairing)"],
        Some("i860 SysV"),
        &[
            "r0 hardwired to 0",
            "load/store use dest register as both base and target",
            "branch delay slot",
            "64-bit FPU registers with 32-bit aliasing",
            "dual-issue core + FPU pipelines — VLIW-style pairing",
        ],
        EncodeStatus::None("encoder pending — emit text only"),
    ),
    s(
        "coldfire",
        "Motorola/Freescale ColdFire (68k family)",
        32,
        Endianness::Big,
        16,
        None,
        "A7 (sp)",
        None,
        0,
        0,
        true,
        &["z", "n", "c", "v", "x"],
        false,
        false,
        &["EMAC (V2+)", "MAC (V1)"],
        Some("m68k SVR4 / ColdFire ABI"),
        &[
            "variable-length instructions (2–10 bytes)",
            "68k-derived but subsets (ISA A/B/C) vary per V1–V5",
            "no zero register — condition codes instead",
            "big-endian",
            "return address on stack — no link register",
            "D0–D7 data + A0–A7 address registers",
        ],
        EncodeStatus::Full,
    ),
    s(
        "x86_64",
        "AMD64 / x86-64",
        64,
        Endianness::Little,
        16,
        None,
        "rsp",
        Some("rbp"),
        0,
        0,
        true,
        &["cf", "pf", "af", "zf", "sf", "of"],
        false,
        false,
        &["SSE", "AVX", "AVX2", "AVX-512"],
        Some("System V AMD64 ABI"),
        &[
            "64-bit extension of x86",
            "16 GPRs (RAX..R15)",
            "RIP-relative addressing",
            "SSE/AVX vector extensions",
            "legacy 16/32-bit modes",
        ],
        EncodeStatus::Full,
    ),
    s(
        "aarch64",
        "ARM64 / AArch64",
        64,
        Endianness::Little,
        31,
        Some("xzr"),
        "sp",
        Some("lr (x30)"),
        0,
        0,
        true,
        &["n", "z", "c", "v"],
        false,
        false,
        &["NEON", "SVE", "SVE2"],
        Some("AAPCS64"),
        &[
            "64-bit ARM architecture",
            "31 GPRs (X0-X30) + SP + ZR",
            "fixed 32-bit instruction encoding",
            "NEON/SVE vector extensions",
            "exception levels EL0-EL3",
        ],
        EncodeStatus::Full,
    ),
    s(
        "superh",
        "Hitachi/Renesas SuperH (SH-1..SH-4)",
        32,
        Endianness::Little,
        16,
        None,
        "r15",
        Some("pr (procedure register)"),
        1,
        0,
        true,
        &["t"],
        false,
        false,
        &["SH-4 DSP (saturating)", "SH-3 FPU (SH-4)"],
        Some("Renesas SH ABI"),
        &[
            "16 GPRs r0–r15; 16-bit halfword instructions (32-bit only in SH-5)",
            "single T flag for all conditions; movt/bt/bf",
            "branch delay slot (rts has one)",
            "pr register for return (rts)",
            "SH-1/2/3/4 flavors; Saturn SH-2 · Dreamcast SH-4",
        ],
        EncodeStatus::Full,
    ),
];

pub fn for_isa(isa: TargetIsa) -> Option<&'static IsaSemantics> {
    PRESERVED_ISAS
        .iter()
        .find(|s| s.name == isa.as_str() || (isa.as_str().starts_with("sh") && s.name == "superh"))
}

pub fn to_json() -> String {
    serde_json::to_string_pretty(PRESERVED_ISAS).expect("catalog serializes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::SuperHFlavor;

    #[test]
    fn eleven_preserved_isas() {
        assert_eq!(PRESERVED_ISAS.len(), 13);
    }

    #[test]
    fn every_preserved_isa_has_unique_name() {
        let mut names: Vec<_> = PRESERVED_ISAS.iter().map(|s| s.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), PRESERVED_ISAS.len());
    }

    #[test]
    fn lookup_by_target() {
        assert_eq!(for_isa(TargetIsa::Alpha).unwrap().word_bits, 64);
        assert_eq!(
            for_isa(TargetIsa::SuperH(SuperHFlavor::Sh4)).unwrap().name,
            "superh"
        );
        assert!(for_isa(TargetIsa::X86_64).is_some());
    }

    #[test]
    fn semantic_facts() {
        let alpha = for_isa(TargetIsa::Alpha).unwrap();
        assert_eq!(alpha.endianness, Endianness::Little);
        assert_eq!(alpha.zero_register, Some("r31"));
        assert!(!alpha.architectural_flags);

        let sparc = for_isa(TargetIsa::Sparc).unwrap();
        assert!(sparc.register_windows);
        assert_eq!(sparc.branch_delay_slots, 1);

        let ia64 = for_isa(TargetIsa::Ia64).unwrap();
        assert!(ia64.predication);
        assert_eq!(ia64.word_bits, 64);

        let superh = for_isa(TargetIsa::SuperH(SuperHFlavor::Sh2)).unwrap();
        assert_eq!(superh.flags, &["t"]);
        assert_eq!(superh.branch_delay_slots, 1);

        let coldfire = for_isa(TargetIsa::ColdFire).unwrap();
        assert_eq!(coldfire.endianness, Endianness::Big);
        assert_eq!(coldfire.gpr_count, 16);
    }

    #[test]
    fn catalog_serializes_to_json() {
        let json = to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 13);
        assert!(json.contains("\"alpha\""));
        assert!(json.contains("\"coldfire\""));
        assert!(json.contains("\"x86_64\""));
        assert!(json.contains("\"aarch64\""));
    }
}
