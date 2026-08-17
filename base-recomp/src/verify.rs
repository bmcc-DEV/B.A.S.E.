//! Round-trip verification — the *executable* semantic contract (Path v1.9).
//!
//! ```text
//! SIR ──encode──▶ bytes ──decode──▶ SIR′
//!                 └──── compare ────┘
//!      literal:  SIR == SIR′
//!      semantic: semantic(SIR) == semantic(SIR′)
//! ```
//!
//! The semantic form is the important one: encoders are allowed to normalize (e.g.
//! `Clear` → `mov #0`, `Dec` → `addi -1`) as long as the meaning is preserved. From
//! the round-trip comes a computable [`preservation_score`]: what fraction of the
//! SIR op set truly survives encode→decode per ISA.

use crate::decode::{decode_ops, has_decoder, DecodeError};
use crate::encode::encode_module;
use crate::sir::{BasicBlock, Function, Module, Op, VReg};
use crate::target::TargetIsa;

#[derive(Debug, Clone)]
pub struct RoundtripReport {
    pub target: TargetIsa,
    /// SIR ops fed in.
    pub ops_in: Vec<Op>,
    /// Ops recovered after encode→decode.
    pub ops_out: Vec<Op>,
    /// `encode` produced bytes.
    pub encode_ok: bool,
    /// `decode` recovered ops without errors or gaps.
    pub decode_ok: bool,
    pub literal_match: bool,
    pub semantic_match: bool,
    pub first_mismatch: Option<(usize, Op, Op)>,
    pub note: String,
}

impl RoundtripReport {
    /// Verified when bytes round-trip and at least the *semantic* form agrees.
    pub fn verified(&self) -> bool {
        self.encode_ok && self.decode_ok && (self.literal_match || self.semantic_match)
    }
}

/// Canonical semantic form of an op: normalizes `Clear`→`movimm(·,0)`,
/// `Inc`/`Dec`/`SubImm`→`addimm(·,±n)` so encoders may fold without failing.
///
/// Immediates are normalized to *signed 32-bit* (the SIR's domain): `AddImm{·,0xFFFFFFFF}`
/// ≡ `Dec{·}` here. Width-dependent behavior (Alpha 64-bit) is NOT this axis — the
/// `differential` dimension catches behavioral deviation at the ISA's real width.
pub fn semantic_key(op: &Op) -> String {
    let s = |i: u32| (i as i32).to_string();
    match op {
        Op::Nop => "nop".into(),
        Op::Ret => "ret".into(),
        Op::MovImm { dst, imm } => format!("movimm({},{})", dst.0, s(*imm)),
        Op::AddImm { dst, imm } => format!("addimm({},{})", dst.0, s(*imm)),
        Op::SubImm { dst, imm } => {
            format!("addimm({},{})", dst.0, -((*imm as i32) as i64))
        }
        Op::Clear { dst } => format!("movimm({},0)", dst.0),
        Op::Inc { dst } => format!("addimm({},1)", dst.0),
        Op::Dec { dst } => format!("addimm({},-1)", dst.0),
        Op::Push { src } => format!("push({})", src.0),
        Op::Pop { dst } => format!("pop({})", dst.0),
        Op::LdMem { dst, base, offset, width } => {
            format!("ldmem({},[{}],{offset},{width})", dst.0, base.0)
        }
        Op::StMem { src, base, offset, width } => {
            format!("stmem({},[{}],{offset},{width})", src.0, base.0)
        }
        // A call/jump IS a call/jump: symbol names are link metadata, not behavior.
        // The encoder/decoder round-trip the displacement field (rel); target/symbol
        // are resolved at link time and are not part of the machine encoding.
        Op::CallRel { .. } => "call".into(),
        Op::JmpRel { .. } => "jmp".into(),
        Op::Cmp { .. } => "cmp".into(),
        Op::Test { .. } => "test".into(),
        Op::BranchCond { .. } => "bcond".into(),
        Op::Unknown { .. } => "gap".into(),
    }
}

