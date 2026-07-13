use crate::types::*;
use std::collections::HashMap;

/// Verificador de contratos temporais.
/// Valida se um trace (real ou simulado) satisfaz os contratos BIR.
pub struct ContractVerifier;

impl ContractVerifier {
    /// Verifica todos os contratos de um device contra um trace de eventos
    pub fn verify(
        device: &BirDevice,
        trace: &[TraceSample],
    ) -> ContractVerification {
        let mut passes = Vec::new();
        let mut violations = Vec::new();

        for contract in &device.contracts {
            // Verificar ordem causal
            for order in &contract.must_occur_before {
                let check = Self::check_causal_order(trace, order);
                if !check.passed {
                    violations.push(ContractViolation {
                        contract: format!("{} -> {}", order.event_a, order.event_b),
                        kind: ViolationKind::CausalOrder,
                        expected: format!("{} before {}", order.event_a, order.event_b),
                        actual: check.detail.clone(),
                    });
                }
                passes.push(check);
            }

            // Verificar latências
            for lat in &contract.latency {
                let check = Self::check_latency(trace, lat);
                if !check.passed {
                    violations.push(ContractViolation {
                        contract: format!("latency_{}", lat.event),
                        kind: ViolationKind::LatencyExceeded,
                        expected: format!("{}ns..{}ns", lat.min_ns, lat.max_ns),
                        actual: check.detail.clone(),
                    });
                }
                passes.push(check);
            }

            // Verificar janela
            if let Some(window_ns) = contract.window_ns {
                let check = Self::check_window(trace, window_ns);
                if !check.passed {
                    violations.push(ContractViolation {
                        contract: "window".into(),
                        kind: ViolationKind::WindowExceeded,
                        expected: format!("<= {}ns", window_ns),
                        actual: check.detail.clone(),
                    });
                }
                passes.push(check);
            }
        }

        ContractVerification {
            device: device.name.clone(),
            contracts_checked: passes.len(),
            all_pass: violations.is_empty(),
            passes,
            violations,
        }
    }

    fn check_causal_order(trace: &[TraceSample], order: &CausalOrder) -> ContractCheck {
        let a_times: Vec<u64> = trace.iter()
            .filter(|t| t.event == order.event_a)
            .map(|t| t.timestamp_ns)
            .collect();

        let b_times: Vec<u64> = trace.iter()
            .filter(|t| t.event == order.event_b)
            .map(|t| t.timestamp_ns)
            .collect();

        if a_times.is_empty() || b_times.is_empty() {
            return ContractCheck {
                name: format!("{} -> {}", order.event_a, order.event_b),
                kind: "causal".into(),
                passed: a_times.is_empty() && b_times.is_empty(),
                detail: if a_times.is_empty() { format!("Missing event: {}", order.event_a) } else { format!("Missing event: {}", order.event_b) },
            };
        }

        // Cada A deve ter um B correspondente depois
        let mut failures = 0u32;
        for &a in &a_times {
            let has_b_after = b_times.iter().any(|&b| {
                if let Some(max_delta) = order.max_delta_ns {
                    b > a && (b - a) <= max_delta
                } else {
                    b > a
                }
            });
            if !has_b_after { failures += 1; }
        }

        let passed = failures == 0;
        ContractCheck {
            name: format!("{} -> {}", order.event_a, order.event_b),
            kind: "causal".into(),
            passed,
            detail: if passed { "ok".into() } else { format!("{} events without follower", failures) },
        }
    }

    fn check_latency(trace: &[TraceSample], lat: &BirLatencyConstraint) -> ContractCheck {
        let samples: Vec<u64> = trace.iter()
            .filter(|t| t.event == lat.event)
            .map(|t| t.timestamp_ns)
            .collect();

        if samples.len() < 2 {
            return ContractCheck {
                name: format!("latency_{}", lat.event),
                kind: "latency".into(),
                passed: true,
                detail: "insufficient samples".into(),
            };
        }

        let intervals: Vec<u64> = samples.windows(2).map(|w| w[1] - w[0]).collect();
        let max_interval = intervals.iter().max().copied().unwrap_or(0);
        let min_interval = intervals.iter().min().copied().unwrap_or(0);

        let passed = max_interval <= lat.max_ns && min_interval >= lat.min_ns;

        ContractCheck {
            name: format!("latency_{}", lat.event),
            kind: "latency".into(),
            passed,
            detail: if passed { format!("{}-{}ns ok", min_interval, max_interval) }
                     else { format!("measured {}-{}ns, expected {}-{}ns", min_interval, max_interval, lat.min_ns, lat.max_ns) },
        }
    }

