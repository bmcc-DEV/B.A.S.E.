/// Evidence DB — fatos puros observados do hardware.
///
///
/// Regra de ouro:
/// - Evidence DB = FATOS (imutáveis, sem interpretação)
/// - Assessment = OPINIÃO (separado, mutável, com confidence)
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceDb {
    pub source: String,
    pub entries: Vec<EvidenceEntry>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceEntry {
    pub id: String,
    pub evidence_type: EvidenceType,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EvidenceType {
    MmioWrite {
        address: u64,
        value: Option<u64>,
    },
    MmioRead {
        address: u64,
    },
    Irq {
        vector: u8,
        polarity: IrqPolarity,
    },
    Dma {
        source: u64,
        destination: u64,
        size: Option<u64>,
    },
    GpioToggle {
        pin: u8,
        value: bool,
    },
    FunctionCall {
        from: String,
        to: String,
    },
    SpiTransfer {
        mosi: Option<u64>,
        miso: Option<u64>,
        cs: u8,
    },
    I2cTransfer {
        device_addr: u8,
        data: Vec<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrqPolarity {
    High,
    Low,
    Rising,
    Falling,
}

impl EvidenceDb {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            entries: Vec::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Constrói Evidence DB a partir de acessos MMIO observados (fatos puros).
    pub fn from_mmio_accesses(
        accesses: &[crate::inference::extraction::MmioAccess],
        source: &str,
    ) -> Self {
        use crate::inference::extraction::MmioAccessType;
        let mut db = Self::new(source);
        for (i, a) in accesses.iter().enumerate() {
            let evidence_type = match a.access_type {
                MmioAccessType::Write => EvidenceType::MmioWrite {
                    address: a.address,
                    value: a.value,
                },
                MmioAccessType::Read => EvidenceType::MmioRead {
                    address: a.address,
                },
            };
            let mut context = HashMap::new();
            context.insert("function".into(), a.function_name.clone());
            context.insert("instr".into(), format!("0x{:x}", a.instruction_addr));
            db.add(EvidenceEntry {
                id: format!("mmio_{}", i),
                evidence_type,
                context,
            });
        }
        db
    }

    pub fn add(&mut self, entry: EvidenceEntry) {
        self.entries.push(entry);
    }

    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    pub fn from_yaml(s: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }

    /// Filtra evidências por tipo
    pub fn filter_by_type(&self, type_name: &str) -> Vec<&EvidenceEntry> {
        self.entries.iter().filter(|e| {
            let t = format!("{:?}", std::mem::discriminant(&e.evidence_type));
            t.contains(type_name)
        }).collect()
    }

    /// Número total de evidências
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Endereços MMIO únicos observados
    pub fn unique_mmio_addresses(&self) -> Vec<u64> {
        let mut addrs = Vec::new();
        for entry in &self.entries {
            match &entry.evidence_type {
                EvidenceType::MmioWrite { address, .. } | EvidenceType::MmioRead { address } => {
                    if !addrs.contains(address) {
                        addrs.push(*address);
                    }
                }
                _ => {}
            }
        }
        addrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_db_new() {
        let db = EvidenceDb::new("test");
        assert_eq!(db.source, "test");
        assert!(db.entries.is_empty());
    }

    #[test]
    fn test_add_evidence() {
        let mut db = EvidenceDb::new("test");
        db.add(EvidenceEntry {
            id: "ev_001".into(),
            evidence_type: EvidenceType::MmioWrite {
                address: 0x10000000,
                value: Some(1),
            },
            context: [("function".into(), "init".into())].into(),
        });
        assert_eq!(db.count(), 1);
    }

    #[test]
    fn test_yaml_roundtrip() {
        let mut db = EvidenceDb::new("test");
        db.add(EvidenceEntry {
            id: "ev_001".into(),
            evidence_type: EvidenceType::Irq { vector: 16, polarity: IrqPolarity::High },
            context: HashMap::new(),
        });
        let yaml = db.to_yaml().unwrap();
        let decoded = EvidenceDb::from_yaml(&yaml).unwrap();
        assert_eq!(decoded.count(), 1);
    }

    #[test]
    fn test_unique_mmio_addresses() {
        let mut db = EvidenceDb::new("test");
        db.add(EvidenceEntry {
            id: "ev_001".into(),
            evidence_type: EvidenceType::MmioWrite { address: 0x10000000, value: None },
            context: HashMap::new(),
        });
        db.add(EvidenceEntry {
            id: "ev_002".into(),
            evidence_type: EvidenceType::MmioWrite { address: 0x10000000, value: None },
            context: HashMap::new(),
        });
        db.add(EvidenceEntry {
            id: "ev_003".into(),
            evidence_type: EvidenceType::MmioRead { address: 0x10000004 },
            context: HashMap::new(),
        });
        let addrs = db.unique_mmio_addresses();
        assert_eq!(addrs.len(), 2);
        assert!(addrs.contains(&0x10000000));
        assert!(addrs.contains(&0x10000004));
    }
}