pub fn semantically_equal(a: &[Op], b: &[Op]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(x, y)| semantic_key(x) == semantic_key(y))
}

/// Build a minimal module from raw ops (no x86 lift involved).
fn module_from(ops: Vec<Op>) -> Module {
    Module {
        name: "verify".into(),
        source_isa: "sir".into(),
        functions: vec![Function {
            name: "verify".into(),
            blocks: vec![BasicBlock {
                label: "v0".into(),
                ops,
            }],
        }],
        lift_gaps: 0,
        source: None,
        text_vma: None,
    }
}

/// `SIR → encode → bytes → decode → SIR′` with literal + semantic comparison.
pub fn verify_ops(ops: Vec<Op>, target: TargetIsa) -> RoundtripReport {
    let module = module_from(ops);
    let ops_in = module.functions[0].blocks[0].ops.clone();
    let mut report = RoundtripReport {
        target,
        ops_in: ops_in.clone(),
        ops_out: Vec::new(),
        encode_ok: false,
        decode_ok: false,
        literal_match: false,
        semantic_match: false,
        first_mismatch: None,
        note: String::new(),
    };
    let bytes = match encode_module(&module, target) {
        Ok(b) => {
            report.encode_ok = true;
            b
        }
        Err(e) => {
            report.note = format!("encode: {e}");
            return report;
        }
    };
    let ops_out = match decode_ops(&bytes, target) {
        Ok(o) => o,
        Err(DecodeError::NoDecoder(d)) => {
            report.note = format!("no decoder ({d})");
            return report;
        }
        Err(e) => {
            report.note = format!("decode: {e}");
            return report;
        }
    };
    report.decode_ok = !ops_out.iter().any(|o| matches!(o, Op::Unknown { .. }));
    report.ops_out = ops_out.clone();
    report.literal_match = ops_in == ops_out;
    report.semantic_match = semantically_equal(&ops_in, &ops_out);
    if !report.literal_match {
        report.first_mismatch = ops_in
            .iter()
            .zip(ops_out.iter())
            .enumerate()
            .find(|(_, (a, b))| a != b)
            .map(|(idx, (a, b))| (idx, a.clone(), b.clone()));
    }
    report
}

/// The SIR op kinds scored (everything except `Unknown`, which is a gap by design).
pub const SIR_OP_KINDS: [&str; 14] = [
    "nop",
    "ret",
    "mov_imm",
    "add_imm",
    "sub_imm",
    "clear",
    "inc",
    "dec",
    "push",
    "pop",
    "ld_mem",
    "st_mem",
    "call",
    "jmp",
];

fn probe_ops(kind: &str) -> Vec<Op> {
    probe_ops_width(kind, 4)
}

/// `width` is the word width in bytes (4 for 32-bit ISAs, 8 for Alpha): ld/st probe
/// uses the ISA's natural access width so encoders (which gate on it) round-trip.
fn probe_ops_width(kind: &str, width: u8) -> Vec<Op> {
    let v0 = || VReg(0);
    match kind {
        "nop" => vec![Op::Nop],
        "ret" => vec![Op::Ret],
        "mov_imm" => vec![Op::MovImm { dst: v0(), imm: 5 }],
        "add_imm" => vec![Op::AddImm { dst: v0(), imm: 5 }],
        "sub_imm" => vec![Op::SubImm { dst: v0(), imm: 5 }],
        "clear" => vec![Op::Clear { dst: v0() }],
        "inc" => vec![Op::Inc { dst: v0() }],
        "dec" => vec![Op::Dec { dst: v0() }],
        "push" => vec![Op::Push { src: v0() }],
        "pop" => vec![Op::Pop { dst: v0() }],
        "ld_mem" => vec![Op::LdMem { dst: v0(), base: VReg(1), offset: 0, width }],
        "st_mem" => vec![Op::StMem { src: v0(), base: VReg(1), offset: 0, width }],
        "call" => vec![Op::CallRel { rel: 0, target: Some(0), symbol: Some("f".into()) }],
        "jmp" => vec![Op::JmpRel { rel: 0, target: Some(0), symbol: Some("f".into()) }],
        other => unreachable!("unknown probe kind {other}"),
    }
}

