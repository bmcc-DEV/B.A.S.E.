use crate::compare::ComparisonItem;
use std::collections::HashMap;

/// Limiares de validação configuráveis (globais e por tipo de bloco)
#[derive(Debug, Clone)]
pub struct ValidationThresholds {
    pub max_latency_ratio: f64,
    pub min_value_accuracy: f64,
    pub max_missing_interrupts: f64,
    pub min_dma_throughput_ratio: f64,
    pub min_fps_ratio: f64,
    /// Limiares específicos por tipo de operação
    pub per_type: HashMap<String, f64>,
}

impl Default for ValidationThresholds {
    fn default() -> Self {
        let mut per_type = HashMap::new();
        per_type.insert("MmioWrite".into(), 1.5);   // writes must be tight
        per_type.insert("MmioRead".into(), 2.0);
        per_type.insert("Interrupt".into(), 3.0);    // IRQ can be slower
        per_type.insert("DmaStart".into(), 2.5);
        per_type.insert("DmaEnd".into(), 2.5);
        per_type.insert("GpioToggle".into(), 1.2);   // GPIO must be fast

        Self {
            max_latency_ratio: 2.0,
            min_value_accuracy: 0.95,
            max_missing_interrupts: 0.05,
            min_dma_throughput_ratio: 0.8,
            min_fps_ratio: 0.75,
            per_type,
        }
    }
}

impl ValidationThresholds {
    /// Retorna o threshold de latência para um tipo de operação
    pub fn latency_for(&self, event_type: &str) -> f64 {
        self.per_type.get(event_type).copied().unwrap_or(self.max_latency_ratio)
    }
}

/// Métricas agregadas de validação
#[derive(Debug, Clone)]
pub struct ValidationMetrics {
    pub total_operations: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate: f64,
    pub avg_latency_ratio: f64,
    pub value_accuracy: f64,
    pub address_accuracy: f64,
    pub warnings: Vec<String>,
}

/// Agrega métricas a partir de itens de comparação
pub fn aggregate_metrics(items: &[ComparisonItem]) -> ValidationMetrics {
    let total = items.len();
    let passed = items.iter().filter(|i| i.passed).count();
    let failed = total - passed;

    let pass_rate = if total == 0 {
        0.0
    } else {
        passed as f64 / total as f64
    };

    let avg_latency = if items.is_empty() {
        0.0
    } else {
        items.iter().map(|i| i.latency_ratio).sum::<f64>() / items.len() as f64
    };

    let value_match = items.iter().filter(|i| i.value_match).count();
    let value_accuracy = if total == 0 { 1.0 } else { value_match as f64 / total as f64 };

    let address_match = items.iter().filter(|i| i.address_match).count();
    let address_accuracy = if total == 0 { 1.0 } else { address_match as f64 / total as f64 };

    let mut warnings = Vec::new();
    if avg_latency > 1.5 {
        warnings.push(format!("Average latency ratio {:.2}x — consider timing compensation", avg_latency));
    }
    if value_accuracy < 0.9 {
        warnings.push(format!("Value accuracy {:.1}% — check HAL bitfield implementation", value_accuracy * 100.0));
    }
    if pass_rate < 0.8 {
        warnings.push("Pass rate below 80% — review hardware mapping".into());
    }

    ValidationMetrics {
        total_operations: total,
        passed,
        failed,
        pass_rate,
        avg_latency_ratio: avg_latency,
        value_accuracy,
        address_accuracy,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracer::{EventType, TraceEvent};

    fn mock_item(passed: bool, latency: f64, value_match: bool, addr_match: bool) -> ComparisonItem {
        ComparisonItem {
            operation_id: 0,
            original_event: TraceEvent { timestamp_ns: 1000, channel: "CH0".into(), event_type: EventType::MmioWrite, address: 0x10000000, value: Some(1) },
            actual_event: None,
            latency_ratio: latency,
            value_match,
            address_match: addr_match,
            passed,
            failures: if !passed { vec!["FAIL".into()] } else { vec![] },
        }
    }

    #[test]
    fn test_aggregate_empty() {
        let metrics = aggregate_metrics(&[]);
        assert_eq!(metrics.total_operations, 0);
        assert_eq!(metrics.pass_rate, 0.0);
    }

    #[test]
    fn test_aggregate_all_pass() {
        let items = vec![
            mock_item(true, 1.0, true, true),
            mock_item(true, 1.2, true, true),
        ];
        let metrics = aggregate_metrics(&items);
        assert_eq!(metrics.pass_rate, 1.0);
        assert!(metrics.warnings.is_empty());
    }

    #[test]
    fn test_aggregate_mixed() {
        let items = vec![
            mock_item(true, 1.0, true, true),
            mock_item(false, 3.0, false, false),
        ];
        let metrics = aggregate_metrics(&items);
        assert_eq!(metrics.pass_rate, 0.5);
        assert!(!metrics.warnings.is_empty(), "Should have warnings");
    }
}
