use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

static COMPONENT_DB_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/component_db");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentEntry {
    pub part: String,
    pub manufacturer: String,
    pub description: String,
    pub category: ComponentCategory,
    pub package: Option<String>,
    pub features: ComponentFeatures,
    pub power: Option<PowerSpec>,
    pub pins: Option<Vec<PinDef>>,
    pub availability: Option<Availability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentCategory {
    Mcu,
    Cpu,
    Dsp,
    Fpga,
    Pmic,
    Memory,
    Connectivity,
    Audio,
    Sensor,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentFeatures {
    pub cpu: Option<CpuFeature>,
    pub memory: Option<MemoryFeature>,
    pub peripherals: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuFeature {
    pub cores: u32,
    pub max_mhz: u32,
    pub architecture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFeature {
    pub sram: Option<String>,
    pub flash: Option<String>,
    pub external: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerSpec {
    pub vcore: Option<String>,
    pub vccio: Option<String>,
    pub vusb: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinDef {
    pub number: u32,
    pub name: String,
    pub functions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Availability {
    pub status: String,
    pub price_1k: Option<f64>,
    pub distributor: Vec<String>,
}

/// Catálogo de componentes carregado de arquivos YAML
#[derive(Debug, Clone)]
pub struct ComponentDb {
    pub entries: Vec<ComponentEntry>,
    pub by_category: HashMap<ComponentCategory, Vec<usize>>,
    pub by_name: HashMap<String, usize>,
}

impl ComponentDb {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            by_category: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    /// Carrega um único arquivo YAML de componente
    pub fn load_yaml(&mut self, path: &Path) -> Result<(), anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let entry: ComponentEntry = serde_yaml::from_str(&content)?;
        self.add_entry(entry);
        Ok(())
    }

    /// Carrega múltiplos arquivos YAML de um diretório
    pub fn load_directory(&mut self, dir: &Path) -> Result<usize, anyhow::Error> {
        let mut count = 0;
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                self.load_yaml(&path)?;
                count += 1;
            }
        }
        Ok(count)
    }

    pub fn add_entry(&mut self, entry: ComponentEntry) {
        let idx = self.entries.len();
        self.by_name.insert(entry.part.clone(), idx);
        self.by_category.entry(entry.category).or_default().push(idx);
        self.entries.push(entry);
    }

    /// Busca componentes por categoria
    pub fn by_category(&self, category: ComponentCategory) -> Vec<&ComponentEntry> {
        self.by_category.get(&category)
            .map(|indices| indices.iter().map(|&i| &self.entries[i]).collect())
            .unwrap_or_default()
    }

    /// Busca componente por nome exato
    pub fn by_name(&self, name: &str) -> Option<&ComponentEntry> {
        self.by_name.get(name).map(|&i| &self.entries[i])
    }

    /// Busca componentes que satisfazem um requisito de periférico
    pub fn with_peripheral(&self, peripheral: &str, min_count: u32) -> Vec<&ComponentEntry> {
        self.entries.iter()
            .filter(|e| e.features.peripherals.get(peripheral).copied().unwrap_or(0) >= min_count)
            .collect()
    }

    /// Busca MCUs com pelo menos N pinos GPIO
    pub fn mcus_with_gpio(&self, min_gpio: u32) -> Vec<&ComponentEntry> {
        self.entries.iter()
            .filter(|e| e.category == ComponentCategory::Mcu)
            .filter(|e| {
                e.pins.as_ref().map_or(false, |pins| pins.len() as u32 >= min_gpio)
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for ComponentDb {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_db_empty() {
        let db = ComponentDb::new();
        assert!(db.is_empty());
    }

    #[test]
    fn test_load_all_yaml_files() {
        let dir = Path::new(COMPONENT_DB_DIR);
        assert!(dir.exists(), "component_db directory should exist");
        let mut db = ComponentDb::new();
        let count = db.load_directory(dir).expect("Should load YAML files");
        assert!(count >= 50, "Should load at least 50 components, got {count}");
        assert!(db.by_name("RP2350A").is_some(), "Should have RP2350A");
        assert!(db.by_name("W5500").is_some(), "Should have W5500");
        assert!(!db.by_category(ComponentCategory::Mcu).is_empty(), "Should have MCUs");
        assert!(!db.by_category(ComponentCategory::Connectivity).is_empty(), "Should have connectivity");
    }

    #[test]
    fn test_add_entry() {
        let mut db = ComponentDb::new();
        db.add_entry(ComponentEntry {
            part: "RP2350A".into(),
            manufacturer: "Raspberry Pi".into(),
            description: "Dual Cortex-M33 + RISC-V".into(),
            category: ComponentCategory::Mcu,
            package: Some("QFN-56".into()),
            features: ComponentFeatures {
                cpu: Some(CpuFeature { cores: 4, max_mhz: 150, architecture: Some("ARMv8-M".into()) }),
                memory: Some(MemoryFeature { sram: Some("520KB".into()), flash: Some("4MB".into()), external: None }),
                peripherals: {
                    let mut p = HashMap::new();
                    p.insert("dma".into(), 8);
                    p.insert("pio".into(), 2);
                    p.insert("i2c".into(), 2);
                    p.insert("spi".into(), 2);
                    p.insert("uart".into(), 2);
                    p.insert("usb".into(), 1);
                    p
                },
            },
            power: Some(PowerSpec {
                vcore: Some("1.1V".into()),
                vccio: Some("3.3V".into()),
                vusb: Some("5V".into()),
            }),
            pins: None,
            availability: Some(Availability {
                status: "production".into(),
                price_1k: Some(1.50),
                distributor: vec!["DigiKey".into(), "Mouser".into()],
            }),
        });

        assert_eq!(db.len(), 1);
        assert!(db.by_name("RP2350A").is_some());
        assert_eq!(db.by_category(ComponentCategory::Mcu).len(), 1);
        assert_eq!(db.with_peripheral("dma", 8).len(), 1);
    }
}
