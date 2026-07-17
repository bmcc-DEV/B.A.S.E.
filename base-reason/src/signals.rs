//! HW→SW signal adapters (tension / wedge / twin → Questions).

use crate::question::Question;
use base_core::tension::Conclusiveness;
use serde::Deserialize;

/// Bundle of perception signals from Hardware-facing crates.
#[derive(Debug, Clone)]
pub struct ReasonSignals {
    pub p0_missing: Vec<String>,
    pub unresolved_classes: Vec<String>,
    pub twin_misses: Vec<String>,
    pub conclusiveness: Option<Conclusiveness>,
    pub tension_note: Option<String>,
    /// Evidence ids present for triad Truth axis
    pub evidence_ids: Vec<String>,
    /// False if twin/atlas contradiction known
    pub coherent: bool,
    /// True if causal edge exists or claim does not need one
    pub causal_ok: bool,
    /// Optional labeled hypothesis scores (label, weight)
    pub hypothesis_scores: Vec<(String, u32)>,
}

impl Default for ReasonSignals {
    fn default() -> Self {
        Self::new()
    }
}

impl ReasonSignals {
    pub fn new() -> Self {
        Self {
            p0_missing: Vec::new(),
            unresolved_classes: Vec::new(),
            twin_misses: Vec::new(),
            conclusiveness: None,
            tension_note: None,
            evidence_ids: Vec::new(),
            coherent: true,
            causal_ok: true,
            hypothesis_scores: Vec::new(),
        }
    }
}

pub fn questions_from_p0_missing(missing: &[String]) -> Vec<Question> {
    missing.iter().map(|c| Question::missing_p0(c)).collect()
}

pub fn questions_from_unresolved(classes: &[String]) -> Vec<Question> {
    classes
        .iter()
        .map(|c| Question::unresolved_addr(c))
        .collect()
}

pub fn questions_from_twin_miss(blocks: &[String]) -> Vec<Question> {
    blocks.iter().map(|b| Question::twin_miss(b)).collect()
}

pub fn questions_from_inconclusive(note: &str) -> Vec<Question> {
    vec![Question::inconclusive(note)]
}

/// Minimal wedge map shape for YAML load (avoids depending on base-port).
#[derive(Debug, Deserialize)]
struct WedgeYaml {
    #[serde(default)]
    p0_missing: Vec<String>,
    #[serde(default)]
    entries: Vec<WedgeEntryYaml>,
}

#[derive(Debug, Deserialize)]
struct WedgeEntryYaml {
    class: String,
    #[serde(default)]
    source: String,
}

pub fn questions_from_wedge_yaml(yaml: &str) -> anyhow::Result<Vec<Question>> {
    let w: WedgeYaml = serde_yaml::from_str(yaml)?;
    let mut qs = questions_from_p0_missing(&w.p0_missing);
    let unresolved: Vec<String> = w
        .entries
        .iter()
        .filter(|e| e.source == "unresolved" || e.source == "Unresolved")
        .map(|e| e.class.clone())
        .collect();
    qs.extend(questions_from_unresolved(&unresolved));
    Ok(qs)
}

/// Build question list from a full signal bundle.
pub fn questions_from_signals(sig: &ReasonSignals) -> Vec<Question> {
    let mut qs = Vec::new();
    qs.extend(questions_from_p0_missing(&sig.p0_missing));
    qs.extend(questions_from_unresolved(&sig.unresolved_classes));
    qs.extend(questions_from_twin_miss(&sig.twin_misses));
    if matches!(sig.conclusiveness, Some(Conclusiveness::Inconclusive)) {
        let note = sig
            .tension_note
            .clone()
            .unwrap_or_else(|| "tension between 15% and 85%".into());
        qs.extend(questions_from_inconclusive(&note));
    }
    if qs.is_empty() && sig.evidence_ids.is_empty() {
        qs.push(Question::needs_lab("session_without_evidence"));
    }
    qs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p0_missing_generates_questions() {
        let qs = questions_from_p0_missing(&["gic_redistributor".into()]);
        assert_eq!(qs.len(), 1);
        assert!(qs[0].prompt.contains("gic_redistributor"));
    }

    #[test]
    fn wedge_yaml_unresolved() {
        let yaml = r#"
p0_missing: ["ufs"]
entries:
  - class: uart0
    source: unresolved
"#;
        let qs = questions_from_wedge_yaml(yaml).unwrap();
        assert!(qs.len() >= 2);
    }
}
