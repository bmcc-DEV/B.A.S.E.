//! Discrete hypothesis distribution over causes (no ML).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: String,
    pub label: String,
    /// Relative weight (normalized later); typically mapper/usb scores × 1000
    pub weight: u32,
    pub falsifiable_by: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HypothesisSet {
    pub items: Vec<Hypothesis>,
}

impl HypothesisSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, h: Hypothesis) {
        self.items.push(h);
    }

    pub fn from_labeled_scores(pairs: &[(impl AsRef<str>, u32)]) -> Self {
        let mut s = Self::new();
        for (i, (label, w)) in pairs.iter().enumerate() {
            let label = label.as_ref().to_string();
            s.push(Hypothesis {
                id: format!("h{i}"),
                falsifiable_by: format!("lab_or_trace:{label}"),
                label,
                weight: *w,
            });
        }
        s
    }

    /// Normalize to percentages that sum ~100 (integer).
    pub fn percentages(&self) -> Vec<(String, u32)> {
        let total: u64 = self.items.iter().map(|h| h.weight as u64).sum();
        if total == 0 {
            return self
                .items
                .iter()
                .map(|h| (h.label.clone(), 0))
                .collect();
        }
        self.items
            .iter()
            .map(|h| {
                let pct = ((h.weight as u64 * 100) / total) as u32;
                (h.label.clone(), pct)
            })
            .collect()
    }
}
