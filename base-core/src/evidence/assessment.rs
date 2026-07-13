/// Evidence Assessment — opinião separada dos fatos.
///
/// Confiança, fontes e justificativas para cada evidência.
/// OPCIONAL — o sistema funciona sem assessment.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentDb {
    pub assessments: Vec<AssessmentEntry>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentEntry {
    pub evidence_id: String,
    pub sources: Vec<Source>,
    pub confidence: f64,
    pub justification: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    pub confidence: f64,
}

impl AssessmentDb {
    pub fn new() -> Self {
        Self {
            assessments: Vec::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    pub fn add(&mut self, entry: AssessmentEntry) {
        self.assessments.push(entry);
    }

    /// Retorna a confiança média para uma evidência específica
    pub fn confidence_for(&self, evidence_id: &str) -> Option<f64> {
        self.assessments.iter()
            .find(|a| a.evidence_id == evidence_id)
            .map(|a| a.confidence)
    }

    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

impl Default for AssessmentDb {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assessment_new() {
        let db = AssessmentDb::new();
        assert!(db.assessments.is_empty());
    }

    #[test]
    fn test_add_assessment() {
        let mut db = AssessmentDb::new();
        db.add(AssessmentEntry {
            evidence_id: "ev_001".into(),
            sources: vec![Source { name: "disassembly".into(), confidence: 0.95 }],
            confidence: 0.95,
            justification: vec!["confirmed_by_disasm".into()],
        });
        assert_eq!(db.assessments.len(), 1);
        assert_eq!(db.confidence_for("ev_001"), Some(0.95));
    }

    #[test]
    fn test_confidence_for_missing() {
        let db = AssessmentDb::new();
        assert_eq!(db.confidence_for("nonexistent"), None);
    }
}
