use crate::inference::extraction::{block_type_to_kind, extract_blocks, raw_to_register, BlockCluster, BlockType, MmioAccess, MmioAccessType};
use crate::inference::fsm::{extract_fsm, fsm_to_protocol};
use crate::inference::protocol::{heuristic_register_name, infer_protocol};
use crate::spec::types::*;

/// Gera um HardwareSpec completo a partir de acessos MMIO brutos
pub fn generate_spec(
    accesses: &[MmioAccess],
    source: &str,
) -> HardwareSpec {
    let mut spec = HardwareSpec::empty();
    spec.source = source.to_string();

    // Stage 1: Extrair blocos
    let clusters = extract_blocks(accesses);

    // Stage 2-4: Para cada cluster, inferir protocolo, FSM e gerar bloco
    let mut total_confidence = 0.0f64;

    for cluster in &clusters {
        // Stage 2: Protocolo
        let protocol = infer_protocol(cluster);

        // Stage 3: FSM
        let fsm = extract_fsm(cluster, &protocol);
        let protocol_type = fsm_to_protocol(&fsm);

        // Monta registradores com nomes heuristicos
        let mut registers: Vec<Register> = cluster.registers.iter()
            .map(|r| {
                let mut reg = raw_to_register(r);
                let role = protocol.register_roles.get(&r.offset);
                if let Some(role) = role {
                    reg.name = Some(heuristic_register_name(r.offset, *role));
                }
                reg
            })
            .collect();
        registers.sort_by_key(|r| r.offset);

        // Timing
        let timing = TimingProfile {
            activation: Some(LatencyRange::new(
                protocol.timing.avg_step_latency_ns,
                protocol.timing.avg_step_latency_ns * 10,
                protocol.timing.avg_step_latency_ns,
            )),
            processing: None,
            interrupt_response: None,
            dma_setup: None,
            polling_interval: None,
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

    // Confidence global
    spec.confidence = if clusters.is_empty() {
        0.0
    } else {
        total_confidence / clusters.len() as f64
    };

    spec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::extraction::MmioAccess;

    fn mock_mmio_write(addr: u64, val: u64) -> MmioAccess {
        MmioAccess {
            address: addr,
            value: Some(val),
            access_type: MmioAccessType::Write,
            function_name: "test".into(),
            instruction_addr: addr,
        }
    }

    #[test]
    fn test_generate_spec_empty() {
        let spec = generate_spec(&[], "test");
        assert_eq!(spec.source, "test");
        assert!(spec.blocks.is_empty());
    }

    #[test]
    fn test_generate_spec_single_block() {
        let accesses = vec![
            mock_mmio_write(0x10000000, 1),
            mock_mmio_write(0x10000004, 0x20000000),
            mock_mmio_write(0x10000008, 0x1000),
            mock_mmio_write(0x1000000C, 1),
        ];
        let spec = generate_spec(&accesses, "test");
        assert_eq!(spec.blocks.len(), 1, "Should produce one block");
        assert!(spec.confidence > 0.0, "Confidence should be > 0");
    }

    #[test]
    fn test_generate_spec_multi_block() {
        let accesses = vec![
            // GPU block
            mock_mmio_write(0x10000000, 1),
            mock_mmio_write(0x10000004, 0x20000000),
            // Audio block
            mock_mmio_write(0x20000000, 1),
            mock_mmio_write(0x20000004, 44100),
        ];
        let spec = generate_spec(&accesses, "multi");
        assert_eq!(spec.blocks.len(), 2, "Should produce two blocks");
    }

    #[test]
    fn test_yaml_roundtrip() {
        let accesses = vec![
            mock_mmio_write(0x10000000, 1),
            mock_mmio_write(0x10000004, 0x20000000),
        ];
        let spec = generate_spec(&accesses, "roundtrip");

        let yaml = spec.to_yaml().expect("Should serialize to YAML");
        let decoded = HardwareSpec::from_yaml(&yaml).expect("Should deserialize from YAML");

        assert_eq!(decoded.blocks.len(), spec.blocks.len());
        assert_eq!(decoded.blocks[0].base_address, spec.blocks[0].base_address);
    }
}