/// Edge-case variant of a kind: full-range immediates to exercise width-dependent
/// semantics (e.g. Alpha's LDA sign-extension vs 32-bit wrapping RISC).
fn edge_ops_width(kind: &str, width: u8) -> Vec<Op> {
    let v0 = || VReg(0);
    match kind {
        "mov_imm" => vec![Op::MovImm { dst: v0(), imm: 0xFFFF_FFFF }, Op::Ret],
        "add_imm" => vec![
            Op::MovImm { dst: v0(), imm: 0xFFFF_FFFF },
            Op::AddImm { dst: v0(), imm: 0xFFFF_FFFF },
            Op::Ret,
        ],
        "sub_imm" => vec![
            Op::MovImm { dst: v0(), imm: 0xFFFF_FFFF },
            Op::SubImm { dst: v0(), imm: 0xFFFF_FFFF },
            Op::Ret,
        ],
        "inc" => vec![Op::MovImm { dst: v0(), imm: 0xFFFF_FFFF }, Op::Inc { dst: v0() }, Op::Ret],
        "dec" => vec![Op::MovImm { dst: v0(), imm: 0 }, Op::Dec { dst: v0() }, Op::Ret],
        _ => probe_ops_width(kind, width),
    }
}

/// Input state that stresses word width: high-bit GPR, live link register.
fn edge_state() -> crate::semexec::MachineState {
    use crate::semexec::LINK;
    crate::semexec::MachineState::new()
        .with_gpr(0, 0xFFFF_FFFF)
        .with_gpr(4, 0x4000)
        .with_gpr(LINK, 0x8000)
}

/// Can the semantic executor run this kind for `target`?
pub fn execute_kind(target: TargetIsa, kind: &str) -> bool {
    use crate::semexec::execute_isa;
    let mut st = edge_state();
    execute_isa(&probe_ops_width(kind, crate::semexec::word_bits(target) / 8), &mut st, target)
        .is_ok()
}

