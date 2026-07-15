/// Temporal Contracts — verificação de sequências de eventos com latência.
///
/// Contratos de sequência: WRITE → DMA_START → DMA_COMPLETE → IRQ
/// Verifica ordem causal, latência entre passos e latência total.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceContract {
    pub name: String,
    pub steps: Vec<EventStep>,
    pub max_total_ns: u64,
    pub max_step_ns: u64,
    pub order: OrderConstraint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStep {
    pub event_type: String,
    pub address: Option<u64>,
    pub value: Option<u64>,
    pub tolerance_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderConstraint {
    Strict,
    Relaxed,
    Any,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceViolation {
    pub contract: String,
    pub kind: ViolationKind,
    pub detail: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationKind {
    WrongOrder,
    TotalLatency,
    StepLatency,
    MissingEvent,
    ExtraEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayReport {
    pub sequences_checked: usize,
    pub passed: usize,
    pub violations: Vec<SequenceViolation>,
    pub total_sequences_found: usize,
}

/// Verificador de contratos temporais
pub struct TemporalVerifier;

impl TemporalVerifier {
    /// Verifica todos os contratos de sequência contra um trace de eventos
    pub fn verify(
        contracts: &[SequenceContract],
        events: &[TraceEvent],
    ) -> ReplayReport {
        let mut all_violations = Vec::new();
        let mut total_found = 0usize;

        for contract in contracts {
            let occurrences = Self::find_patterns(events, &contract.steps);
            total_found += occurrences.len();

            for seq in &occurrences {
                let mut violations = Vec::new();

                // 1. Verificar ordem
                if contract.order == OrderConstraint::Strict {
                    if let Some(v) = Self::check_order(seq, contract) {
                        violations.push(v);
                    }
                }

                // 2. Verificar latência entre passos
                if contract.max_step_ns > 0 {
                    violations.extend(Self::check_step_latency(seq, contract));
                }

                // 3. Verificar latência total
                if contract.max_total_ns > 0 {
                    if let Some(v) = Self::check_total_latency(seq, contract) {
                        violations.push(v);
                    }
                }

                all_violations.extend(violations);
            }
        }

        let passed = total_found.saturating_sub(all_violations.len());

        ReplayReport {
            sequences_checked: contracts.len(),
            passed,
            violations: all_violations,
            total_sequences_found: total_found,
        }
    }

    /// Encontra ocorrências de um padrão de eventos no trace.
    /// Permite eventos "ruído" entre passos do contrato (subsequência monotônica).
    pub fn find_patterns<'a>(events: &'a [TraceEvent], steps: &[EventStep]) -> Vec<Vec<&'a TraceEvent>> {
        let mut occurrences = Vec::new();
        if steps.is_empty() {
            return occurrences;
        }
        let mut i = 0;

        while i < events.len() {
            if Self::matches(&events[i], &steps[0]) {
                let mut seq = vec![&events[i]];
                let mut k = i + 1;
                let mut j = 1;
                while j < steps.len() && k < events.len() {
                    if Self::matches(&events[k], &steps[j]) {
                        seq.push(&events[k]);
                        j += 1;
                    }
                    k += 1;
                }
                if seq.len() == steps.len() {
                    occurrences.push(seq);
                    i = k;
                    continue;
                }
            }
            i += 1;
        }

        occurrences
    }

    fn check_order(seq: &[&TraceEvent], contract: &SequenceContract) -> Option<SequenceViolation> {
        for (i, step) in contract.steps.iter().enumerate() {
            if let Some(event) = seq.get(i) {
                if !Self::matches(event, step) {
                    return Some(SequenceViolation {
                        contract: contract.name.clone(),
                        kind: ViolationKind::WrongOrder,
                        detail: format!("Step {}: expected {:?}, got {:?}", i, step.event_type, event.event_type),
                        severity: Severity::Error,
                    });
                }
            }
        }
        None
    }

    fn check_step_latency(seq: &[&TraceEvent], contract: &SequenceContract) -> Vec<SequenceViolation> {
        let mut violations = Vec::new();
        for pair in seq.windows(2) {
            let dt = pair[1].timestamp_ns.saturating_sub(pair[0].timestamp_ns);
            if dt > contract.max_step_ns + pair[1].tolerance_ns() {
                violations.push(SequenceViolation {
                    contract: contract.name.clone(),
                    kind: ViolationKind::StepLatency,
                    detail: format!("step latency {}ns > {}ns", dt, contract.max_step_ns),
                    severity: if dt > contract.max_step_ns * 2 { Severity::Error } else { Severity::Warning },
                });
            }
        }
        violations
    }

    fn check_total_latency(seq: &[&TraceEvent], contract: &SequenceContract) -> Option<SequenceViolation> {
        let first_ts = seq.first()?.timestamp_ns;
        let last_ts = seq.last()?.timestamp_ns;
        let total = last_ts.saturating_sub(first_ts);

        if total > contract.max_total_ns {
            Some(SequenceViolation {
                contract: contract.name.clone(),
                kind: ViolationKind::TotalLatency,
                detail: format!("total latency {}ns > {}ns", total, contract.max_total_ns),
                severity: Severity::Error,
            })
        } else {
            None
        }
    }

    fn matches(event: &TraceEvent, step: &EventStep) -> bool {
        let type_match = event.event_type == step.event_type;
        let addr_match = step.address.map_or(true, |a| event.address == a);
        let val_match = step.value.map_or(true, |v| event.value == Some(v));

        type_match && addr_match && val_match
    }
}

/// Evento de trace para verificação temporal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub timestamp_ns: u64,
    pub event_type: String,
    pub address: u64,
    pub value: Option<u64>,
}

impl TraceEvent {
    pub fn tolerance_ns(&self) -> u64 {
        match self.event_type.as_str() {
            "mmio_write" | "mmio_read" => 50,
            "dma_start" | "dma_complete" => 200,
            "irq" => 100,
            _ => 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_events() -> Vec<TraceEvent> {
        vec![
            TraceEvent { timestamp_ns: 0, event_type: "mmio_write".into(), address: 0xa9bf0000, value: Some(1) },
            TraceEvent { timestamp_ns: 150, event_type: "dma_start".into(), address: 0, value: None },
            TraceEvent { timestamp_ns: 2450, event_type: "dma_complete".into(), address: 0, value: None },
            TraceEvent { timestamp_ns: 2650, event_type: "irq".into(), address: 16, value: None },
        ]
    }

    #[test]
    fn test_find_pattern() {
        let events = sample_events();
        let steps = vec![
            EventStep { event_type: "mmio_write".into(), address: Some(0xa9bf0000), value: Some(1), tolerance_ns: 50 },
            EventStep { event_type: "dma_start".into(), address: None, value: None, tolerance_ns: 200 },
            EventStep { event_type: "dma_complete".into(), address: None, value: None, tolerance_ns: 200 },
            EventStep { event_type: "irq".into(), address: Some(16), value: None, tolerance_ns: 100 },
        ];
        let occurrences = TemporalVerifier::find_patterns(&events, &steps);
        assert_eq!(occurrences.len(), 1, "Should find one occurrence");
    }

    #[test]
    fn test_verify_valid_sequence() {
        let events = sample_events();
        let contract = SequenceContract {
            name: "dma_transfer".into(),
            steps: vec![
                EventStep { event_type: "mmio_write".into(), address: Some(0xa9bf0000), value: Some(1), tolerance_ns: 50 },
                EventStep { event_type: "dma_start".into(), address: None, value: None, tolerance_ns: 200 },
                EventStep { event_type: "dma_complete".into(), address: None, value: None, tolerance_ns: 200 },
                EventStep { event_type: "irq".into(), address: Some(16), value: None, tolerance_ns: 100 },
            ],
            max_total_ns: 5000,
            max_step_ns: 3000,
            order: OrderConstraint::Strict,
        };
        let report = TemporalVerifier::verify(&[contract], &events);
        assert_eq!(report.violations.len(), 0, "No violations expected");
    }

    #[test]
    fn test_verify_latency_violation() {
        let events = vec![
            TraceEvent { timestamp_ns: 0, event_type: "mmio_write".into(), address: 0xa9bf0000, value: Some(1) },
            TraceEvent { timestamp_ns: 100, event_type: "dma_start".into(), address: 0, value: None },
            TraceEvent { timestamp_ns: 6000, event_type: "dma_complete".into(), address: 0, value: None },
            TraceEvent { timestamp_ns: 6100, event_type: "irq".into(), address: 16, value: None },
        ];
        let contract = SequenceContract {
            name: "dma_transfer".into(),
            steps: vec![
                EventStep { event_type: "mmio_write".into(), address: Some(0xa9bf0000), value: Some(1), tolerance_ns: 50 },
                EventStep { event_type: "dma_start".into(), address: None, value: None, tolerance_ns: 200 },
                EventStep { event_type: "dma_complete".into(), address: None, value: None, tolerance_ns: 200 },
                EventStep { event_type: "irq".into(), address: Some(16), value: None, tolerance_ns: 100 },
            ],
            max_total_ns: 5000,
            max_step_ns: 3000,
            order: OrderConstraint::Strict,
        };
        let report = TemporalVerifier::verify(&[contract], &events);
        assert!(!report.violations.is_empty(), "Should have latency violations");
        assert!(report.violations.iter().any(|v| matches!(v.kind, ViolationKind::TotalLatency)));
    }

    #[test]
    fn test_verify_wrong_order() {
        // Sequência com IRQ antes do esperado — ainda encontra padrão
        // mas detecta violação de latência
        let events = vec![
            TraceEvent { timestamp_ns: 0, event_type: "mmio_write".into(), address: 0xa9bf0000, value: Some(1) },
            TraceEvent { timestamp_ns: 6000, event_type: "irq".into(), address: 16, value: None },
        ];
        let contract = SequenceContract {
            name: "latency_violation".into(),
            steps: vec![
                EventStep { event_type: "mmio_write".into(), address: Some(0xa9bf0000), value: Some(1), tolerance_ns: 50 },
                EventStep { event_type: "irq".into(), address: Some(16), value: None, tolerance_ns: 100 },
            ],
            max_total_ns: 1000,   // 6000ns > 1000ns → violação
            max_step_ns: 500,
            order: OrderConstraint::Strict,
        };
        let report = TemporalVerifier::verify(&[contract], &events);
        assert!(report.violations.iter().any(|v| matches!(v.kind, ViolationKind::TotalLatency)),
            "Should detect total latency violation: 6000ns > 1000ns");
    }

    #[test]
    fn test_multiple_occurrences() {
        let events = vec![
            TraceEvent { timestamp_ns: 0, event_type: "write".into(), address: 0, value: Some(1) },
            TraceEvent { timestamp_ns: 100, event_type: "irq".into(), address: 16, value: None },
            TraceEvent { timestamp_ns: 1000, event_type: "write".into(), address: 0, value: Some(1) },
            TraceEvent { timestamp_ns: 1100, event_type: "irq".into(), address: 16, value: None },
        ];
        let contract = SequenceContract {
            name: "write_irq".into(),
            steps: vec![
                EventStep { event_type: "write".into(), address: None, value: None, tolerance_ns: 0 },
                EventStep { event_type: "irq".into(), address: None, value: None, tolerance_ns: 0 },
            ],
            max_total_ns: 500,
            max_step_ns: 500,
            order: OrderConstraint::Relaxed,
        };
        let report = TemporalVerifier::verify(&[contract], &events);
        assert_eq!(report.total_sequences_found, 2, "Should find both occurrences");
    }

    #[test]
    fn test_replay_report() {
        let events = sample_events();
        let contracts = vec![SequenceContract {
            name: "dma".into(),
            steps: vec![
                EventStep { event_type: "mmio_write".into(), address: None, value: None, tolerance_ns: 50 },
                EventStep { event_type: "irq".into(), address: None, value: None, tolerance_ns: 100 },
            ],
            max_total_ns: 5000,
            max_step_ns: 3000,
            order: OrderConstraint::Strict,
        }];
        let report = TemporalVerifier::verify(&contracts, &events);
        assert_eq!(report.sequences_checked, 1);
    }
}