    fn check_window(trace: &[TraceSample], window_ns: u64) -> ContractCheck {
        let total_time = if trace.len() >= 2 {
            trace.last().unwrap().timestamp_ns - trace.first().unwrap().timestamp_ns
        } else { 0 };

        let passed = total_time <= window_ns;
        ContractCheck {
            name: "window".into(),
            kind: "window".into(),
            passed,
            detail: if passed { format!("{}ns <= {}ns", total_time, window_ns) }
                     else { format!("{}ns > {}ns", total_time, window_ns) },
        }
    }
}

/// Amostra de trace para verificação de contrato
#[derive(Debug, Clone)]
pub struct TraceSample {
    pub timestamp_ns: u64,
    pub event: String,
    pub value: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_trace() -> Vec<TraceSample> {
        vec![
            TraceSample { timestamp_ns: 100, event: "DMA_START".into(), value: None },
            TraceSample { timestamp_ns: 200, event: "DMA_BUSY".into(), value: None },
            TraceSample { timestamp_ns: 450, event: "DMA_COMPLETE".into(), value: None },
            TraceSample { timestamp_ns: 500, event: "IRQ_GPU".into(), value: None },
        ]
    }

    #[test]
    fn test_causal_order_ok() {
        let trace = sample_trace();
        let order = CausalOrder {
            event_a: "DMA_START".into(),
            event_b: "DMA_COMPLETE".into(),
            max_delta_ns: Some(1000),
        };
        let check = ContractVerifier::check_causal_order(&trace, &order);
        assert!(check.passed);
    }

    #[test]
    fn test_causal_order_violation() {
        let trace = sample_trace();
        let order = CausalOrder {
            event_a: "DMA_COMPLETE".into(),
            event_b: "DMA_START".into(),
            max_delta_ns: None,
        };
        let check = ContractVerifier::check_causal_order(&trace, &order);
        assert!(!check.passed, "DMA_COMPLETE should not be before DMA_START");
    }

    #[test]
    fn test_latency_ok() {
        let trace = sample_trace();
        let lat = BirLatencyConstraint {
            event: "DMA_BUSY".into(),
            min_ns: 100, max_ns: 500, unit: None,
        };
        let check = ContractVerifier::check_latency(&trace, &lat);
        assert!(check.passed);
    }

    #[test]
    fn test_latency_violation() {
        let mut trace = sample_trace();
        // Add more samples to have measurable intervals
        trace.push(TraceSample { timestamp_ns: 300, event: "DMA_BUSY".into(), value: None });
        let lat = BirLatencyConstraint {
            event: "DMA_BUSY".into(),
            min_ns: 50, max_ns: 80, unit: None,
        };
        let check = ContractVerifier::check_latency(&trace, &lat);
        assert!(!check.passed, "100ns interval > 80ns max");
    }

    #[test]
    fn test_window_ok() {
        let trace = sample_trace();
        let check = ContractVerifier::check_window(&trace, 500);
        assert!(check.passed);
    }

    #[test]
    fn test_full_verification() {
        let trace = sample_trace();
        let mut dev = BirDevice::new("test");
        dev.contracts.push(BirContract {
            must_occur_before: vec![CausalOrder {
                event_a: "DMA_START".into(), event_b: "DMA_COMPLETE".into(),
                max_delta_ns: None,
            }],
            latency: vec![BirLatencyConstraint {
                event: "DMA_BUSY".into(), min_ns: 100, max_ns: 400, unit: None,
            }],
            window_ns: Some(500),
            jitter_ns: None, repetition_rate: None,
        });

        let result = ContractVerifier::verify(&dev, &trace);
        assert!(result.all_pass);
        assert_eq!(result.contracts_checked, 3);
    }
}