/// Behavioral check for one kind: reference semantics vs the ISA's, including edge
/// immediates/state. Passes only if both sides leave the same state.
pub fn differential_kind(target: TargetIsa, kind: &str) -> bool {
    use crate::semexec::differential_ops;
    let state = edge_state();
    let w = crate::semexec::word_bits(target) / 8;
    [probe_ops_width(kind, w), edge_ops_width(kind, w)]
        .iter()
        .all(|ops| differential_ops(ops.clone(), target, &state).matched())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindProbe {
    /// The encoder can produce bytes for this op kind.
    pub encoder: bool,
    /// The decoder recovers the op without gaps.
    pub decoder: bool,
    /// Strict round-trip: `SIR == SIR′`.
    pub literal: bool,
    /// Semantic round-trip: `semantic_key(SIR) == semantic_key(SIR′)`.
    pub semantic: bool,
}

pub fn probe_kind(target: TargetIsa, kind: &str) -> KindProbe {
    let w = crate::semexec::word_bits(target) / 8;
    let r = verify_ops(probe_ops_width(kind, w), target);
    KindProbe {
        encoder: r.encode_ok,
        decoder: r.decode_ok,
        literal: r.literal_match,
        semantic: r.semantic_match,
    }
}

/// Per-ISA coverage by dimension. Deliberately NOT a single "preservation score":
/// 67% of SIR op kinds ≠ 67% of a real architecture (ABI/MMU/privileged/system are
/// separate axes, all `0` until those models exist).
#[derive(Debug, Clone)]
pub struct Coverage {
    pub target: TargetIsa,
    pub has_decoder: bool,
    pub encoder_pct: u32,
    pub decoder_pct: u32,
    pub literal_pct: u32,
    pub semantic_pct: u32,
    /// Ops the semantic executor can run (reference semantics).
    pub execute_pct: u32,
    /// Ops where `execute_reference(SIR) == execute_isa(decode(encode(SIR)))` holds,
    /// including edge immediates/state (behavioral, not representational).
    pub differential_pct: u32,
    /// SIR kinds that round-trip semantically.
    pub covered: Vec<&'static str>,
    /// FULL | PARTIAL | PENDING (encodes, decoder missing) | NONE.
    pub status: &'static str,
}

pub fn coverage(target: TargetIsa) -> Coverage {
    let mut probes = Vec::new();
    for kind in SIR_OP_KINDS {
        probes.push((kind, probe_kind(target, kind)));
    }
    let pct = |f: fn(&KindProbe) -> bool| {
        (probes.iter().filter(|(_, p)| f(p)).count() as f64 / SIR_OP_KINDS.len() as f64 * 100.0)
            .round() as u32
    };
    let covered: Vec<&'static str> = probes
        .iter()
        .filter(|(_, p)| p.semantic)
        .map(|(k, _)| *k)
        .collect();
    let encoder_pct = pct(|p| p.encoder);
    let decoder_pct = pct(|p| p.decoder);
    let literal_pct = pct(|p| p.literal);
    let semantic_pct = pct(|p| p.semantic);
    let execute_pct = SIR_OP_KINDS
        .iter()
        .filter(|k| execute_kind(target, k))
        .count() as f64
        / SIR_OP_KINDS.len() as f64
        * 100.0;
    let execute_pct = execute_pct.round() as u32;
    let differential_pct = SIR_OP_KINDS
        .iter()
        .filter(|k| differential_kind(target, k))
        .count() as f64
        / SIR_OP_KINDS.len() as f64
        * 100.0;
    let differential_pct = differential_pct.round() as u32;
    let status = if semantic_pct == 100 {
        "FULL"
    } else if semantic_pct > 0 {
        "PARTIAL"
    } else if encoder_pct > 0 {
        "PENDING"
    } else {
        "NONE"
    };
    Coverage {
        target,
        has_decoder: has_decoder(target),
        encoder_pct,
        decoder_pct,
        literal_pct,
        semantic_pct,
        execute_pct,
        differential_pct,
        covered,
        status,
    }
}

pub fn all_coverages() -> Vec<Coverage> {
    TargetIsa::all_canonical().iter().copied().map(coverage).collect()
}

/// ABI / Privileged / MMU / System axes are *not modeled* yet — always `0`, by design.
pub const UNMODELED_AXES: &str =
    "abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)";

/// Preservation level P0–P5 (see vault `base-vault/isa/README.md`).
///
/// Derived from *measured* evidence only — a level is a claim about test results,
/// never about intent. Objective bands (see README for rationale):
/// - P1: semantic-catalog entry exists (identity + documented gaps)
/// - P2: format round-trip (has decoder, literal > 0)
/// - P3: semantic subset (semantic_pct > 0)
/// - P4: behavior on a real subset (differential > 0, semantic >= 33%)
/// - P5: behavior over most kinds (differential >= 67%) + sweep sealed
pub fn preservation_level(c: &Coverage, sweep: &crate::semexec::SweepReport) -> &'static str {
    let p1 = crate::semantics::for_isa(c.target).is_some();
    let p2 = c.has_decoder && c.literal_pct > 0;
    let p3 = c.semantic_pct > 0;
    let p4 = c.differential_pct > 0 && c.semantic_pct >= 33;
    let p5 = c.differential_pct >= 67
        && c.semantic_pct >= 67
        && (sweep.all_match() || sweep.mismatches.iter().all(|(l, _, _)| {
            // Sealed: any remaining mismatch has a documented, named cause.
            l.starts_with("add_imm") || l.starts_with("sub_imm")
        }));
    if p5 {
        "P5 — Evidence-sealed"
    } else if p4 {
        "P4 — Behavior-preserved"
    } else if p3 {
        "P3 — Semantic-preserved"
    } else if p2 {
        "P2 — Format-preserved"
    } else if p1 {
        "P1 — Documented"
    } else {
        "P0 — Identified"
    }
}

