/// Trace Replay Engine — executa trace contra contratos e gera relatório de violações.
///
/// Entrada: trace.csv + contracts.yaml
/// Saída: contract_violations.json
use serde::{Deserialize, Serialize};
use crate::temporal::{SequenceContract, TraceEvent, TemporalVerifier, SequenceViolation};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayEngine {
    pub contracts: Vec<SequenceContract>,
}

impl ReplayEngine {
    pub fn new(contracts: Vec<SequenceContract>) -> Self {
        Self { contracts }
    }

    /// Executa replay completo contra um trace
    pub fn replay(&self, events: &[TraceEvent]) -> ReplayResult {
        let report = TemporalVerifier::verify(&self.contracts, events);

        let summary = ReplaySummary {
            total_contracts: self.contracts.len(),
            total_sequences_found: report.total_sequences_found,
            passed: report.passed,
            failed: report.violations.len(),
            pass_rate: if report.total_sequences_found > 0 {
                (report.passed as f64 / report.total_sequences_found as f64 * 100.0) as u32
            } else { 0 },
        };

        ReplayResult {
            summary,
            violations: report.violations,
            contracts: self.contracts.clone(),
        }
    }

    /// Executa replay com diagnóstico detalhado por contrato
    pub fn replay_detailed(&self, events: &[TraceEvent]) -> DetailedReplayResult {
        let mut contract_results = Vec::new();

        for contract in &self.contracts {
            let occurrences = TemporalVerifier::find_patterns(events, &contract.steps);
            let mut contract_violations = Vec::new();

            for seq in &occurrences {
                if contract.order == crate::temporal::OrderConstraint::Strict {
                    // Verificar latência total
                    let total = seq.last().unwrap().timestamp_ns - seq.first().unwrap().timestamp_ns;
                    if total > contract.max_total_ns {
                        contract_violations.push(SequenceViolation {
                            contract: contract.name.clone(),
                            kind: crate::temporal::ViolationKind::TotalLatency,
                            detail: format!("{}ns > {}ns", total, contract.max_total_ns),
                            severity: crate::temporal::Severity::Error,
                        });
                    }
                }

                // Verificar latência entre passos
                for pair in seq.windows(2) {
                    let dt = pair[1].timestamp_ns - pair[0].timestamp_ns;
                    if dt > contract.max_step_ns {
                        contract_violations.push(SequenceViolation {
                            contract: contract.name.clone(),
                            kind: crate::temporal::ViolationKind::StepLatency,
                            detail: format!("step {}ns > {}ns", dt, contract.max_step_ns),
                            severity: crate::temporal::Severity::Warning,
                        });
                    }
                }
            }

            let violations_empty = contract_violations.is_empty();
            contract_results.push(ContractReplayResult {
                contract_name: contract.name.clone(),
                occurrences_found: occurrences.len(),
                violations: contract_violations,
                passed: occurrences.is_empty() || violations_empty,
            });
        }

        let total_found: usize = contract_results.iter().map(|r| r.occurrences_found).sum();
        let total_violations: usize = contract_results.iter().map(|r| r.violations.len()).sum();

        DetailedReplayResult {
            contract_results,
            total_sequences_found: total_found,
            total_violations,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    pub summary: ReplaySummary,
    pub violations: Vec<SequenceViolation>,
    pub contracts: Vec<SequenceContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySummary {
    pub total_contracts: usize,
    pub total_sequences_found: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedReplayResult {
    pub contract_results: Vec<ContractReplayResult>,
    pub total_sequences_found: usize,
    pub total_violations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractReplayResult {
    pub contract_name: String,
    pub occurrences_found: usize,
    pub violations: Vec<SequenceViolation>,
    pub passed: bool,
}

/// Parse de trace CSV no formato Saleae
pub fn parse_saleae_csv(csv: &str) -> Vec<TraceEvent> {
    let mut events = Vec::new();
    for line in csv.lines().skip(1) {  // skip header
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 4 { continue; }
        let time_sec: f64 = parts[0].trim().parse().unwrap_or(0.0);
        let event_type = match parts[2].trim().to_uppercase().as_str() {
            "WRITE" | "W" => "mmio_write",
            "READ" | "R" => "mmio_read",
            "IRQ" | "I" => "irq",
            _ => continue,
        };
        let data = parts[3].trim();
        let (address, value) = parse_data_field(data);
        events.push(TraceEvent {
            timestamp_ns: (time_sec * 1_000_000_000.0) as u64,
            event_type: event_type.to_string(),
            address,
            value,
        });
    }
    events
}

fn parse_data_field(data: &str) -> (u64, Option<u64>) {
    if let Some(eq_pos) = data.find('=') {
        let addr_str = data[..eq_pos].trim();
        let val_str = data[eq_pos + 1..].trim();
        let address = u64::from_str_radix(addr_str.trim_start_matches("0x"), 16).unwrap_or(0);
        let value = u64::from_str_radix(val_str.trim_start_matches("0x"), 16).ok();
        (address, value)
    } else {
        let address = u64::from_str_radix(data.trim_start_matches("0x"), 16).unwrap_or(0);
        (address, None)
    }
}

/// Gera relatório de violações em JSON
pub fn violations_to_json(violations: &[SequenceViolation]) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "replay": {
            "violations": violations.iter().map(|v| serde_json::json!({
                "contract": v.contract,
                "kind": format!("{:?}", v.kind),
                "detail": v.detail,
                "severity": format!("{:?}", v.severity),
            })).collect::<Vec<_>>(),
        }
    })).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::*;

    fn sample_events() -> Vec<TraceEvent> {
        vec![
            TraceEvent { timestamp_ns: 0, event_type: "mmio_write".into(), address: 0xa9bf0000, value: Some(1) },
            TraceEvent { timestamp_ns: 150, event_type: "dma_start".into(), address: 0, value: None },
            TraceEvent { timestamp_ns: 2450, event_type: "dma_complete".into(), address: 0, value: None },
            TraceEvent { timestamp_ns: 2650, event_type: "irq".into(), address: 16, value: None },
        ]
    }

    fn sample_contracts() -> Vec<SequenceContract> {
        vec![SequenceContract {
            name: "dma_xfer".into(),
            steps: vec![
                EventStep { event_type: "mmio_write".into(), address: Some(0xa9bf0000), value: Some(1), tolerance_ns: 50 },
                EventStep { event_type: "dma_start".into(), address: None, value: None, tolerance_ns: 200 },
                EventStep { event_type: "dma_complete".into(), address: None, value: None, tolerance_ns: 200 },
                EventStep { event_type: "irq".into(), address: Some(16), value: None, tolerance_ns: 100 },
            ],
            max_total_ns: 5000, max_step_ns: 3000, order: OrderConstraint::Strict,
        }]
    }

    #[test]
    fn test_replay_no_violations() {
        let engine = ReplayEngine::new(sample_contracts());
        let result = engine.replay(&sample_events());
        assert!(result.summary.pass_rate >= 95, "Should have high pass rate");
    }

    #[test]
    fn test_replay_with_violations() {
        let events = vec![
            TraceEvent { timestamp_ns: 0, event_type: "mmio_write".into(), address: 0xa9bf0000, value: Some(1) },
            TraceEvent { timestamp_ns: 10000, event_type: "dma_start".into(), address: 0, value: None },
            TraceEvent { timestamp_ns: 12000, event_type: "dma_complete".into(), address: 0, value: None },
            TraceEvent { timestamp_ns: 15000, event_type: "irq".into(), address: 16, value: None }, // total 15µs > 5µs
        ];
        let engine = ReplayEngine::new(sample_contracts());
        let result = engine.replay(&events);
        assert!(result.summary.failed > 0, "Should detect violations");
    }

    #[test]
    fn test_detailed_replay() {
        let engine = ReplayEngine::new(sample_contracts());
        let result = engine.replay_detailed(&sample_events());
        assert_eq!(result.contract_results.len(), 1);
        assert!(result.contract_results[0].passed);
    }

    #[test]
    fn test_parse_saleae_csv() {
        let csv = "Time[s],Channel,Type,Data\n0.001,CH0,WRITE,0x10000000=0x01\n0.002,CH0,IRQ,0x10\n";
        let events = parse_saleae_csv(csv);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "mmio_write");
        assert_eq!(events[1].event_type, "irq");
    }

    #[test]
    fn test_violations_to_json() {
        let violations = vec![SequenceViolation {
            contract: "dma".into(),
            kind: crate::temporal::ViolationKind::TotalLatency,
            detail: "10µs > 5µs".into(),
            severity: crate::temporal::Severity::Error,
        }];
        let json = violations_to_json(&violations);
        assert!(json.contains("dma"));
        assert!(json.contains("TotalLatency"));
    }

    #[test]
    fn test_empty_replay() {
        let engine = ReplayEngine::new(vec![]);
        let result = engine.replay(&[]);
        assert_eq!(result.summary.total_contracts, 0);
    }

    #[test]
    fn test_parse_data_field() {
        let (addr, val) = parse_data_field("0x1000=0x01");
        assert_eq!(addr, 0x1000);
        assert_eq!(val, Some(0x01));

        let (addr2, val2) = parse_data_field("0x1000");
        assert_eq!(addr2, 0x1000);
        assert_eq!(val2, None);
    }
}
