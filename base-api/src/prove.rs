//! POST /v1/prove — contratos temporais → SMT proof (pay-as-you-go).

use anyhow::{bail, Context, Result};
use base_core::smt::SmtProver;
use base_core::temporal::SequenceContract;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ProveRequest {
    /// Contratos temporais (YAML SequenceContract list)
    pub contracts_yaml: String,
    #[serde(default)]
    pub label: Option<String>,
    /// Se true, também corre deadlock_free
    #[serde(default)]
    pub deadlock: bool,
}

#[derive(Debug, Serialize)]
pub struct ProveResponse {
    pub id: String,
    pub object: &'static str,
    pub label: Option<String>,
    pub contracts_proved: usize,
    pub contracts_total: usize,
    pub all_satisfied: bool,
    pub backend: String,
    pub results: Vec<ProveContractRow>,
    pub deadlock: Option<DeadlockRow>,
    pub usage: crate::billing::UsageBreakdown,
    pub honesty: ProveHonesty,
}

#[derive(Debug, Serialize)]
pub struct ProveContractRow {
    pub contract: String,
    pub proved: bool,
    pub satisfiable: bool,
    pub backend: String,
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeadlockRow {
    pub proved: bool,
    pub satisfiable: bool,
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProveHonesty {
    pub saas_production: bool,
    pub auto_fix_complete: bool,
    pub note: &'static str,
}

pub struct ProveOutcome {
    pub response_without_usage: ProveDraft,
    pub contracts: u64,
}

#[derive(Debug, Serialize)]
pub struct ProveDraft {
    pub id: String,
    pub object: &'static str,
    pub label: Option<String>,
    pub contracts_proved: usize,
    pub contracts_total: usize,
    pub all_satisfied: bool,
    pub backend: String,
    pub results: Vec<ProveContractRow>,
    pub deadlock: Option<DeadlockRow>,
    pub honesty: ProveHonesty,
}

pub fn run_prove(req: ProveRequest) -> Result<ProveOutcome> {
    let yaml = req.contracts_yaml.trim();
    if yaml.is_empty() {
        bail!("contracts_yaml empty");
    }
    let contracts: Vec<SequenceContract> =
        serde_yaml::from_str(yaml).context("contracts_yaml parse")?;
    if contracts.is_empty() {
        bail!("no contracts in YAML");
    }
    if contracts.len() > 256 {
        bail!("too many contracts (max 256)");
    }

    let report = SmtProver::prove_all(&contracts);
    let results: Vec<ProveContractRow> = report
        .results
        .iter()
        .map(|r| ProveContractRow {
            contract: r.contract.clone(),
            proved: r.proved,
            satisfiable: r.satisfiable,
            backend: format!("{:?}", r.backend),
            model: r.model.clone(),
        })
        .collect();

    let deadlock = if req.deadlock {
        let d = SmtProver::deadlock_free(&contracts);
        Some(DeadlockRow {
            proved: d.proved,
            satisfiable: d.satisfiable,
            model: d.model,
        })
    } else {
        None
    };

    let n = contracts.len() as u64;
    Ok(ProveOutcome {
        contracts: n,
        response_without_usage: ProveDraft {
            id: format!("proof_{}", Uuid::new_v4()),
            object: "prove.result",
            label: req.label,
            contracts_proved: report.contracts_proved,
            contracts_total: contracts.len(),
            all_satisfied: report.all_satisfied,
            backend: format!("{:?}", report.backend),
            results,
            deadlock,
            honesty: ProveHonesty {
                saas_production: false,
                auto_fix_complete: false,
                note: "Formal contracts only — no LLM in path",
            },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prove_minimal() {
        let yaml = r#"
- name: t0
  steps:
    - event_type: mmio_write
      address: 0x1000
      value: 1
      tolerance_ns: 50
  max_total_ns: 1000
  max_step_ns: 500
  order: Strict
"#;
        let out = run_prove(ProveRequest {
            contracts_yaml: yaml.into(),
            label: None,
            deadlock: false,
        })
        .unwrap();
        assert_eq!(out.contracts, 1);
        assert_eq!(out.response_without_usage.contracts_total, 1);
    }
}
