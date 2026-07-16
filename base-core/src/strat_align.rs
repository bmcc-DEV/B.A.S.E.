//! StratAlign — alinhamento estratigráfico (Paleocomputação Estrutural §7.1).
//!
//! Programação dinâmica que alinha duas sequências fósseis com custos de gap
//! baseados na taxa de erosão / meia-vida de cada fóssil.
//!
//! Fonte: *Anacroclastia e Paleocomputação Estrutural* (maio/2026).
//! Honestidade: algoritmo de assist — ≠ PaleoCLI completo / ≠ descompilação / ≠ auto-fix.

use crate::evidence::{EvidenceDb, EvidenceType};
use serde::{Deserialize, Serialize};

/// Classe de persistência (PDF §11.5 — meia-vida fóssil).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FossilPersistence {
    /// Desaparece cedo sob otimização (gap barato).
    Fragile,
    /// Persiste sob erosão moderada.
    Resilient,
    /// Sobrevive a compiladores antigos/modernos.
    Ancestral,
    /// Pode reaparecer por convergência (não ancestralidade).
    Convergent,
    /// Traço de eras passadas (raro, gap caro).
    Atavic,
}

impl FossilPersistence {
    /// Taxa de erosão ρ ∈ (0,1] — maior = mais frágil.
    pub fn erosion_rate(self) -> f64 {
        match self {
            Self::Fragile => 0.85,
            Self::Resilient => 0.45,
            Self::Ancestral => 0.20,
            Self::Convergent => 0.55,
            Self::Atavic => 0.10,
        }
    }

    /// Custo de abrir gap sobre este fóssil (PDF: gap ∝ erosão inversa dos ancestrais).
    pub fn gap_cost(self) -> f64 {
        // Ancestrais/atávicos: gap caro (não se “perde” fácil no alinhamento).
        // Frágeis: gap barato (podem ter sido erodidos).
        1.0 / self.erosion_rate().max(0.05)
    }

    /// Reward de match exacto.
    pub fn match_reward(self) -> f64 {
        self.gap_cost()
    }
}

/// Fóssil atómico na sequência estratigráfica.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FossilAtom {
    /// Identificador estável (ex.: `mmio:0x40034000:w`, `irq:42`).
    pub id: String,
    pub persistence: FossilPersistence,
    /// Página/região opcional para scoring fuzzy.
    pub region: Option<u64>,
}

impl FossilAtom {
    pub fn new(id: impl Into<String>, persistence: FossilPersistence) -> Self {
        Self {
            id: id.into(),
            persistence,
            region: None,
        }
    }

    pub fn with_region(mut self, region: u64) -> Self {
        self.region = Some(region);
        self
    }
}

/// Sequência fóssil (nuvem ordenada ao longo do estrato temporal/espacial).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FossilSequence {
    pub label: String,
    pub atoms: Vec<FossilAtom>,
    pub stratum_hint: Option<String>,
}

impl FossilSequence {
    pub fn new(label: impl Into<String>, atoms: Vec<FossilAtom>) -> Self {
        Self {
            label: label.into(),
            atoms,
            stratum_hint: None,
        }
    }

