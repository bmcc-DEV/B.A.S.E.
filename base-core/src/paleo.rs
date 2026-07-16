//! Pipeline paleo de 3 estágios (PDF §8) — assist B.A.S.E.
//!
//! 1. Extrator de observáveis Ω  
//! 2. Motor de falsificação (Ψ + StratAlign)  
//! 3. Atlas de coerência (markdown/YAML)
//!
//! ≠ PaleoCLI produto completo · ≠ HMC · ≠ cohomologia · ≠ auto-fix.

use crate::evidence::EvidenceDb;
use crate::spec::types::HardwareSpec;
use crate::strat_align::{FossilSequence, StratAlignResult, StratAligner};
use crate::tension::{TensionMetric, TensionReport};
use serde::{Deserialize, Serialize};

/// Coordenadas de observáveis Ω(p) — adaptação hardware do PDF §4.1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservablesOmega {
    pub block_count: usize,
    pub evidence_count: usize,
    pub unique_mmio: usize,
    pub irq_count: usize,
    pub dma_count: usize,
    pub call_count: usize,
    /// dimCFG proxy: número de blocos.
    pub dim_cfg: usize,
    /// Entropia local proxy: ln(1 + evidence/blocks).
    pub h_local: f64,
}

impl ObservablesOmega {
    pub fn extract(spec: &HardwareSpec, evidence: &EvidenceDb) -> Self {
        let mut irq = 0usize;
        let mut dma = 0usize;
        let mut calls = 0usize;
        for e in &evidence.entries {
            match &e.evidence_type {
                crate::evidence::EvidenceType::Irq { .. } => irq += 1,
                crate::evidence::EvidenceType::Dma { .. } => dma += 1,
                crate::evidence::EvidenceType::FunctionCall { .. } => calls += 1,
                _ => {}
            }
        }
        let blocks = spec.blocks.len().max(1);
        let h_local = ((1.0 + evidence.count() as f64) / blocks as f64).ln().max(0.0);
        Self {
            block_count: spec.blocks.len(),
            evidence_count: evidence.count(),
            unique_mmio: evidence.unique_mmio_addresses().len(),
            irq_count: irq,
            dma_count: dma,
            call_count: calls,
            dim_cfg: spec.blocks.len(),
            h_local,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaleoExcavateResult {
    pub claim: &'static str,
    pub generates_os: bool,
    pub auto_fix_complete: bool,
    pub omega: ObservablesOmega,
    pub tension: TensionReport,
    pub strat_align: Option<StratAlignResult>,
    pub atlas_summary: Vec<String>,
    pub honesty: &'static str,
}

/// Executa o pipeline: Ω → falsificação (Ψ [+ StratAlign]) → atlas.
pub fn excavate(
    evidence: &EvidenceDb,
    spec: &HardwareSpec,
    reference: Option<&EvidenceDb>,
    function_count: usize,
    instruction_count: usize,
    call_edge_count: usize,
) -> PaleoExcavateResult {
    // Estágio 1
    let omega = ObservablesOmega::extract(spec, evidence);

    // Estágio 2 — Ψ
    let tension = TensionMetric::compute(
        evidence,
        spec,
        function_count,
        instruction_count,
        call_edge_count,
    );

    // Estágio 2b — StratAlign vs estrato de referência (opcional)
    let strat_align = reference.map(|ref_db| {
        let a = FossilSequence::from_evidence(ref_db);
        let b = FossilSequence::from_evidence(evidence);
        StratAligner::default().align(&a, &b)
    });

    // Estágio 3 — atlas textual
    let mut atlas_summary = vec![
        format!(
            "Ω: blocks={} mmio_unique={} irq={} h_local={:.3}",
            omega.block_count, omega.unique_mmio, omega.irq_count, omega.h_local
        ),
        format!(
            "Ψ: tension={:.4} confidence={:.1}% {:?}",
            tension.overall_tension, tension.overall_confidence * 100.0, tension.conclusiveness
        ),
        format!(
            "S(B) compilatory_entropy={:.4}",
            tension.compilatory_entropy
        ),
    ];
    if let Some(sa) = &strat_align {
        atlas_summary.push(format!(
            "StratAlign: similarity={:.1}% T0={:.3} matches={}",
            sa.normalized_similarity * 100.0, sa.raw_tension, sa.match_count
        ));
    }
    atlas_summary.push(
        "Axioma: minimizar Ψ ≠ reconstrução única; gaps = erosão ou estrato em falta".into(),
    );

    PaleoExcavateResult {
        claim: "paleo_excavate_assist",
        generates_os: false,
        auto_fix_complete: false,
        omega,
        tension,
        strat_align,
        atlas_summary,
        honesty: "PDF §8 pipeline assist — ≠ PaleoCLI v0.8 product; ≠ source recovery",
    }
}

impl PaleoExcavateResult {
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Paleo Excavate — atlas de coerência\n\n");
        md.push_str("> Nós não descompilamos. Nós escavamos.\n\n");
        md.push_str("## Observáveis Ω\n\n");
        md.push_str(&format!(
            "- blocks: {}\n- evidence: {}\n- unique MMIO: {}\n- irq: {}\n- dma: {}\n- calls: {}\n- h_local: {:.3}\n\n",
            self.omega.block_count,
            self.omega.evidence_count,
            self.omega.unique_mmio,
            self.omega.irq_count,
            self.omega.dma_count,
            self.omega.call_count,
            self.omega.h_local
        ));
        md.push_str("## Falsificação (Ψ)\n\n");
        md.push_str(&format!(
            "- overall_tension: {:.4}\n- confidence: {:.1}%\n- conclusiveness: {:?}\n- compilatory_entropy: {:.4}\n\n",
            self.tension.overall_tension,
            self.tension.overall_confidence * 100.0,
            self.tension.conclusiveness,
            self.tension.compilatory_entropy
        ));
        if let Some(sa) = &self.strat_align {
            md.push_str("## StratAlign\n\n");
            md.push_str(&format!(
                "- similarity: {:.1}%\n- T0: {:.3}\n- score: {:.3}\n\n",
                sa.normalized_similarity * 100.0, sa.raw_tension, sa.score
            ));
        }
        md.push_str("## Atlas\n\n");
        for line in &self.atlas_summary {
            md.push_str(&format!("- {line}\n"));
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
    use crate::evidence::{EvidenceEntry, EvidenceType};
    use crate::spec::types::{
        AccessType, BlockKind, FunctionalBlock, HardwareSpec, Protocol, Register,
        RegisterPurpose, TimingProfile,
    };

    fn tiny_spec() -> HardwareSpec {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(FunctionalBlock {
            id: "b0".into(),
            kind: BlockKind::Uart,
            base_address: 0x4000,
            size: 0x1000,
            registers: vec![Register {
                offset: 0,
                name: Some("dr".into()),
                width: 32,
                access: AccessType::ReadWrite,
                purpose: RegisterPurpose::UnknownPurpose,
                reset_value: None,
                observed_values: vec![],
                bitfields: vec![],
                polling: false,
                count: 0,
            }],
            protocol: Protocol {
                states: vec![],
                transitions: vec![],
                entry_condition: None,
                exit_condition: None,
            },
            timing: TimingProfile {
                activation: None,
                processing: None,
                interrupt_response: None,
                dma_setup: None,
                polling_interval: None,
            },
            dma: None,
            dependencies: vec![],
            confidence: 0.5,
        });
        spec
    }

    #[test]
    fn excavate_runs_without_reference() {
        let mut db = EvidenceDb::new("fw");
        db.add(EvidenceEntry {
            id: "e0".into(),
            evidence_type: EvidenceType::MmioWrite {
                address: 0x4000,
                value: Some(1),
            },
            context: Default::default(),
        });
        let r = excavate(&db, &tiny_spec(), None, 1, 10, 0);
        assert!(!r.generates_os);
        assert!(r.strat_align.is_none());
        assert!(!r.atlas_summary.is_empty());
    }
}
