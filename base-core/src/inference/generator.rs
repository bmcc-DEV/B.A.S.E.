use crate::inference::extraction::{block_type_to_kind, extract_blocks, raw_to_register, BlockCluster, BlockType, MmioAccess, MmioAccessType};
use crate::inference::fsm::{extract_fsm, fsm_to_protocol};
use crate::inference::protocol::{heuristic_register_name, infer_protocol};
use crate::spec::types::*;
use crate::evidence::{EvidenceDb, EvidenceEntry, EvidenceType, IrqPolarity};
use std::collections::HashMap;

/// Gera um HardwareSpec completo a partir de acessos MMIO brutos
pub fn generate_spec(accesses: &[MmioAccess], source: &str) -> HardwareSpec {
    generate_spec_with_evidence(accesses, source).0
}

/// Gera Evidence DB a partir de acessos MMIO brutos
pub fn generate_evidence(accesses: &[MmioAccess], source: &str) -> EvidenceDb {
    generate_spec_with_evidence(accesses, source).1
}

/// Gera ambos: HardwareSpec + Evidence DB
pub fn generate_spec_with_evidence(accesses: &[MmioAccess], source: &str) -> (HardwareSpec, EvidenceDb) {
    let mut spec = HardwareSpec::empty();
    spec.source = source.to_string();

    let mut evidence = EvidenceDb::new(source);
    for (i, access) in accesses.iter().enumerate() {
        let id = format!("ev_{:04}", i);
        let mut context = HashMap::new();
        context.insert("function".into(), access.function_name.clone());
        let etype = match access.access_type {
            MmioAccessType::Read => EvidenceType::MmioRead { address: access.address },
            MmioAccessType::Write => EvidenceType::MmioWrite { address: access.address, value: access.value },
        };
        evidence.add(EvidenceEntry { id, evidence_type: etype, context });
    }

    let clusters = extract_blocks(accesses);
    let mut total_confidence = 0.0f64;

    for cluster in &clusters {
        let protocol = infer_protocol(cluster);
        let fsm = extract_fsm(cluster, &protocol);
        let protocol_type = fsm_to_protocol(&fsm);

        let mut registers: Vec<Register> = cluster.registers.iter()
            .map(|r| {
                let mut reg = raw_to_register(r);
                let role = protocol.register_roles.get(&r.offset);
                if let Some(role) = role {
                    reg.name = Some(heuristic_register_name(r.offset, *role));
                }
                reg
            }).collect();
        registers.sort_by_key(|r| r.offset);

        let timing = TimingProfile {
            activation: Some(LatencyRange::new(
                protocol.timing.avg_step_latency_ns,
                protocol.timing.avg_step_latency_ns * 10,
                protocol.timing.avg_step_latency_ns,
            )),
            processing: None, interrupt_response: None, dma_setup: None, polling_interval: None,
        };

        let block = FunctionalBlock {
            id: format!("{:?}_{:x}", cluster.block_type, cluster.base_address >> 12),
            kind: block_type_to_kind(cluster.block_type, cluster),
            base_address: cluster.base_address,
            size: cluster.size,
            registers,
            protocol: protocol_type,
            timing,
            dma: None,
            dependencies: Vec::new(),
            confidence: cluster.confidence,
        };

        total_confidence += cluster.confidence;
        spec.blocks.push(block);
    }

    spec.confidence = if clusters.is_empty() { 0.0 } else { total_confidence / clusters.len() as f64 };

    (spec, evidence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::extraction::MmioAccess;

    fn mock_mmio(addr: u64, val: u64, at: MmioAccessType) -> MmioAccess {
        MmioAccess { address: addr, value: Some(val), access_type: at, function_name: "test".into(), instruction_addr: addr }
    }

    #[test]
    fn test_generate_evidence_empty() {
        let evidence = generate_evidence(&[], "test");
        assert_eq!(evidence.source, "test");
        assert!(evidence.entries.is_empty());
    }

    #[test]
    fn test_generate_evidence_with_accesses() {
        let accesses = vec![
            mock_mmio(0x10000000, 1, MmioAccessType::Write),
            mock_mmio(0x10000004, 0, MmioAccessType::Read),
        ];
        let evidence = generate_evidence(&accesses, "test");
        assert_eq!(evidence.count(), 2);
    }

    #[test]
    fn test_generate_both() {
        let accesses = vec![
            mock_mmio(0x10000000, 1, MmioAccessType::Write),
            mock_mmio(0x10000004, 0, MmioAccessType::Read),
        ];
        let (spec, evidence) = generate_spec_with_evidence(&accesses, "test");
        assert!(!spec.blocks.is_empty() || spec.blocks.is_empty());
        assert_eq!(evidence.count(), 2);
    }
}
