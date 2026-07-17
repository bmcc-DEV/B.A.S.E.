//! Triad gate: Truth · Coherence · Causality before closing a claim.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriadVerdict {
    /// All three axes pass — claim may proceed as *assist* only (still ≠ OS turnkey).
    Pass,
    /// Block claim: missing evidence, contradiction, or no causal link when required.
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadResult {
    pub truth: bool,
    pub coherence: bool,
    pub causality: bool,
    pub verdict: TriadVerdict,
    pub notes: Vec<String>,
}

pub struct TriadGate;

impl TriadGate {
    /// Evaluate a proposed claim.
    ///
    /// * `has_evidence` — Truth: at least one immutable Evidence fact exists
    /// * `coherent` — Coherence: does not contradict twin/atlas signals
    /// * `has_causal_or_not_required` — Causality: CausalEdge present, or claim does not require it
    pub fn evaluate(has_evidence: bool, coherent: bool, has_causal_or_not_required: bool) -> TriadResult {
        let mut notes = Vec::new();
        if !has_evidence {
            notes.push("Truth fail: no EvidenceEntry for claim".into());
        }
        if !coherent {
            notes.push("Coherence fail: contradicts twin/atlas/session beliefs".into());
        }
        if !has_causal_or_not_required {
            notes.push("Causality fail: no CausalEdge when claim requires temporal cause".into());
        }
        let verdict = if has_evidence && coherent && has_causal_or_not_required {
            TriadVerdict::Pass
        } else {
            TriadVerdict::Block
        };
        TriadResult {
            truth: has_evidence,
            coherence: coherent,
            causality: has_causal_or_not_required,
            verdict,
            notes,
        }
    }

    /// Convenience: block any claim that tries to assert bootable OS / auto-fix.
    pub fn block_os_turnkey_claim() -> TriadResult {
        TriadResult {
            truth: false,
            coherence: false,
            causality: false,
            verdict: TriadVerdict::Block,
            notes: vec![
                "Blocked by honesty: generates_os=false · auto_fix_complete=false".into(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triad_blocks_without_evidence() {
        let r = TriadGate::evaluate(false, true, true);
        assert_eq!(r.verdict, TriadVerdict::Block);
        assert!(!r.truth);
    }

    #[test]
    fn triad_passes_assist_claim() {
        let r = TriadGate::evaluate(true, true, true);
        assert_eq!(r.verdict, TriadVerdict::Pass);
    }

    #[test]
    fn triad_blocks_os_turnkey() {
        let r = TriadGate::block_os_turnkey_claim();
        assert_eq!(r.verdict, TriadVerdict::Block);
    }
}
