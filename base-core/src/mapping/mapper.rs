use crate::component_db::{ComponentCategory, ComponentDb, ComponentEntry};
use crate::mapping::solver::{check_constraints, extract_constraints};
use crate::spec::types::{
    ComponentAssignment, FunctionalBlock, HardwareSpec, SynthesisConstraints, SynthesizedSpec,
};

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
        self.map_spec_with_budget(spec, None)
    }

    /// Como [`map_spec`], mas respeita teto de BOM (`max_bom_cost` em USD / price_1k).
    pub fn map_spec_with_budget(
        &self,
        spec: &HardwareSpec,
        max_bom_cost: Option<f64>,
    ) -> SynthesizedSpec {
        let mut assignments = Vec::new();
        let mut running_cost = 0.0f64;

        for block in &spec.blocks {
            let best = self.find_best_component_budget(block, spec, max_bom_cost, running_cost);
            if let Some(assignment) = best {
                if let Some(entry) = self.db.by_name(&assignment.component) {
                    if let Some(price) = entry.availability.as_ref().and_then(|a| a.price_1k) {
                        running_cost += price;
                    }
                }
                assignments.push(assignment);
            }
        }

        SynthesizedSpec {
            original: spec.clone(),
            assignments,
            netlist: None,
            constraints: SynthesisConstraints {
                max_bom_cost,
                preferred_manufacturer: None,
                preferred_package: None,
            },
        }
    }

    /// Encontra o melhor componente para um bloco específico
    pub fn find_best_component(
        &self,
        block: &FunctionalBlock,
        spec: &HardwareSpec,
    ) -> Option<ComponentAssignment> {
        self.find_best_component_budget(block, spec, None, 0.0)
    }

    fn find_best_component_budget(
        &self,
        block: &FunctionalBlock,
        spec: &HardwareSpec,
        max_bom_cost: Option<f64>,
        running_cost: f64,
    ) -> Option<ComponentAssignment> {
        let constraints = extract_constraints(block, spec);
        let candidates = self.find_candidates(block);

        let best = candidates
            .iter()
            .filter(|c| {
                if let Some(budget) = max_bom_cost {
                    // Preço ausente ou $0: não entra sob budget (evita "grátis" silencioso)
                    match c.availability.as_ref().and_then(|a| a.price_1k) {
                        Some(price) if price > 0.0 => running_cost + price <= budget,
                        _ => false,
                    }
                } else {
                    true
                }
            })
            .map(|c| check_constraints(c, &constraints))
            .filter(|a| a.match_score > 0.3)
            .max_by(|a, b| {
                // 1) score  2) prefer MCU  3) cheaper
                a.match_score
                    .partial_cmp(&b.match_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        category_preference(a.component.category)
                            .cmp(&category_preference(b.component.category))
                    })
                    .then_with(|| {
                        let pa = a
                            .component
                            .availability
                            .as_ref()
                            .and_then(|x| x.price_1k)
                            .unwrap_or(f64::MAX);
                        let pb = b
                            .component
                            .availability
                            .as_ref()
                            .and_then(|x| x.price_1k)
                            .unwrap_or(f64::MAX);
                        pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
                    })
            });

        best.map(|solved| ComponentAssignment {
            block_id: block.id.clone(),
            component: solved.component.part.clone(),
            interface: solved.interface.clone(),
            config: serde_json::json!({
                "match_score": solved.match_score,
                "constraint_satisfied": solved.constraint_satisfied,
                "category": format!("{:?}", solved.component.category),
                "preference": "mcu_over_fpga_then_price",
            }),
        })
    }

    /// Encontra candidatos no DB para um bloco
    fn find_candidates(&self, block: &FunctionalBlock) -> Vec<&ComponentEntry> {
        match block.kind {
            // Periféricos típicos: MCU primeiro; FPGA só se não houver MCU
            crate::spec::types::BlockKind::Uart
            | crate::spec::types::BlockKind::Spi
            | crate::spec::types::BlockKind::I2c
            | crate::spec::types::BlockKind::Usb
            | crate::spec::types::BlockKind::Timer
            | crate::spec::types::BlockKind::InterruptController => {
                let mcus = self.db.by_category(ComponentCategory::Mcu);
                if !mcus.is_empty() {
                    mcus
                } else {
                    self.db.by_category(ComponentCategory::Fpga)
                }
            }
            crate::spec::types::BlockKind::Gpu
            | crate::spec::types::BlockKind::Dma
            | crate::spec::types::BlockKind::Audio => {
                let mut candidates: Vec<&ComponentEntry> =
                    self.db.by_category(ComponentCategory::Mcu);
                candidates.extend(self.db.by_category(ComponentCategory::Fpga));
                candidates
            }
            crate::spec::types::BlockKind::Ethernet => {
                self.db.by_category(ComponentCategory::Connectivity)
            }
            crate::spec::types::BlockKind::MemoryController => {
                self.db.by_category(ComponentCategory::Memory)
            }
            crate::spec::types::BlockKind::Crypto => self.db.with_peripheral("crypto", 1),
            _ => self.db.by_category(ComponentCategory::Mcu),
        }
    }

    /// Busca um componente específico pelo nome
    pub fn find_by_name(&self, name: &str) -> Option<&ComponentEntry> {
        self.db.by_name(name)
    }
}

