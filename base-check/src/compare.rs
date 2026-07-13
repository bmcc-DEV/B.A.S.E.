use crate::aligner::TraceAligner;
use crate::tracer::{DeviceTrace, TraceEvent};
use crate::metrics::ValidationThresholds;
use base_core::spec::types::HardwareSpec;

/// Resultado da comparação de uma operação
#[derive(Debug, Clone)]
pub struct ComparisonItem {
    pub operation_id: usize,
    pub original_event: TraceEvent,
    pub actual_event: Option<TraceEvent>,
    pub latency_ratio: f64,
    pub value_match: bool,
    pub address_match: bool,
    pub passed: bool,
    pub failures: Vec<String>,
}

/// Comparador de operações entre trace original e novo
pub struct OperationComparator;

impl OperationComparator {
    /// Compara dois traces e retorna itens de comparação
    pub fn compare(
        original: &DeviceTrace,
        actual: &DeviceTrace,
        _spec: &HardwareSpec,
        thresholds: &ValidationThresholds,
    ) -> Vec<ComparisonItem> {
        let align_pairs = TraceAligner::align(original, actual, 100_000); // 100us window
        let mut items = Vec::new();

        for (i, (oi, opt_ai)) in align_pairs.iter().enumerate() {
            let orig = &original.events[*oi];
            let act = opt_ai.map(|ai| &actual.events[ai]);
            let mut failures = Vec::new();

            let address_match = act.map_or(false, |a| a.address == orig.address);
            let value_match = match (orig.value, act.and_then(|a| a.value)) {
                (Some(ov), Some(av)) => ov == av,
                (None, _) => true,
                _ => false,
            };

            let latency_ratio = match act {
                Some(a) if a.timestamp_ns > orig.timestamp_ns => {
                    (a.timestamp_ns - orig.timestamp_ns) as f64
                        / orig.timestamp_ns.max(1) as f64
                }
                _ => 0.0,
            };

            if !address_match {
                failures.push("ADDRESS_MISMATCH".into());
            }
            if !value_match {
                failures.push("VALUE_MISMATCH".into());
            }
            if latency_ratio > thresholds.max_latency_ratio {
                failures.push("TIMING_VIOLATION".into());
            }

            items.push(ComparisonItem {
                operation_id: i,
                original_event: orig.clone(),
                actual_event: act.cloned(),
                latency_ratio,
                value_match,
                address_match,
                passed: failures.is_empty(),
                failures,
            });
        }

        items
    }

    /// Alinha dois traces por timestamp, encontrando operações correspondentes
    fn align_traces<'a>(
        original: &'a DeviceTrace,
        actual: &'a DeviceTrace,
    ) -> Vec<(&'a TraceEvent, Option<&'a TraceEvent>)> {
        let mut aligned = Vec::new();

        for orig_event in &original.events {
            // Encontra o evento mais próximo no trace actual
            let best = actual.events.iter()
                .filter(|a| a.event_type == orig_event.event_type)
                .min_by_key(|a| {
                    let diff = if a.timestamp_ns > orig_event.timestamp_ns {
                        a.timestamp_ns - orig_event.timestamp_ns
                    } else {
                        orig_event.timestamp_ns - a.timestamp_ns
                    };
                    diff
                });

            aligned.push((orig_event, best));
        }

        aligned
    }

    /// Agrupa resultados por tipo de operação
    pub fn group_by_type(items: &[ComparisonItem]) -> std::collections::HashMap<String, Vec<&ComparisonItem>> {
        let mut groups: std::collections::HashMap<String, Vec<&ComparisonItem>> = std::collections::HashMap::new();
        for item in items {
            let key = format!("{:?}", item.original_event.event_type);
            groups.entry(key).or_default().push(item);
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracer::{EventType, TraceEvent};

    fn mock_trace(name: &str) -> DeviceTrace {
        DeviceTrace {
            source: name.into(),
            device_name: name.into(),
            events: vec![
                TraceEvent { timestamp_ns: 1000, channel: "CH0".into(), event_type: EventType::MmioWrite, address: 0x10000000, value: Some(1) },
                TraceEvent { timestamp_ns: 2000, channel: "CH0".into(), event_type: EventType::MmioRead, address: 0x10000004, value: None },
                TraceEvent { timestamp_ns: 3000, channel: "CH1".into(), event_type: EventType::Interrupt, address: 16, value: None },
            ],
        }
    }

    #[test]
    fn test_compare_identical() {
        let trace = mock_trace("test");
        let spec = HardwareSpec::empty();
        let thresholds = ValidationThresholds::default();

        let items = OperationComparator::compare(&trace, &trace, &spec, &thresholds);
        assert_eq!(items.len(), 3);
        for item in &items {
            assert!(item.passed, "Identical traces should pass all checks");
        }
    }

    #[test]
    fn test_address_mismatch() {
        let orig = mock_trace("orig");
        let mut actual = mock_trace("actual");
        actual.events[0].address = 0x20000000; // wrong address

        let spec = HardwareSpec::empty();
        let thresholds = ValidationThresholds::default();

        let items = OperationComparator::compare(&orig, &actual, &spec, &thresholds);
        assert!(!items[0].passed, "Address mismatch should fail");
        assert!(items[0].failures.contains(&"ADDRESS_MISMATCH".to_string()));
    }

    #[test]
    fn test_group_by_type() {
        let trace = mock_trace("test");
        let spec = HardwareSpec::empty();
        let thresholds = ValidationThresholds::default();

        let items = OperationComparator::compare(&trace, &trace, &spec, &thresholds);
        let groups = OperationComparator::group_by_type(&items);
        assert!(groups.contains_key("MmioWrite"), "Should have MmioWrite group");
        assert!(groups.contains_key("MmioRead"), "Should have MmioRead group");
        assert!(groups.contains_key("Interrupt"), "Should have Interrupt group");
    }
}
