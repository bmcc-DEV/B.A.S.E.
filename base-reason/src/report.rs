//! Reasoning report — JSON/markdown with honesty gates.

use crate::belief::BeliefGraph;
use crate::hypothesis::HypothesisSet;
use crate::question::Question;
use crate::triad::TriadResult;
use base_core::honesty;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasonReport {
    pub questions: Vec<Question>,
    pub hypotheses: HypothesisSet,
    pub hypothesis_pct: Vec<(String, u32)>,
    pub triad: TriadResult,
    pub belief_node_count: usize,
    pub belief_edge_count: usize,
    pub generates_os: bool,
    pub auto_fix_complete: bool,
    pub honesty: String,
    pub note: String,
}

impl ReasonReport {
    pub fn from_parts(
        questions: Vec<Question>,
        hypotheses: HypothesisSet,
        triad: TriadResult,
        beliefs: &BeliefGraph,
    ) -> Self {
        let hypothesis_pct = hypotheses.percentages();
        Self {
            belief_node_count: beliefs.nodes.len(),
            belief_edge_count: beliefs.edges.len(),
            questions,
            hypotheses,
            hypothesis_pct,
            triad,
            generates_os: honesty::GENERATES_OS,
            auto_fix_complete: honesty::AUTO_FIX_COMPLETE,
            honesty: honesty::NOTE.to_string(),
            note: "RE reasoning assist — ≠ OS turnkey · ≠ auto flash · ≠ Transformer".into(),
        }
    }

    pub fn to_json_pretty(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# B.A.S.E. Reason Report\n\n");
        md.push_str(&honesty::markdown_section());
        md.push_str("\n## Questions (QRM)\n\n");
        if self.questions.is_empty() {
            md.push_str("- (none open)\n");
        } else {
            for q in &self.questions {
                md.push_str(&format!("- **{:?}** `{}`: {}\n", q.kind, q.subject, q.prompt));
            }
        }
        md.push_str("\n## Hypotheses\n\n");
        for (label, pct) in &self.hypothesis_pct {
            md.push_str(&format!("- {label}: {pct}%\n"));
        }
        md.push_str("\n## Triad Gate\n\n");
        md.push_str(&format!(
            "- verdict: **{:?}** · truth={} · coherence={} · causality={}\n",
            self.triad.verdict, self.triad.truth, self.triad.coherence, self.triad.causality
        ));
        for n in &self.triad.notes {
            md.push_str(&format!("- note: {n}\n"));
        }
        md.push_str(&format!(
            "\n## Belief graph\n\n- nodes: {} · edges: {}\n",
            self.belief_node_count, self.belief_edge_count
        ));
        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triad::{TriadGate, TriadVerdict};

    #[test]
    fn report_honesty_false() {
        let r = ReasonReport::from_parts(
            vec![],
            HypothesisSet::new(),
            TriadGate::evaluate(true, true, true),
            &BeliefGraph::new(),
        );
        assert!(!r.generates_os);
        assert!(!r.auto_fix_complete);
        assert_eq!(r.triad.verdict, TriadVerdict::Pass);
    }
}