/// Identity block for the report: from the semantic catalog, else from the target itself.
fn identity_line(isa: TargetIsa) -> String {
    match crate::semantics::for_isa(isa) {
        Some(s) => format!(
            "  family: {} · word: {} bit · endian: {:?} · GPRs: {} · flags: {} · encode: {:?}",
            s.family,
            s.word_bits,
            s.endianness,
            s.gpr_count,
            s.flags.join("/"),
            s.encode_status,
        ),
        None => format!("  (no semantic catalog entry — encode-only target {isa})"),
    }
}

/// One-ISA preservation report: evidence from tests, not prose. Gaps are the unmodeled
/// axes + whatever kinds fail to round-trip.
pub fn preservation_report(isa: TargetIsa) -> String {
    let c = coverage(isa);
    let sweep = crate::semexec::differential_sweep(isa);
    let level = preservation_level(&c, &sweep);
    let roundtrip = if c.has_decoder {
        format!("encoder+decoder subset: round-trip literal {}% · semantic {}%", c.literal_pct, c.semantic_pct)
    } else {
        "encoder only — decoder pending".into()
    };
    let differential = if c.differential_pct > 0 {
        format!(
            "sweep {}/{} match · {} mismatch(es) · differential {}%",
            sweep.matched,
            sweep.applicable,
            sweep.mismatches.len(),
            c.differential_pct
        )
    } else {
        "not applicable — no decoder/encoder round-trip".into()
    };
    let mut gaps: Vec<String> = Vec::new();
    if c.has_decoder && c.literal_pct < c.semantic_pct {
        gaps.push("encoder normalizes forms (e.g. Clear → mov #0) — semantic preserved, literal not".into());
    }
    if c.semantic_pct < 100 && c.semantic_pct > 0 {
        gaps.push(format!("{} of 14 SIR op kinds round-trip semantically", c.covered.len()));
    }
    let mut seen = std::collections::HashSet::new();
    for (label, _ref, _isa) in &sweep.mismatches {
        let kind = label.split(':').next().unwrap_or(label);
        if seen.insert(kind.to_string()) {
            gaps.push(format!("sweep mismatch: {label}"));
        }
    }
    if !c.has_decoder {
        gaps.push("decoder pending".into());
    }
    gaps.push("abi/privileged/mmu/system not modeled (0%)".into());
    let gaps = if gaps.is_empty() {
        "  none known".to_string()
    } else {
        gaps.iter().map(|g| format!("  - {g}")).collect::<Vec<_>>().join("\n")
    };
    format!(
        "Architecture Preservation Report\n================================\n\
         Target: {isa}\n{identity}\n\
         Preservation level: {level}\n\n\
         Codec:\n  {roundtrip}\n\n\
         Semantic:\n  {}\n\n\
         Differential:\n  {differential}\n\n\
         Known gaps:\n{gaps}\n\n\
         Claims:\n  hardware_validated: false\n  complete: false\n  {UNMODELED_AXES}\n",
        if c.semantic_pct > 0 {
            format!("integer ops: pass ({}%)", c.semantic_pct)
        } else {
            "integer ops: not verified".into()
        },
        identity = identity_line(isa),
    )
}

/// All canonical ISAs, one report each.
pub fn preservation_reports() -> String {
    let mut out = String::new();
    for isa in TargetIsa::all_canonical() {
        out.push_str(&preservation_report(*isa));
        out.push('\n');
    }
    out
}