    /// Extrai sequência a partir de EvidenceDb (ordem das entradas = estratigrafia observada).
    pub fn from_evidence(db: &EvidenceDb) -> Self {
        let mut atoms = Vec::with_capacity(db.entries.len());
        for e in &db.entries {
            let (id, persistence, region) = match &e.evidence_type {
                EvidenceType::MmioWrite { address, .. } => (
                    format!("mmio:{address:#x}:w"),
                    FossilPersistence::Resilient,
                    Some(address & !0xfff),
                ),
                EvidenceType::MmioRead { address } => (
                    format!("mmio:{address:#x}:r"),
                    FossilPersistence::Fragile,
                    Some(address & !0xfff),
                ),
                EvidenceType::Irq { vector, .. } => (
                    format!("irq:{vector}"),
                    FossilPersistence::Ancestral,
                    None,
                ),
                EvidenceType::Dma {
                    source,
                    destination,
                    ..
                } => (
                    format!("dma:{source:#x}->{destination:#x}"),
                    FossilPersistence::Resilient,
                    Some(source & !0xfff),
                ),
                EvidenceType::GpioToggle { pin, value } => (
                    format!("gpio:{pin}:{value}"),
                    FossilPersistence::Fragile,
                    None,
                ),
                EvidenceType::FunctionCall { from, to } => (
                    format!("call:{from}->{to}"),
                    FossilPersistence::Convergent,
                    None,
                ),
                EvidenceType::SpiTransfer { cs, .. } => (
                    format!("spi:cs{cs}"),
                    FossilPersistence::Resilient,
                    None,
                ),
                EvidenceType::I2cTransfer { device_addr, .. } => (
                    format!("i2c:{device_addr:#x}"),
                    FossilPersistence::Resilient,
                    None,
                ),
            };
            let mut atom = FossilAtom::new(id, persistence);
            if let Some(r) = region {
                atom = atom.with_region(r);
            }
            atoms.push(atom);
        }
        Self {
            label: db.source.clone(),
            atoms,
            stratum_hint: Some("evidence_db".into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignOp {
    Match,
    Mismatch,
    GapA,
    GapB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignStep {
    pub op: AlignOp,
    pub a: Option<FossilAtom>,
    pub b: Option<FossilAtom>,
    pub score_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratAlignResult {
    pub claim: &'static str,
    pub generates_os: bool,
    pub auto_fix_complete: bool,
    pub score: f64,
    pub normalized_similarity: f64,
    /// Tensão bruta T₀ ≈ 1 − similarity (PDF §3.4 / PaleoCLI T = 1 − sim).
    pub raw_tension: f64,
    pub match_count: usize,
    pub mismatch_count: usize,
    pub gap_a_count: usize,
    pub gap_b_count: usize,
    pub path: Vec<AlignStep>,
    pub honesty: &'static str,
}

/// Parâmetros do alinhamento estratigráfico.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratAlignParams {
    pub mismatch_penalty: f64,
    /// Bonus se mesma região MMIO apesar de id distinto (fuzzy).
    pub region_fuzzy_reward: f64,
}

impl Default for StratAlignParams {
    fn default() -> Self {
        Self {
            mismatch_penalty: 1.0,
            region_fuzzy_reward: 0.35,
        }
    }
}

/// Algoritmo StratAlign (Needleman–Wunsch com custos de erosão).
pub struct StratAligner {
    pub params: StratAlignParams,
}

impl Default for StratAligner {
    fn default() -> Self {
        Self {
            params: StratAlignParams::default(),
        }
    }
}

impl StratAligner {
    pub fn new(params: StratAlignParams) -> Self {
        Self { params }
    }

    /// Alinha `a` (referência / estrato X) com `b` (artefato observado).
    pub fn align(&self, a: &FossilSequence, b: &FossilSequence) -> StratAlignResult {
        let n = a.atoms.len();
        let m = b.atoms.len();

        // DP score matrix + traceback
        let mut dp = vec![vec![0.0f64; m + 1]; n + 1];
        let mut bt = vec![vec![AlignOp::Match; m + 1]; n + 1]; // sentinel

        for i in 1..=n {
            let gap = a.atoms[i - 1].persistence.gap_cost();
            dp[i][0] = dp[i - 1][0] - gap;
            bt[i][0] = AlignOp::GapB;
        }
        for j in 1..=m {
            let gap = b.atoms[j - 1].persistence.gap_cost();
            dp[0][j] = dp[0][j - 1] - gap;
            bt[0][j] = AlignOp::GapA;
        }

        for i in 1..=n {
            for j in 1..=m {
                let (s, op_diag) = self.pair_score(&a.atoms[i - 1], &b.atoms[j - 1]);
                let diag = dp[i - 1][j - 1] + s;
                let up = dp[i - 1][j] - a.atoms[i - 1].persistence.gap_cost();
                let left = dp[i][j - 1] - b.atoms[j - 1].persistence.gap_cost();

                let (best, op) = if diag >= up && diag >= left {
                    (diag, op_diag)
                } else if up >= left {
                    (up, AlignOp::GapB)
                } else {
                    (left, AlignOp::GapA)
                };
                dp[i][j] = best;
                bt[i][j] = op;
            }
        }

        // traceback
        let mut path = Vec::new();
        let mut i = n;
        let mut j = m;
        let mut match_count = 0usize;
        let mut mismatch_count = 0usize;
        let mut gap_a_count = 0usize;
        let mut gap_b_count = 0usize;

        while i > 0 || j > 0 {
            let op = if i == 0 {
                AlignOp::GapA
            } else if j == 0 {
                AlignOp::GapB
            } else {
                bt[i][j]
            };
            match op {
                AlignOp::Match | AlignOp::Mismatch => {
                    let (delta, real_op) = self.pair_score(&a.atoms[i - 1], &b.atoms[j - 1]);
                    if real_op == AlignOp::Match {
                        match_count += 1;
                    } else {
                        mismatch_count += 1;
                    }
                    path.push(AlignStep {
                        op: real_op,
                        a: Some(a.atoms[i - 1].clone()),
                        b: Some(b.atoms[j - 1].clone()),
                        score_delta: delta,
                    });
                    i -= 1;
                    j -= 1;
                }
                AlignOp::GapA => {
                    gap_a_count += 1;
                    let atom = b.atoms[j - 1].clone();
                    let delta = -atom.persistence.gap_cost();
                    path.push(AlignStep {
                        op: AlignOp::GapA,
                        a: None,
                        b: Some(atom),
                        score_delta: delta,
                    });
                    j -= 1;
                }
                AlignOp::GapB => {
                    gap_b_count += 1;
                    let atom = a.atoms[i - 1].clone();
                    let delta = -atom.persistence.gap_cost();
                    path.push(AlignStep {
                        op: AlignOp::GapB,
                        a: Some(atom),
                        b: None,
                        score_delta: delta,
                    });
                    i -= 1;
                }
            }
        }
        path.reverse();

        let score = dp[n][m];
        let max_len = n.max(m).max(1) as f64;
        // Similaridade normalizada ∈ [0,1]: matches / max_len (gaps/mismatches reduzem)
        let normalized_similarity =
            (match_count as f64 / max_len).clamp(0.0, 1.0);
        let raw_tension = 1.0 - normalized_similarity;

        StratAlignResult {
            claim: "strat_align_assist",
            generates_os: false,
            auto_fix_complete: false,
            score,
            normalized_similarity,
            raw_tension,
            match_count,
            mismatch_count,
            gap_a_count,
            gap_b_count,
            path,
            honesty: "StratAlign ≠ source recovery; gaps may be erosion or missing stratum X",
        }
    }

    fn pair_score(&self, x: &FossilAtom, y: &FossilAtom) -> (f64, AlignOp) {
        if x.id == y.id {
            let r = x
                .persistence
                .match_reward()
                .max(y.persistence.match_reward());
            return (r, AlignOp::Match);
        }
        // mesma região MMIO → fuzzy match parcial
        if let (Some(rx), Some(ry)) = (x.region, y.region) {
            if rx == ry {
                return (self.params.region_fuzzy_reward, AlignOp::Match);
            }
        }
        (-self.params.mismatch_penalty, AlignOp::Mismatch)
    }
}

impl StratAlignResult {
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# StratAlign — alinhamento estratigráfico\n\n");
        md.push_str(&format!(
            "> score={:.3} · similarity={:.1}% · T0={:.3} · matches={} mismatches={} gaps_a={} gaps_b={}\n\n",
            self.score,
            self.normalized_similarity * 100.0,
            self.raw_tension,
            self.match_count,
            self.mismatch_count,
            self.gap_a_count,
            self.gap_b_count
        ));
        md.push_str("| # | Op | A | B | Δ |\n|---|----|---|---|---|\n");
        for (i, step) in self.path.iter().take(80).enumerate() {
            let a = step
                .a
                .as_ref()
                .map(|f| f.id.as_str())
                .unwrap_or("—");
            let b = step
                .b
                .as_ref()
                .map(|f| f.id.as_str())
                .unwrap_or("—");
            md.push_str(&format!(
                "| {} | {:?} | `{}` | `{}` | {:.2} |\n",
                i + 1,
                step.op,
                a,
                b,
                step.score_delta
            ));
        }
        if self.path.len() > 80 {
            md.push_str(&format!("\n… +{} steps\n", self.path.len() - 80));
        }
        md.push_str("\n## Honesty\n\n");
        md.push_str("- `generates_os: false` · `auto_fix_complete: false`\n");
        md.push_str(&format!("- {}\n", self.honesty));
        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_sequences_high_similarity() {
        let atoms = vec![
            FossilAtom::new("mmio:0x4000:w", FossilPersistence::Resilient).with_region(0x4000),
            FossilAtom::new("irq:12", FossilPersistence::Ancestral),
            FossilAtom::new("mmio:0x4004:r", FossilPersistence::Fragile).with_region(0x4000),
        ];
        let a = FossilSequence::new("ref", atoms.clone());
        let b = FossilSequence::new("obs", atoms);
        let r = StratAligner::default().align(&a, &b);
        assert_eq!(r.match_count, 3);
        assert!(r.normalized_similarity > 0.99);
        assert!(r.raw_tension < 0.01);
        assert!(!r.generates_os);
    }

    #[test]
    fn eroded_fragile_allows_cheap_gap() {
        let a = FossilSequence::new(
            "ancient",
            vec![
                FossilAtom::new("core:init", FossilPersistence::Ancestral),
                FossilAtom::new("noise:nop", FossilPersistence::Fragile),
                FossilAtom::new("core:done", FossilPersistence::Ancestral),
            ],
        );
        let b = FossilSequence::new(
            "modern",
            vec![
                FossilAtom::new("core:init", FossilPersistence::Ancestral),
                FossilAtom::new("core:done", FossilPersistence::Ancestral),
            ],
        );
        let r = StratAligner::default().align(&a, &b);
        assert_eq!(r.match_count, 2);
        assert_eq!(r.gap_b_count, 1); // fragile eroded from modern
        assert!(r.normalized_similarity >= 0.66);
    }

    #[test]
    fn from_evidence_builds_sequence() {
        let mut db = EvidenceDb::new("test.bin");
        db.add(crate::evidence::EvidenceEntry {
            id: "e0".into(),
            evidence_type: EvidenceType::MmioWrite {
                address: 0x4003_4000,
                value: Some(1),
            },
            context: Default::default(),
        });
        db.add(crate::evidence::EvidenceEntry {
            id: "e1".into(),
            evidence_type: EvidenceType::Irq {
                vector: 5,
                polarity: crate::evidence::IrqPolarity::Rising,
            },
            context: Default::default(),
        });
        let seq = FossilSequence::from_evidence(&db);
        assert_eq!(seq.atoms.len(), 2);
        assert!(seq.atoms[0].id.contains("mmio"));
    }
}