fn category_preference(cat: ComponentCategory) -> u8 {
    match cat {
        ComponentCategory::Mcu => 3,
        ComponentCategory::Cpu => 2,
        ComponentCategory::Fpga => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::types::*;
    use std::collections::HashMap;

    fn mock_spec_uart() -> HardwareSpec {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(FunctionalBlock {
            id: "uart_0".into(),
            kind: BlockKind::Uart,
            base_address: 0x40034000,
            size: 0x1000,
            registers: vec![],
            protocol: Protocol {
                states: vec!["idle".into()],
                transitions: vec![],
                entry_condition: None,
                exit_condition: None,
            },
            timing: TimingProfile {
                activation: None,
                processing: None,
                interrupt_response: None,
                dma_setup: None,
                polling_interval: None,
            },
            dma: None,
            dependencies: vec![],
            confidence: 0.8,
        });
        spec
    }

    fn mock_db() -> ComponentDb {
        let mut db = ComponentDb::new();
        db.add_entry(ComponentEntry {
            part: "RP2350A".into(),
            manufacturer: "RPi".into(),
            description: "MCU".into(),
            category: ComponentCategory::Mcu,
            package: Some("QFN-56".into()),
            features: crate::component_db::ComponentFeatures {
                cpu: Some(crate::component_db::CpuFeature {
                    cores: 2,
                    max_mhz: 150,
                    architecture: None,
                }),
                memory: None,
                peripherals: {
                    let mut p = HashMap::new();
                    p.insert("uart".into(), 2);
                    p.insert("dma".into(), 8);
                    p
                },
            },
            timing: None,
            compatible_with: vec![],
            power: None,
            pins: None,
            availability: Some(crate::component_db::Availability {
                status: "production".into(),
                price_1k: Some(1.50),
                distributor: vec![],
            }),
        });
        db.add_entry(ComponentEntry {
            part: "ECP5-12F".into(),
            manufacturer: "Lattice".into(),
            description: "FPGA".into(),
            category: ComponentCategory::Fpga,
            package: Some("CABGA-256".into()),
            features: crate::component_db::ComponentFeatures {
                cpu: Some(crate::component_db::CpuFeature {
                    cores: 0,
                    max_mhz: 0,
                    architecture: None,
                }),
                memory: None,
                peripherals: HashMap::new(),
            },
            timing: None,
            compatible_with: vec![],
            power: None,
            pins: None,
            availability: Some(crate::component_db::Availability {
                status: "production".into(),
                price_1k: Some(25.0),
                distributor: vec![],
            }),
        });
        db
    }

    #[test]
    fn test_mapper_empty_db() {
        let db = ComponentDb::new();
        let mapper = ComponentMapper::new(db);
        let result = mapper.map_spec(&mock_spec_uart());
        assert!(result.assignments.is_empty());
    }

    #[test]
    fn uart_prefers_mcu_over_fpga() {
        let mapper = ComponentMapper::new(mock_db());
        let syn = mapper.map_spec(&mock_spec_uart());
        assert_eq!(syn.assignments.len(), 1);
        assert_eq!(syn.assignments[0].component, "RP2350A");
        assert_eq!(syn.assignments[0].interface, "uart");
    }

    #[test]
    fn budget_excludes_expensive_parts() {
        let mapper = ComponentMapper::new(mock_db());
        // Only FPGA would fit if we forced FPGA — with MCU at $1.50, budget $1 still fails MCU
        // Budget $1.0: RP costs 1.50 → no assignment
        let syn = mapper.map_spec_with_budget(&mock_spec_uart(), Some(1.0));
        assert!(syn.assignments.is_empty());
        // Budget $2: RP2350 fits
        let syn2 = mapper.map_spec_with_budget(&mock_spec_uart(), Some(2.0));
        assert_eq!(syn2.assignments[0].component, "RP2350A");
    }
}
