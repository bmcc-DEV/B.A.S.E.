/// Contract Solver — matching formal entre `requires` e `provides`.
///
/// Sem ML. Sem heurística. Sem magia.
/// Apenas matching determinístico entre contratos de requisito e provedores.
use serde::{Deserialize, Serialize};
use crate::component_db::{ComponentCategory, ComponentDb, ComponentEntry};
use crate::spec::types::{BlockKind, FunctionalBlock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractRequirement {
    pub contract: String,
    pub params: Vec<ContractParam>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContractParam {
    Presence { name: String },
    Numeric { name: String, min: Option<f64>, max: Option<f64> },
    Count { name: String, min: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractResult {
    pub contract: String,
    pub satisfied: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    pub component: String,
    pub results: Vec<ContractResult>,
    pub all_satisfied: bool,
}

/// Extrai contratos de um bloco funcional para o formato de matching
pub fn extract_contracts(block: &FunctionalBlock) -> Vec<ContractRequirement> {
    let mut contracts = Vec::new();

    // Presença de MMIO (sempre verdade para blocos mapeados)
    contracts.push(ContractRequirement {
        contract: "mmio_registers".into(),
        params: vec![ContractParam::Presence { name: "mmio".into() }],
    });

    // DMA requirement
    if let Some(ref dma) = block.dma {
        if dma.required {
            contracts.push(ContractRequirement {
                contract: "dma".into(),
                params: vec![
                    ContractParam::Presence { name: "dma".into() },
                    ContractParam::Count { name: "dma_channels".into(), min: dma.max_channels.max(1) },
                ],
            });
        }
    }

    // Interrupt requirement (se tem registrador de IRQ ou polling)
    let has_irq = block.registers.iter().any(|r| {
        let name = r.name.as_deref().unwrap_or("");
        name.contains("irq") || name.contains("int") || name.contains("status")
    });
    if has_irq {
        contracts.push(ContractRequirement {
            contract: "interrupt".into(),
            params: vec![ContractParam::Presence { name: "interrupt".into() }],
        });
    }

    // GPIO count baseado no kind do bloco
    let gpio_needed = match block.kind {
        BlockKind::Gpu => 16u32,
        BlockKind::Dma => 8,
        BlockKind::Audio => 4,
        BlockKind::Spi => 4,
        BlockKind::I2c => 2,
        BlockKind::Uart => 2,
        _ => 4,
    };
    contracts.push(ContractRequirement {
        contract: "gpio".into(),
        params: vec![ContractParam::Count { name: "gpio".into(), min: gpio_needed }],
    });

    contracts
}

/// Verifica se um componente satisfaz um contrato
pub fn verify_contract(comp: &ComponentEntry, req: &ContractRequirement) -> ContractResult {
    let (satisfied, detail) = match &req.params[0] {
        ContractParam::Presence { name } => {
            let has = match name.as_str() {
                "dma" => comp.features.peripherals.get("dma").copied().unwrap_or(0) > 0,
                "mmio" => true, // todo MCU tem MMIO
                "interrupt" => matches!(
                    comp.category,
                    ComponentCategory::Mcu | ComponentCategory::Cpu | ComponentCategory::Fpga
                ) || comp.features.cpu.is_some(),
                _ => false,
            };
            (has, if has { format!("has {}", name) } else { format!("missing {}", name) })
        }
        ContractParam::Count { name, min } => {
            let count = match name.as_str() {
                "gpio" => match &comp.pins {
                    Some(pins) => pins.len() as u32,
                    // Pinout omitido no YAML: MCU/CPU tipicamente cobrem UART/SPI GPIO
                    None if matches!(
                        comp.category,
                        ComponentCategory::Mcu | ComponentCategory::Cpu
                    ) =>
                    {
                        32
                    }
                    None => 0,
                },
                "dma_channels" => comp.features.peripherals.get("dma").copied().unwrap_or(0),
                _ => 0,
            };
            let ok = count >= *min;
            (
                ok,
                if ok {
                    format!("{} >= {} ({})", count, min, comp.part)
                } else {
                    format!("{} < {} ({})", count, min, comp.part)
                },
            )
        }
        ContractParam::Numeric { name, min, max } => {
            let val = match name.as_str() {
                "clock_mhz" => comp.features.cpu.as_ref().map(|c| c.max_mhz as f64).unwrap_or(0.0),
                _ => 0.0,
            };
            let ok = min.map_or(true, |m| val >= m) && max.map_or(true, |m| val <= m);
            (ok, if ok { format!("{} ok", name) } else { format!("{} = {} out of range", name, val) })
        }
    };

    ContractResult {
        contract: req.contract.clone(),
        satisfied,
        detail,
    }
}

/// Encontra o melhor componente que satisfaz TODOS os contratos
pub fn find_solution(db: &ComponentDb, contracts: &[ContractRequirement]) -> Option<Solution> {
    // Busca em todas as categorias
    let all_candidates: Vec<ComponentEntry> = db.by_category(ComponentCategory::Mcu)
        .into_iter()
        .chain(db.by_category(ComponentCategory::Cpu).into_iter())
        .chain(db.by_category(ComponentCategory::Connectivity).into_iter())
        .cloned()
        .collect();

    for comp in &all_candidates {
        let results: Vec<ContractResult> = contracts.iter()
            .map(|c| verify_contract(comp, c))
            .collect();

        let all_satisfied = results.iter().all(|r| r.satisfied);

        if all_satisfied {
            return Some(Solution {
                component: comp.part.clone(),
                results,
                all_satisfied,
            });
        }
    }

    None
}

/// Busca o melhor componente por bloco (fallback compatível)
pub fn solve_block(db: &ComponentDb, block: &FunctionalBlock) -> Option<Solution> {
    let contracts = extract_contracts(block);
    find_solution(db, &contracts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::spec::types::*;

    fn mock_db() -> ComponentDb {
        let mut db = ComponentDb::new();
        db.add_entry(ComponentEntry {
            part: "RP2350A".into(), manufacturer: "RPi".into(), description: "MCU".into(),
            category: ComponentCategory::Mcu, package: Some("QFN-56".into()),
            features: crate::component_db::ComponentFeatures {
                cpu: Some(crate::component_db::CpuFeature { cores: 4, max_mhz: 150, architecture: None }),
                memory: None,
                peripherals: { let mut p = HashMap::new(); p.insert("dma".into(), 8); p.insert("spi".into(), 2); p },
            },
            timing: None, compatible_with: vec![],
            power: None,
            pins: Some((0..48).map(|i| crate::component_db::PinDef { number: i, name: format!("GP{}", i), functions: vec!["gpio".into()] }).collect()),
            availability: Some(crate::component_db::Availability { status: "production".into(), price_1k: Some(1.50), distributor: vec![] }),
        });
        db
    }

    #[test]
    fn test_extract_contracts() {
        let block = FunctionalBlock {
            id: "gpu_0".into(), kind: BlockKind::Gpu,
            base_address: 0x10000000, size: 0x1000,
            registers: vec![Register {
                offset: 0, name: Some("status".into()), width: 32,
                access: AccessType::Read, purpose: RegisterPurpose::Status,
                reset_value: None, observed_values: vec![], bitfields: vec![], polling: false, count: 0,
            }],
            protocol: Protocol { states: vec![], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: TimingProfile { activation: None, processing: None, interrupt_response: None, dma_setup: None, polling_interval: None },
            dma: Some(DmaRequirement { required: true, min_bandwidth_mbps: 100.0, alignment: 4, max_channels: 2 }),
            dependencies: vec![], confidence: 0.8,
        };
        let contracts = extract_contracts(&block);
        assert!(contracts.iter().any(|c| c.contract == "dma"));
        assert!(contracts.iter().any(|c| c.contract == "gpio"));
        assert!(contracts.iter().any(|c| c.contract == "mmio_registers"));
    }

    #[test]
    fn test_verify_contract_dma() {
        let db = mock_db();
        let comp = db.by_name("RP2350A").unwrap();
        let req = ContractRequirement {
            contract: "dma".into(),
            params: vec![ContractParam::Presence { name: "dma".into() }],
        };
        let result = verify_contract(comp, &req);
        assert!(result.satisfied);
    }

    #[test]
    fn test_verify_contract_gpio() {
        let db = mock_db();
        let comp = db.by_name("RP2350A").unwrap();
        let req = ContractRequirement {
            contract: "gpio".into(),
            params: vec![ContractParam::Count { name: "gpio".into(), min: 16 }],
        };
        let result = verify_contract(comp, &req);
        assert!(result.satisfied);
    }

    fn bare_mcu(pins: Option<Vec<crate::component_db::PinDef>>) -> ComponentEntry {
        ComponentEntry {
            part: "TEST_MCU".into(),
            manufacturer: "Test".into(),
            description: "MCU".into(),
            category: ComponentCategory::Mcu,
            package: None,
            features: crate::component_db::ComponentFeatures {
                cpu: Some(crate::component_db::CpuFeature {
                    cores: 1,
                    max_mhz: 100,
                    architecture: None,
                }),
                memory: None,
                peripherals: HashMap::new(),
            },
            timing: None,
            compatible_with: vec![],
            power: None,
            pins,
            availability: None,
        }
    }

    #[test]
    fn test_gpio_pin_aware_uses_pins_len() {
        let pins = (0..30)
            .map(|i| crate::component_db::PinDef {
                number: i,
                name: format!("GP{i}"),
                functions: vec!["gpio".into()],
            })
            .collect();
        let comp = bare_mcu(Some(pins));
        let req = ContractRequirement {
            contract: "gpio".into(),
            params: vec![ContractParam::Count {
                name: "gpio".into(),
                min: 16,
            }],
        };
        let result = verify_contract(&comp, &req);
        assert!(result.satisfied);
        assert!(result.detail.contains("30 >= 16"));
    }

    #[test]
    fn test_gpio_pin_aware_fails_when_short() {
        let pins = vec![crate::component_db::PinDef {
            number: 0,
            name: "GP0".into(),
            functions: vec!["gpio".into()],
        }];
        let comp = bare_mcu(Some(pins));
        let req = ContractRequirement {
            contract: "gpio".into(),
            params: vec![ContractParam::Count {
                name: "gpio".into(),
                min: 16,
            }],
        };
        let result = verify_contract(&comp, &req);
        assert!(!result.satisfied);
        assert!(result.detail.contains("1 < 16"));
    }

    #[test]
    fn test_gpio_omitted_mcu_fallback_32() {
        let comp = bare_mcu(None);
        let req = ContractRequirement {
            contract: "gpio".into(),
            params: vec![ContractParam::Count {
                name: "gpio".into(),
                min: 16,
            }],
        };
        let result = verify_contract(&comp, &req);
        assert!(result.satisfied);
        assert!(result.detail.contains("32 >="));
    }

    #[test]
    fn test_find_solution() {
        let db = mock_db();
        let contracts = vec![
            ContractRequirement { contract: "dma".into(), params: vec![ContractParam::Presence { name: "dma".into() }] },
            ContractRequirement { contract: "mmio_registers".into(), params: vec![ContractParam::Presence { name: "mmio".into() }] },
            ContractRequirement { contract: "gpio".into(), params: vec![ContractParam::Count { name: "gpio".into(), min: 16 }] },
        ];
        let solution = find_solution(&db, &contracts);
        assert!(solution.is_some());
        assert!(solution.unwrap().all_satisfied);
    }

    #[test]
    fn test_solve_block() {
        let db = mock_db();
        let block = FunctionalBlock {
            id: "gpu_0".into(), kind: BlockKind::Gpu,
            base_address: 0x10000000, size: 0x1000,
            registers: vec![],
            protocol: Protocol { states: vec![], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: TimingProfile { activation: None, processing: None, interrupt_response: None, dma_setup: None, polling_interval: None },
            dma: Some(DmaRequirement { required: true, min_bandwidth_mbps: 100.0, alignment: 4, max_channels: 2 }),
            dependencies: vec![], confidence: 0.8,
        };
        let solution = solve_block(&db, &block);
        assert!(solution.is_some());
        assert!(solution.unwrap().all_satisfied);
    }
}
