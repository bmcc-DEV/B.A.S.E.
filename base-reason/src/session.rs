//! Reasoning session — ingest signals, update beliefs, emit report.

use crate::belief::{BeliefGraph, BeliefRelation};
use crate::hypothesis::{Hypothesis, HypothesisSet};
use crate::report::ReasonReport;
use crate::signals::{questions_from_signals, ReasonSignals};
use crate::triad::TriadGate;

#[derive(Debug, Clone, Default)]
pub struct ReasoningSession {
    pub beliefs: BeliefGraph,
    pub hypotheses: HypothesisSet,
}

impl ReasoningSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest Hardware-facing signals → questions, beliefs, triad → report.
    pub fn ingest_signals(&mut self, sig: &ReasonSignals) -> ReasonReport {
        let questions = questions_from_signals(sig);

        for q in &questions {
            let id = format!("{:?}:{}", q.kind, q.subject);
            self.beliefs.upsert_node(&id, &q.prompt, 2_500);
            self.beliefs.upsert_node("open_question", "session has open questions", 3_000);
            self.beliefs.add_edge(&id, "open_question", BeliefRelation::Supports, 1_000);
        }

        for (label, w) in &sig.hypothesis_scores {
            self.hypotheses.push(Hypothesis {
                id: format!("score:{label}"),
                label: label.clone(),
                weight: *w,
                falsifiable_by: format!("trace_or_lab:{label}"),
            });
            self.beliefs.upsert_node(format!("hyp:{label}"), label.clone(), (*w).min(10_000) as u16);
        }

        for eid in &sig.evidence_ids {
            self.beliefs.upsert_node(format!("ev:{eid}"), format!("evidence:{eid}"), 8_000);
            self.beliefs.strengthen(&format!("ev:{eid}"), 500);
        }

        let has_evidence = !sig.evidence_ids.is_empty();
        let triad = TriadGate::evaluate(has_evidence, sig.coherent, sig.causal_ok);

        // Online forget of very weak open questions if evidence arrived
        if has_evidence {
            self.beliefs.forget_weak(500, 200);
        }

        ReasonReport::from_parts(
            questions,
            self.hypotheses.clone(),
            triad,
            &self.beliefs,
        )
    }

    pub fn strengthen(&mut self, node_id: &str, delta: u16) {
        self.beliefs.strengthen(node_id, delta);
    }

    pub fn forget_weak(&mut self, floor: u16, decay: u16) {
        self.beliefs.forget_weak(floor, decay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triad::TriadVerdict;

    #[test]
    fn session_g35_style_p0_missing() {
        let mut s = ReasoningSession::new();
        let mut sig = ReasonSignals::new();
        sig.p0_missing.push("gic_redistributor".into());
        sig.coherent = true;
        sig.causal_ok = true;
        // no evidence → triad blocks closing claims
        let report = s.ingest_signals(&sig);
        assert!(!report.questions.is_empty());
        assert_eq!(report.triad.verdict, TriadVerdict::Block);
        assert!(!report.generates_os);
    }

    #[test]
    fn session_strengthen_forget() {
        let mut s = ReasoningSession::new();
        s.beliefs.upsert_node("a", "test", 100);
        s.strengthen("a", 50);
        assert_eq!(s.beliefs.nodes["a"].confidence, 150);
        s.forget_weak(200, 150);
        assert!(!s.beliefs.nodes.contains_key("a"));
    }
}
