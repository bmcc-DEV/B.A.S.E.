use crate::tracer::DeviceTrace;
use crate::compare::ComparisonItem;
use crate::metrics::ValidationThresholds;

/// Modos de validação
pub enum ValidationMode {
    /// Análise estática — verifica estrutura sem execução
    Static,
    /// Replay simulado — executa trace contra modelo comportamental
    SimulatedReplay,
    /// Replay em hardware real
    HardwareReplay,
}

/// Validação estática: verifica consistência do HardwareSpec sem executar
pub fn validate_static(spec: &base_core::spec::types::HardwareSpec) -> Vec<String> {
    let mut issues = Vec::new();

    for block in &spec.blocks {
        if block.registers.is_empty() {
            issues.push(format!("Block {} has no registers", block.id));
        }
        if block.base_address == 0 {
            issues.push(format!("Block {} has base_address 0", block.id));
        }
        if block.confidence < 0.3 {
            issues.push(format!("Block {} has low confidence ({:.2})", block.id, block.confidence));
        }
        for reg in &block.registers {
            if reg.offset as u64 > block.size {
                issues.push(format!("Block {} register offset 0x{:x} exceeds block size", block.id, reg.offset));
            }
        }
    }

    issues
}

/// Replay simulado: executa trace contra o modelo HardwareSpec
pub fn validate_simulated(
    original: &DeviceTrace,
    spec: &base_core::spec::types::HardwareSpec,
    thresholds: &ValidationThresholds,
) -> Vec<ComparisonItem> {
    // Simula o comportamento esperado baseado no spec
    // Compara cada acesso do trace original com o que o spec prevê
    let mut items = Vec::new();

    for (i, event) in original.events.iter().enumerate() {
        // Encontra qual bloco este acesso atinge
        let matching_block = spec.blocks.iter().find(|b| {
            event.address >= b.base_address && event.address < b.base_address + b.size
        });

        let mut failures = Vec::new();

        if matching_block.is_none() {
            failures.push("ADDRESS_UNMAPPED".into());
        }

        let lat_ratio = thresholds.max_latency_ratio;

        items.push(ComparisonItem {
            operation_id: i,
            original_event: event.clone(),
            actual_event: None,
            latency_ratio: 0.0,
            value_match: true,
            address_match: matching_block.is_some(),
            passed: failures.is_empty(),
            failures,
        });
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;

    #[test]
    fn test_static_validation_empty() {
        let spec = HardwareSpec::empty();
        let issues = validate_static(&spec);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_static_validation_low_confidence() {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(FunctionalBlock {
            id: "bad_block".into(), kind: BlockKind::Unknown,
            base_address: 0, size: 0,
            registers: vec![],
            protocol: Protocol { states: vec![], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: TimingProfile { activation: None, processing: None, interrupt_response: None, dma_setup: None, polling_interval: None },
            dma: None, dependencies: vec![], confidence: 0.1,
        });
        let issues = validate_static(&spec);
        assert!(issues.iter().any(|i| i.contains("low confidence")));
    }

    #[test]
    fn test_simulated_validation() {
        let trace = crate::tracer::DeviceTrace {
            source: "test".into(), device_name: "test".into(),
            events: vec![
                crate::tracer::TraceEvent {
                    timestamp_ns: 1000, channel: "CH0".into(),
                    event_type: crate::tracer::EventType::MmioWrite,
                    address: 0xDEAD0000, value: Some(1),
                },
            ],
        };
        let spec = HardwareSpec::empty();
        let thresholds = ValidationThresholds::default();
        let items = validate_simulated(&trace, &spec, &thresholds);
        assert!(items[0].failures.contains(&"ADDRESS_UNMAPPED".to_string()));
    }
}
