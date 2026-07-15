/// Reference Design — saída principal do pipeline.
///
/// YAML descritivo da arquitetura sugerida, derivado do HardwareSpec + component DB +
/// contract solver. NÃO é PCB final — engineering draft para revisão humana.
use serde::{Deserialize, Serialize};

use crate::component_db::{ComponentCategory, ComponentDb};
use crate::mapping::mapper::ComponentMapper;
use crate::solver::{extract_contracts, verify_contract};
use crate::spec::types::{BlockKind, HardwareSpec, SynthesizedSpec};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceDesign {
    pub design: DesignMeta,
    pub architecture: Architecture,
    pub contracts: ContractReport,
    pub bom: BomSummary,
    pub pcb: PcbNote,
    pub validation: ValidationStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assignments: Vec<AssignmentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignMeta {
    pub title: String,
    pub version: u32,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Architecture {
    pub cpu: ComponentChoice,
    pub memory: Vec<ComponentChoice>,
    pub peripherals: Vec<ComponentChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentChoice {
    pub part: String,
    pub interface: Option<String>,
    pub package: Option<String>,
    pub price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractReport {
    pub total: u32,
    pub satisfied: u32,
    pub violations: Vec<ContractViolation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractViolation {
    pub contract: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomSummary {
    pub total_parts: u32,
    pub estimated_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbNote {
    pub pcb_type: String,
    pub layers: u8,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationStatus {
    pub status: String,
    pub contracts_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentSummary {
    pub block_id: String,
    pub block_kind: String,
    pub component: String,
    pub interface: String,
    pub match_score: Option<f64>,
}

impl ReferenceDesign {
    /// Skeleton vazio — preferir [`from_hardware_spec`].
    pub fn new(title: &str, source: &str) -> Self {
        Self {
            design: DesignMeta {
                title: title.to_string(),
                version: 1,
                source: source.to_string(),
            },
            architecture: Architecture {
                cpu: ComponentChoice {
                    part: "unassigned".into(),
                    interface: None,
                    package: None,
                    price: None,
                },
                memory: Vec::new(),
                peripherals: Vec::new(),
            },
            contracts: ContractReport {
                total: 0,
                satisfied: 0,
                violations: Vec::new(),
            },
            bom: BomSummary {
                total_parts: 0,
                estimated_cost: 0.0,
            },
            pcb: PcbNote {
                pcb_type: "engineering_draft".into(),
                layers: 2,
                notes: vec![
                    "Engineering draft — requires layout review".into(),
                    "Power tree not included".into(),
                ],
            },
            validation: ValidationStatus {
                status: "pending".into(),
                contracts_verified: false,
            },
            assignments: Vec::new(),
        }
    }

    /// Gera reference design a partir do HardwareSpec + component DB (mapper + contratos).
    pub fn from_hardware_spec(spec: &HardwareSpec, db: &ComponentDb) -> Self {
        let mapper = ComponentMapper::new(db.clone());
        let synthesized = mapper.map_spec(spec);
        Self::from_synthesized(spec, &synthesized, db)
    }

    /// Constrói o design a partir de um SynthesizedSpec já mapeado.
    pub fn from_synthesized(spec: &HardwareSpec, synthesized: &SynthesizedSpec, db: &ComponentDb) -> Self {
        let title = if spec.source.is_empty() {
            "Reference Design".into()
        } else {
            format!("Reference Design — {}", spec.source)
        };

        let mut design = Self::new(&title, &spec.source);

        let mut total_contracts = 0u32;
        let mut satisfied_contracts = 0u32;
        let mut violations = Vec::new();
        let mut est_cost = 0.0f64;
        let mut parts: Vec<String> = Vec::new();

        let mut cpu: Option<ComponentChoice> = None;
        let mut memory = Vec::new();
        let mut peripherals = Vec::new();
        let mut summaries = Vec::new();

        for assignment in &synthesized.assignments {
            let entry = db.by_name(&assignment.component);
            let price = entry.and_then(|e| e.availability.as_ref().and_then(|a| a.price_1k));
            let package = entry.and_then(|e| e.package.clone());
            let category = entry.map(|e| e.category);

            if let Some(p) = price {
                est_cost += p;
            }
            if !parts.iter().any(|p| p == &assignment.component) {
                parts.push(assignment.component.clone());
            }

            let choice = ComponentChoice {
                part: assignment.component.clone(),
                interface: Some(assignment.interface.clone()),
                package,
                price,
            };

            match category {
                Some(ComponentCategory::Mcu) | Some(ComponentCategory::Cpu) | Some(ComponentCategory::Fpga) => {
                    if cpu.is_none() {
                        cpu = Some(choice.clone());
                    } else {
                        peripherals.push(choice);
                    }
                }
                Some(ComponentCategory::Memory) => memory.push(choice),
                _ => peripherals.push(choice),
            }

            let match_score = assignment
                .config
                .get("match_score")
                .and_then(|v| v.as_f64());

            let block_kind = spec
                .blocks
                .iter()
                .find(|b| b.id == assignment.block_id)
                .map(|b| format!("{:?}", b.kind))
                .unwrap_or_else(|| "Unknown".into());

            summaries.push(AssignmentSummary {
                block_id: assignment.block_id.clone(),
                block_kind,
                component: assignment.component.clone(),
                interface: assignment.interface.clone(),
                match_score,
            });
        }

        // Se não houve assignment de CPU, tenta o melhor MCU da DB (antes de contratos)
        let fallback_mcu_name = if cpu.is_none() {
            db.by_category(ComponentCategory::Mcu)
                .into_iter()
                .max_by(|a, b| {
                    let ma = a.features.cpu.as_ref().map(|c| c.max_mhz).unwrap_or(0);
                    let mb = b.features.cpu.as_ref().map(|c| c.max_mhz).unwrap_or(0);
                    ma.cmp(&mb)
                })
                .map(|m| m.part.clone())
        } else {
            None
        };

        if cpu.is_none() {
            if let Some(ref name) = fallback_mcu_name {
                if let Some(mcu) = db.by_name(name) {
                    let price = mcu.availability.as_ref().and_then(|a| a.price_1k);
                    if let Some(p) = price {
                        est_cost += p;
                    }
                    if !parts.iter().any(|p| p == &mcu.part) {
                        parts.push(mcu.part.clone());
                    }
                    cpu = Some(ComponentChoice {
                        part: mcu.part.clone(),
                        interface: Some("memory_bus".into()),
                        package: mcu.package.clone(),
                        price,
                    });
                }
            }
        }

        // Contratos por bloco — componente do assignment ou MCU de fallback
        for block in &spec.blocks {
            let reqs = extract_contracts(block);
            total_contracts += reqs.len() as u32;

            let assigned = synthesized
                .assignments
                .iter()
                .find(|a| a.block_id == block.id)
                .and_then(|a| db.by_name(&a.component))
                .or_else(|| {
                    fallback_mcu_name
                        .as_ref()
                        .and_then(|n| db.by_name(n))
                        .or_else(|| cpu.as_ref().and_then(|c| db.by_name(&c.part)))
                });

            match assigned {
                Some(comp) => {
                    for req in &reqs {
                        let result = verify_contract(comp, req);
                        if result.satisfied {
                            satisfied_contracts += 1;
                        } else {
                            violations.push(ContractViolation {
                                contract: format!("{}:{}", block.id, result.contract),
                                reason: format!("{} ({})", result.detail, comp.part),
                            });
                        }
                    }
                }
                None => {
                    for req in &reqs {
                        violations.push(ContractViolation {
                            contract: format!("{}:{}", block.id, req.contract),
                            reason: "no component assigned".into(),
                        });
                    }
                }
            }
        }

        // Memória a partir das regiões do spec, se a DB tiver candidatos
        if memory.is_empty() {
            for region in &spec.memory.regions {
                if let Some(mem) = db.by_category(ComponentCategory::Memory).first() {
                    let price = mem.availability.as_ref().and_then(|a| a.price_1k);
                    if let Some(p) = price {
                        est_cost += p;
                    }
                    if !parts.iter().any(|p| p == &mem.part) {
                        parts.push(mem.part.clone());
                    }
                    memory.push(ComponentChoice {
                        part: mem.part.clone(),
                        interface: Some(format!("{:?}", region.region_type)),
                        package: mem.package.clone(),
                        price,
                    });
                }
            }
        }

        let layers = estimate_layers(spec);
        // Path to Real R3: ≥70% contratos satisfeitos é o gate do wedge
        let ratio = if total_contracts > 0 {
            satisfied_contracts as f64 / total_contracts as f64
        } else {
            0.0
        };
        let contracts_ok = total_contracts > 0 && ratio >= 0.70;
        let status = if contracts_ok && violations.is_empty() {
            "contracts_satisfied".into()
        } else if contracts_ok {
            "contracts_mostly_satisfied".into()
        } else if synthesized.assignments.is_empty() {
            "unmapped".into()
        } else if violations.is_empty() {
            "draft".into()
        } else {
            "needs_review".into()
        };

        design.architecture.cpu = cpu.unwrap_or(ComponentChoice {
            part: "unassigned".into(),
            interface: None,
            package: None,
            price: None,
        });
        design.architecture.memory = memory;
        design.architecture.peripherals = peripherals;
        design.contracts = ContractReport {
            total: total_contracts,
            satisfied: satisfied_contracts,
            violations,
        };
        design.bom = BomSummary {
            total_parts: parts.len() as u32,
            estimated_cost: (est_cost * 100.0).round() / 100.0,
        };
        design.pcb.layers = layers;
        design.pcb.notes = build_pcb_notes(spec, &synthesized);
        design.validation = ValidationStatus {
            status,
            contracts_verified: contracts_ok,
        };
        design.assignments = summaries;
        design
    }

    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

fn estimate_layers(spec: &HardwareSpec) -> u8 {
    let complex = spec.blocks.iter().any(|b| {
        matches!(
            b.kind,
            BlockKind::Gpu | BlockKind::Dma | BlockKind::Ethernet | BlockKind::Usb
        )
    });
    if complex || spec.blocks.len() > 6 {
        4
    } else if spec.blocks.len() > 2 {
        2
    } else {
        2
    }
}

fn build_pcb_notes(spec: &HardwareSpec, synthesized: &SynthesizedSpec) -> Vec<String> {
    let mut notes = vec![
        "Engineering draft — requires layout review".into(),
        format!(
            "{} functional blocks → {} component assignments",
            spec.blocks.len(),
            synthesized.assignments.len()
        ),
    ];
    if synthesized.assignments.is_empty() {
        notes.push("No component mapping — expand component_db or refine HardwareSpec".into());
    }
    if spec.blocks.iter().any(|b| b.confidence < 0.5) {
        notes.push("Low-confidence blocks present — validate with HIL/trace before fab".into());
    }
    notes.push("Power tree and decoupling not auto-generated".into());
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::types::*;
    use std::collections::HashMap;

    fn mock_db() -> ComponentDb {
        let mut db = ComponentDb::new();
        db.add_entry(crate::component_db::ComponentEntry {
            part: "RP2350A".into(),
            manufacturer: "RPi".into(),
            description: "MCU".into(),
            category: ComponentCategory::Mcu,
            package: Some("QFN-56".into()),
            features: crate::component_db::ComponentFeatures {
                cpu: Some(crate::component_db::CpuFeature {
                    cores: 2,
                    max_mhz: 150,
                    architecture: Some("armv8-m".into()),
                }),
                memory: None,
                peripherals: {
                    let mut p = HashMap::new();
                    p.insert("dma".into(), 12);
                    p.insert("spi".into(), 2);
                    p.insert("uart".into(), 2);
                    p
                },
            },
            timing: None,
            compatible_with: vec![],
            power: None,
            pins: Some(
                (0..48)
                    .map(|i| crate::component_db::PinDef {
                        number: i,
                        name: format!("GP{}", i),
                        functions: vec!["gpio".into()],
                    })
                    .collect(),
            ),
            availability: Some(crate::component_db::Availability {
                status: "production".into(),
                price_1k: Some(1.50),
                distributor: vec![],
            }),
        });
        db
    }

    fn sample_spec() -> HardwareSpec {
        let mut spec = HardwareSpec::empty();
        spec.source = "test.bin".into();
        spec.blocks.push(FunctionalBlock {
            id: "uart_0".into(),
            kind: BlockKind::Uart,
            base_address: 0x40034000,
            size: 0x1000,
            registers: vec![Register {
                offset: 0,
                name: Some("dr".into()),
                width: 32,
                access: AccessType::ReadWrite,
                purpose: RegisterPurpose::DataPort,
                reset_value: None,
                observed_values: vec![],
                bitfields: vec![],
                polling: false,
                count: 1,
            }],
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

    #[test]
    fn test_reference_design_new() {
        let rd = ReferenceDesign::new("Test Design", "firmware.bin");
        assert_eq!(rd.design.title, "Test Design");
        assert_eq!(rd.pcb.pcb_type, "engineering_draft");
    }

    #[test]
    fn test_from_hardware_spec_assigns_cpu() {
        let db = mock_db();
        let spec = sample_spec();
        let rd = ReferenceDesign::from_hardware_spec(&spec, &db);
        assert_ne!(rd.architecture.cpu.part, "unassigned");
        assert_ne!(rd.architecture.cpu.part, "TBD");
        assert_eq!(rd.architecture.cpu.part, "RP2350A");
        assert!(rd.bom.total_parts > 0);
        assert!(rd.bom.estimated_cost > 0.0);
        assert!(!rd.assignments.is_empty());
        assert!(rd.contracts.total > 0);
        let ratio = rd.contracts.satisfied as f64 / rd.contracts.total as f64;
        assert!(
            ratio >= 0.70,
            "expected ≥70% contracts, got {}/{}",
            rd.contracts.satisfied,
            rd.contracts.total
        );
        assert!(rd.validation.contracts_verified);
    }

    #[test]
    fn test_yaml_output() {
        let db = mock_db();
        let rd = ReferenceDesign::from_hardware_spec(&sample_spec(), &db);
        let yaml = rd.to_yaml().unwrap();
        assert!(yaml.contains("engineering_draft"));
        assert!(yaml.contains("RP2350A") || yaml.contains("uart_0"));
    }
}
