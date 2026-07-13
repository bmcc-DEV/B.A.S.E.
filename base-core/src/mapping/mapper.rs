use crate::component_db::{ComponentCategory, ComponentDb, ComponentEntry};
use crate::mapping::solver::{check_constraints, extract_constraints};
use crate::spec::types::{ComponentAssignment, FunctionalBlock, HardwareSpec, SynthesisConstraints, SynthesizedSpec};

/// Mapeia blocos lógicos para componentes reais do DB
pub struct ComponentMapper {
    db: ComponentDb,
}

impl ComponentMapper {
    pub fn new(db: ComponentDb) -> Self {
        Self { db }
    }

    /// Mapeia um HardwareSpec completo, encontrando o melhor componente para cada bloco
    pub fn map_spec(&self, spec: &HardwareSpec) -> SynthesizedSpec {
        let mut assignments = Vec::new();

        for block in &spec.blocks {
            let best = self.find_best_component(block, spec);
            if let Some(assignment) = best {
                assignments.push(assignment);
            }
        }

        SynthesizedSpec {
            original: spec.clone(),
            assignments,
            netlist: None,
            constraints: SynthesisConstraints {
                max_bom_cost: None,
                preferred_manufacturer: None,
                preferred_package: None,
            },
        }
    }

    /// Encontra o melhor componente para um bloco específico
    pub fn find_best_component(&self, block: &FunctionalBlock, spec: &HardwareSpec) -> Option<ComponentAssignment> {
        let constraints = extract_constraints(block, spec);
        let candidates = self.find_candidates(block);

        let best = candidates
            .iter()
            .map(|c| check_constraints(c, &constraints))
            .filter(|a| a.match_score > 0.3)
            .max_by(|a, b| a.match_score.partial_cmp(&b.match_score).unwrap_or(std::cmp::Ordering::Equal));

        best.map(|solved| ComponentAssignment {
            block_id: block.id.clone(),
            component: solved.component.part.clone(),
            interface: solved.interface.clone(),
            config: serde_json::json!({
                "match_score": solved.match_score,
                "constraint_satisfied": solved.constraint_satisfied,
            }),
        })
    }

    /// Encontra candidatos no DB para um bloco
    fn find_candidates(&self, block: &FunctionalBlock) -> Vec<&ComponentEntry> {
        match block.kind {
            crate::spec::types::BlockKind::Gpu
            | crate::spec::types::BlockKind::Dma
            | crate::spec::types::BlockKind::Audio
            | crate::spec::types::BlockKind::Spi
            | crate::spec::types::BlockKind::I2c
            | crate::spec::types::BlockKind::Uart
            | crate::spec::types::BlockKind::Usb
            | crate::spec::types::BlockKind::Timer
            | crate::spec::types::BlockKind::InterruptController => {
                // MCUs com DMA podem emular a maioria dos blocos
                let mut candidates: Vec<&ComponentEntry> = self.db.by_category(ComponentCategory::Mcu);
                // Adiciona também CPLDs e FPGAs
                candidates.extend(self.db.by_category(ComponentCategory::Fpga));
                candidates
            }
            crate::spec::types::BlockKind::Ethernet => {
                self.db.by_category(ComponentCategory::Connectivity)
            }
            crate::spec::types::BlockKind::MemoryController => {
                self.db.by_category(ComponentCategory::Memory)
            }
            crate::spec::types::BlockKind::Crypto => {
                // MCUs com crypto accelerator
                self.db.with_peripheral("crypto", 1)
            }
            _ => {
                // Fallback: todos os MCUs
                self.db.by_category(ComponentCategory::Mcu)
            }
        }
    }

    /// Busca um componente específico pelo nome
    pub fn find_by_name(&self, name: &str) -> Option<&ComponentEntry> {
        self.db.by_name(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::types::*;

    fn mock_spec() -> HardwareSpec {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(FunctionalBlock {
            id: "gpu_0".into(),
            kind: BlockKind::Gpu,
            base_address: 0x10000000,
            size: 0x1000,
            registers: vec![],
            protocol: Protocol { states: vec!["idle".into()], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: TimingProfile { activation: None, processing: None, interrupt_response: None, dma_setup: None, polling_interval: None },
            dma: None,
            dependencies: vec![],
            confidence: 0.8,
        });
        spec
    }

    #[test]
    fn test_mapper_empty_db() {
        let db = ComponentDb::new();
        let mapper = ComponentMapper::new(db);
        let spec = mock_spec();
        let result = mapper.map_spec(&spec);
        assert!(result.assignments.is_empty(), "Empty DB should yield no assignments");
    }
}