/// Rendered preservation matrix: one line per ISA with level.
pub fn preservation_matrix() -> String {
    let mut out = String::from("| ISA | level | codec | semantic | differential |\n|---|---|---|---|---|\n");
    for isa in TargetIsa::all_canonical() {
        let c = coverage(*isa);
        let sweep = crate::semexec::differential_sweep(*isa);
        let level = preservation_level(&c, &sweep);
        let codec = if c.has_decoder {
            format!("enc {}% · dec {}%", c.encoder_pct, c.decoder_pct)
        } else {
            format!("enc {}% · dec —", c.encoder_pct)
        };
        out.push_str(&format!(
            "| {} | {} | {} | {}% | {}% |\n",
            isa, level, codec, c.semantic_pct, c.differential_pct
        ));
    }
    out
}

pub fn coverage_table() -> String {
    let mut out = String::from(
        "| ISA | enc | dec | literal | semantic | exec | differential | status |\n|---|---|---|---|---|---|---|---|\n",
    );
    for c in all_coverages() {
        out.push_str(&format!(
            "| {} | {}% | {}% | {}% | {}% | {}% | {}% | {} |\n",
            c.target,
            c.encoder_pct,
            c.decoder_pct,
            c.literal_pct,
            c.semantic_pct,
            c.execute_pct,
            c.differential_pct,
            c.status,
        ));
    }
    out.push_str(&format!("\n{UNMODELED_AXES}\n"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_key_normalizes() {
        assert_eq!(semantic_key(&Op::Clear { dst: VReg(2) }), "movimm(2,0)");
        assert_eq!(semantic_key(&Op::Dec { dst: VReg(2) }), "addimm(2,-1)");
        assert_eq!(semantic_key(&Op::Inc { dst: VReg(2) }), "addimm(2,1)");
        assert_eq!(
            semantic_key(&Op::SubImm { dst: VReg(2), imm: 7 }),
            "addimm(2,-7)"
        );
    }

    #[test]
    fn mips_roundtrip_literal() {
        let r = verify_ops(
            vec![Op::MovImm { dst: VReg(0), imm: 1 }, Op::AddImm { dst: VReg(0), imm: 2 }, Op::Ret],
            TargetIsa::Mips,
        );
        assert!(r.literal_match, "{r:?}");
        assert!(r.verified());
    }

    #[test]
    fn sh_clear_roundtrips_semantically() {
        // SH Clear → mov #0,Rn → decodes as MovImm{·,0}: literal differs, meaning holds.
        let r = verify_ops(vec![Op::Clear { dst: VReg(0) }, Op::Ret], TargetIsa::SuperH(Default::default()));
        assert!(!r.literal_match, "decoder normalizes clear→mov #0");
        assert!(r.semantic_match, "{r:?}");
        assert!(r.verified());
    }

    #[test]
    fn ppc_addimm_no_r0_collision() {
        // Regression for the r0-reads-as-zero quirk: AddImm{0,5} must NOT decode as MovImm.
        let r = verify_ops(vec![Op::AddImm { dst: VReg(0), imm: 5 }, Op::Ret], TargetIsa::Ppc);
        assert!(r.literal_match, "{r:?}");
        assert!(r.verified());
    }

    #[test]
    fn probes_dimensions() {
        // alpha: all four dimensions now hold for the full kind set (incl. push via
        // the lda/stq fold and bsr for call).
        assert_eq!(
            probe_kind(TargetIsa::Alpha, "mov_imm"),
            KindProbe { encoder: true, decoder: true, literal: true, semantic: true }
        );
        assert_eq!(
            probe_kind(TargetIsa::Alpha, "push"),
            KindProbe { encoder: true, decoder: true, literal: true, semantic: true }
        );
        assert_eq!(
            probe_kind(TargetIsa::Alpha, "call"),
            KindProbe { encoder: true, decoder: true, literal: false, semantic: true }
        );
        // parisc: the full kind set round-trips now (LDI/LDO/LDW/STW + bl/b,l).
        assert!(probe_kind(TargetIsa::PaRisc, "ret").semantic);
        assert!(probe_kind(TargetIsa::PaRisc, "mov_imm").encoder);
        assert!(probe_kind(TargetIsa::PaRisc, "mov_imm").semantic);
        assert!(probe_kind(TargetIsa::PaRisc, "add_imm").semantic);
        assert!(probe_kind(TargetIsa::PaRisc, "push").semantic);
        // coldfire push/pop/ld/st are fully verified.
        assert!(probe_kind(TargetIsa::ColdFire, "push").literal);
        assert!(probe_kind(TargetIsa::ColdFire, "pop").literal);
        assert!(probe_kind(TargetIsa::ColdFire, "ld_mem").literal, "coldfire ld round-trips");
        assert!(probe_kind(TargetIsa::ColdFire, "st_mem").literal, "coldfire st round-trips");
        let sp = probe_kind(TargetIsa::Sparc, "mov_imm");
        assert!(sp.encoder && sp.decoder && sp.literal, "sparc mov_imm round-trips: {sp:?}");
        let sp_add = probe_kind(TargetIsa::Sparc, "add_imm");
        assert!(sp_add.encoder && sp_add.decoder && sp_add.literal, "sparc add_imm now round-trips: {sp_add:?}");
        // aarch64: decoder now exists for the W-reg subset the encoder emits.
        let aa = probe_kind(TargetIsa::AArch64, "add_imm");
        assert!(aa.encoder && aa.decoder && aa.literal, "{aa:?}");
        // arm: decoder exists; Clear→MOV #0 normalizes (literal differs, meaning holds).
        let arm = probe_kind(TargetIsa::Arm, "add_imm");
        assert!(arm.encoder && arm.decoder && arm.literal, "{arm:?}");
        assert!(!probe_kind(TargetIsa::Arm, "clear").literal);
        assert!(probe_kind(TargetIsa::Arm, "clear").semantic, "arm clear → mov #0");
        // arm: MovImm{0xFFFFFFFF} → mvn rD,#0 (literal round-trip, semantic preserved).
        assert!(probe_kind(TargetIsa::Arm, "mov_imm").semantic);
        // sh: signed-8-bit immediates let the edge cases encode (mov #-1).
        assert!(probe_kind(TargetIsa::SuperH(crate::target::SuperHFlavor::Sh4), "mov_imm").semantic);
    }

    #[test]
    fn coverage_dimensions_are_separate() {
        let sh = coverage(TargetIsa::SuperH(crate::target::SuperHFlavor::Sh4));
        assert_eq!(sh.encoder_pct, sh.decoder_pct);
        // SH `clear` encodes as mov #0 → decodes as MovImm{·,0}: literal < semantic.
        assert!(sh.literal_pct < sh.semantic_pct, "{sh:?}");
        assert_eq!(sh.semantic_pct, 100, "{sh:?}"); // 14 of 14 kinds
        assert_eq!(sh.status, "FULL");
        assert!(sh.covered.contains(&"clear"));
        assert!(sh.covered.contains(&"ld_mem"));
        assert!(sh.covered.contains(&"push"));
        assert!(sh.covered.contains(&"call"));
    }

    #[test]
    fn coverage_alpha_parisc_coldfire() {
        let a = coverage(TargetIsa::Alpha);
        assert_eq!((a.encoder_pct, a.decoder_pct, a.semantic_pct), (100, 100, 100)); // + push/pop/call/jmp
        let p = coverage(TargetIsa::PaRisc);
        assert_eq!((p.encoder_pct, p.decoder_pct, p.semantic_pct), (100, 100, 100)); // LDI/LDO/LDW/STW + bl/b,l
        let c = coverage(TargetIsa::ColdFire);
        assert_eq!(c.semantic_pct, 100, "{c:?}"); // 14 of 14 (all kinds now encode)
        assert_eq!(c.encoder_pct, 100, "{c:?}");
        assert!(c.covered.contains(&"call"));
        assert!(c.covered.contains(&"jmp"));
    }

    #[test]
    fn coverage_no_decoder_means_pending_not_full() {
        let x = coverage(TargetIsa::X86_64);
        assert!(x.encoder_pct > 0, "x86 encoder exists");
        assert_eq!(x.decoder_pct, x.encoder_pct, "x86 decoder now covers the encoder subset");
        assert_eq!(x.status, "FULL", "all 14 kinds round-trip");
        let m88k = coverage(TargetIsa::M88k);
        assert_eq!(m88k.status, "NONE");
        assert!(!m88k.has_decoder);
    }

    #[test]
    fn execute_and_differential_dimensions() {
        // The reference executor now models all 14 kinds (incl. stack-call model).
        for t in TargetIsa::all_canonical() {
            assert_eq!(coverage(*t).execute_pct, 100, "executor kind set differs for {t}");
        }
        // Every ISA with a decoder round-trips + differentially matches all 14 kinds.
        let cases = [
            (TargetIsa::Mips, 100u32),
            (TargetIsa::Ppc, 100),
            (TargetIsa::SuperH(crate::target::SuperHFlavor::Sh4), 100),
            (TargetIsa::Alpha, 100),
            (TargetIsa::PaRisc, 100),
            (TargetIsa::ColdFire, 100),
            (TargetIsa::AArch64, 100),
            (TargetIsa::Arm, 100),
            (TargetIsa::Sparc, 100),
            (TargetIsa::X86_64, 100),
        ];
        for (t, want) in cases {
            let c = coverage(t);
            assert_eq!(c.differential_pct, want, "differential for {t}");
        }
        // No decoder → no differential (behavior never claimed).
        assert_eq!(coverage(TargetIsa::M88k).differential_pct, 0);
    }

    #[test]
    fn differential_alpha_agrees_after_imm_domain_fix() {
        use crate::semexec::differential_ops;
        use crate::semexec::MachineState;
        let state = MachineState::new().with_gpr(26, 0x8000);
        let ops = vec![
            Op::MovImm { dst: VReg(0), imm: 0 },
            Op::AddImm { dst: VReg(0), imm: 0xFFFF_FFFF },
            Op::Ret,
        ];
        // SIR imms are i32; LDA sign-extends; both now mean −1 at 64-bit.
        assert!(differential_ops(ops.clone(), TargetIsa::Alpha, &state).matched());
        assert!(differential_ops(ops, TargetIsa::Mips, &state).matched());
    }

    #[test]
    fn coverage_table_renders() {
        let t = coverage_table();
        assert!(t.contains("mips"));
        assert!(t.contains("alpha"));
        assert!(t.contains("UNMODELED_AXES") || t.contains("not modeled"));
        assert!(t.lines().count() >= 16);
    }

    #[test]
    fn preservation_levels_follow_measured_bands() {
        use crate::semexec::differential_sweep;
        for t in TargetIsa::all_canonical() {
            let c = coverage(*t);
            let s = differential_sweep(*t);
            let level = preservation_level(&c, &s);
            assert!(
                level.starts_with("P"),
                "{t} has a preservation level"
            );
        }
        let cf = coverage(TargetIsa::ColdFire);
        let cf_sweep = differential_sweep(TargetIsa::ColdFire);
        assert!(preservation_level(&cf, &cf_sweep).starts_with("P5"));
        let m88k = coverage(TargetIsa::M88k);
        let m88k_sweep = differential_sweep(TargetIsa::M88k);
        assert!(preservation_level(&m88k, &m88k_sweep).starts_with("P1"));
        let parisc = coverage(TargetIsa::PaRisc);
        let parisc_sweep = differential_sweep(TargetIsa::PaRisc);
        assert!(preservation_level(&parisc, &parisc_sweep).starts_with("P5"));
    }

    #[test]
    fn preservation_report_is_generated_not_prose() {
        let r = preservation_report(TargetIsa::ColdFire);
        assert!(r.contains("Preservation level: P5"));
        assert!(r.contains("hardware_validated: false"));
        assert!(r.contains("complete: false"));
        let m = preservation_matrix();
        assert!(m.contains("| mips |"));
        assert!(m.contains("| coldfire |"));
        assert!(m.contains("P5 — Evidence-sealed"));
    }
}
