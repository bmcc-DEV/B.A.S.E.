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
        Op::CallRel { symbol, .. } => format!("call({})", symbol.as_deref().unwrap_or("?")),
        Op::JmpRel { symbol, .. } => format!("jmp({})", symbol.as_deref().unwrap_or("?")),
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
pub const SIR_OP_KINDS: [&str; 12] = [
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
    "call",
    "jmp",
];

fn probe_ops(kind: &str) -> Vec<Op> {
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
        "call" => vec![Op::CallRel { rel: 0, target: Some(0), symbol: Some("f".into()) }],
        "jmp" => vec![Op::JmpRel { rel: 0, target: Some(0), symbol: Some("f".into()) }],
        other => unreachable!("unknown probe kind {other}"),
    }
}

/// Edge-case variant of a kind: full-range immediates to exercise width-dependent
/// semantics (e.g. Alpha's LDA sign-extension vs 32-bit wrapping RISC).
fn edge_ops(kind: &str) -> Vec<Op> {
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
        _ => probe_ops(kind),
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
    execute_isa(&probe_ops(kind), &mut st, target).is_ok()
}

/// Behavioral check for one kind: reference semantics vs the ISA's, including edge
/// immediates/state. Passes only if both sides leave the same state.
pub fn differential_kind(target: TargetIsa, kind: &str) -> bool {
    use crate::semexec::differential_ops;
    let state = edge_state();
    [probe_ops(kind), edge_ops(kind)]
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
    let r = verify_ops(probe_ops(kind), target);
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
        // alpha: encoder+decoder+literal for mov_imm; nothing for push.
        assert_eq!(
            probe_kind(TargetIsa::Alpha, "mov_imm"),
            KindProbe { encoder: true, decoder: true, literal: true, semantic: true }
        );
        assert_eq!(
            probe_kind(TargetIsa::Alpha, "push"),
            KindProbe { encoder: false, decoder: false, literal: false, semantic: false }
        );
        // parisc: only nop/ret round-trip.
        assert!(probe_kind(TargetIsa::PaRisc, "ret").semantic);
        assert!(!probe_kind(TargetIsa::PaRisc, "mov_imm").encoder);
        // coldfire push/pop are fully verified.
        assert!(probe_kind(TargetIsa::ColdFire, "push").literal);
        assert!(probe_kind(TargetIsa::ColdFire, "pop").literal);
        // no decoder → encoder may hold, but nothing round-trips.
        assert!(probe_kind(TargetIsa::Alpha, "add_imm").encoder);
        let sp = probe_kind(TargetIsa::Sparc, "mov_imm");
        assert!(sp.encoder && !sp.semantic, "sparc has no decoder yet");
    }

    #[test]
    fn coverage_dimensions_are_separate() {
        let sh = coverage(TargetIsa::SuperH(crate::target::SuperHFlavor::Sh4));
        assert_eq!(sh.encoder_pct, sh.decoder_pct);
        // SH `clear` encodes as mov #0 → decodes as MovImm{·,0}: literal < semantic.
        assert!(sh.literal_pct < sh.semantic_pct, "{sh:?}");
        assert_eq!(sh.semantic_pct, 67, "{sh:?}");
        assert_eq!(sh.status, "PARTIAL");
        assert!(sh.covered.contains(&"clear"));
    }

    #[test]
    fn coverage_alpha_parisc_coldfire() {
        let a = coverage(TargetIsa::Alpha);
        assert_eq!((a.encoder_pct, a.decoder_pct, a.semantic_pct), (67, 67, 67));
        let p = coverage(TargetIsa::PaRisc);
        assert_eq!((p.encoder_pct, p.decoder_pct, p.semantic_pct), (17, 17, 17));
        let c = coverage(TargetIsa::ColdFire);
        assert_eq!(c.semantic_pct, 83, "{c:?}"); // push/pop extra vs risc subset
        assert!(c.covered.contains(&"push"));
        assert!(c.covered.contains(&"pop"));
    }

    #[test]
    fn coverage_no_decoder_means_pending_not_full() {
        let x = coverage(TargetIsa::X86_64);
        assert!(x.encoder_pct > 0, "x86 encoder exists");
        assert_eq!(x.decoder_pct, 0);
        assert_eq!(x.status, "PENDING");
        let m88k = coverage(TargetIsa::M88k);
        assert_eq!(m88k.status, "NONE");
        assert!(!m88k.has_decoder);
    }

    #[test]
    fn execute_and_differential_dimensions() {
        // The reference executor runs the same 10 kinds (incl. push/pop) for every ISA.
        for t in TargetIsa::all_canonical() {
            assert_eq!(coverage(*t).execute_pct, 83, "executor kind set differs for {t}");
        }
        let cases = [
            (TargetIsa::Mips, 67u32),
            (TargetIsa::Ppc, 67),
            (TargetIsa::SuperH(crate::target::SuperHFlavor::Sh4), 33), // edge 32-bit imms not encodable
            (TargetIsa::Alpha, 50), // LDA sign-extension on negative adds detected
            (TargetIsa::PaRisc, 17),
            (TargetIsa::ColdFire, 83), // + push/pop (only ISA that encodes them)
        ];
        for (t, want) in cases {
            let c = coverage(t);
            assert_eq!(c.differential_pct, want, "differential for {t}");
        }
        // No decoder → no differential (behavior never claimed).
        assert_eq!(coverage(TargetIsa::X86_64).differential_pct, 0);
        assert_eq!(coverage(TargetIsa::M88k).differential_pct, 0);
    }

    #[test]
    fn differential_detects_alpha_sign_extension() {
        use crate::semexec::differential_ops;
        use crate::semexec::MachineState;
        let state = MachineState::new().with_gpr(26, 0x8000);
        let ops = vec![
            Op::MovImm { dst: VReg(0), imm: 0 },
            Op::AddImm { dst: VReg(0), imm: 0xFFFF_FFFF },
            Op::Ret,
        ];
        assert!(!differential_ops(ops.clone(), TargetIsa::Alpha, &state).matched());
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
}
