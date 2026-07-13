/// Reference Design — saída principal do pipeline.
///
/// Um YAML descritivo que especifica a arquitetura do sistema.
/// NÃO é uma PCB final — é um "engineering draft" que o engenheiro humano refina.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceDesign {
    pub design: DesignMeta,
    pub architecture: Architecture,
    pub contracts: ContractReport,
    pub bom: BomSummary,
    pub pcb: PcbNote,
    pub validation: ValidationStatus,
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

impl ReferenceDesign {
    pub fn new(title: &str, source: &str) -> Self {
        Self {
            design: DesignMeta { title: title.to_string(), version: 1, source: source.to_string() },
            architecture: Architecture {
                cpu: ComponentChoice { part: "TBD".into(), interface: None, package: None, price: None },
                memory: Vec::new(),
                peripherals: Vec::new(),
            },
            contracts: ContractReport { total: 0, satisfied: 0, violations: Vec::new() },
            bom: BomSummary { total_parts: 0, estimated_cost: 0.0 },
            pcb: PcbNote {
                pcb_type: "engineering_draft".into(),
                layers: 2,
                notes: vec![
                    "Engineering draft — requires layout review".into(),
                    "Power tree not included".into(),
                    "Decoupling capacitors TBD".into(),
                ],
            },
            validation: ValidationStatus { status: "pending".into(), contracts_verified: false },
        }
    }

    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reference_design_new() {
        let rd = ReferenceDesign::new("Test Design", "firmware.bin");
        assert_eq!(rd.design.title, "Test Design");
        assert_eq!(rd.pcb.pcb_type, "engineering_draft");
    }

    #[test]
    fn test_yaml_output() {
        let rd = ReferenceDesign::new("LK Bootloader", "lk-sign.bin");
        let yaml = rd.to_yaml().unwrap();
        assert!(yaml.contains("engineering_draft"));
        assert!(yaml.contains("LK Bootloader"));
    }
}
