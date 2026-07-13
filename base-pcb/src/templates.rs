use base_core::spec::types::{ComponentAssignment, SynthesizedSpec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Template de bloco comum para PCB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PcbTemplate {
    pub name: String,
    pub description: String,
    pub category: String,
    pub required_interfaces: Vec<String>,
    pub schematic_fragment: String,
}

/// Sistema de templates para blocos de PCB comuns
pub struct TemplateLibrary {
    templates: Vec<PcbTemplate>,
    by_category: HashMap<String, Vec<usize>>,
}

impl TemplateLibrary {
    pub fn new() -> Self {
        let mut lib = Self {
            templates: Vec::new(),
            by_category: HashMap::new(),
        };
        lib.load_builtins();
        lib
    }

    /// Carrega templates built-in
    fn load_builtins(&mut self) {
        self.add(PcbTemplate {
            name: "rp2350-minimal".into(),
            description: "RP2350 + cristal 12MHz + QSPI flash + USB-C + SWD".into(),
            category: "mcu".into(),
            required_interfaces: vec!["spi".into(), "usb".into(), "swd".into()],
            schematic_fragment: include_str!("../templates/rp2350_minimal.kicad_sch").to_string(),
        });
        self.add(PcbTemplate {
            name: "power-3v3".into(),
            description: "Buck converter 5V→3.3V @ 1A + LDO + decoupling caps".into(),
            category: "power".into(),
            required_interfaces: vec!["power".into()],
            schematic_fragment: include_str!("../templates/power_3v3.kicad_sch").to_string(),
        });
        self.add(PcbTemplate {
            name: "usb-c-pd".into(),
            description: "USB-C connector + CC logic + PD controller".into(),
            category: "connectivity".into(),
            required_interfaces: vec!["usb".into()],
            schematic_fragment: include_str!("../templates/usb_c.kicad_sch").to_string(),
        });
        self.add(PcbTemplate {
            name: "ethernet-rgmii".into(),
            description: "Ethernet PHY + magnetics + RJ45".into(),
            category: "connectivity".into(),
            required_interfaces: vec!["ethernet".into()],
            schematic_fragment: include_str!("../templates/ethernet.kicad_sch").to_string(),
        });
        self.add(PcbTemplate {
            name: "audio-codec".into(),
            description: "I2S DAC PCM5102A + headphone jack + decoupling".into(),
            category: "audio".into(),
            required_interfaces: vec!["i2c".into(), "i2s".into()],
            schematic_fragment: include_str!("../templates/audio_codec.kicad_sch").to_string(),
        });
    }

    pub fn add(&mut self, template: PcbTemplate) {
        let idx = self.templates.len();
        self.by_category
            .entry(template.category.clone())
            .or_default()
            .push(idx);
        self.templates.push(template);
    }

    /// Carrega templates de um diretório (arquivos .yaml)
    pub fn load_directory(&mut self, dir: &Path) -> Result<usize, anyhow::Error> {
        let mut count = 0;
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                let content = std::fs::read_to_string(&path)?;
                let tmpl: PcbTemplate = serde_yaml::from_str(&content)?;
                self.add(tmpl);
                count += 1;
            }
        }
        Ok(count)
    }

    /// Encontra o melhor template para um assignment
    pub fn find_best_match(&self, assignment: &ComponentAssignment) -> Option<&PcbTemplate> {
        // Tenta match por interface
        for tmpl in &self.templates {
            if tmpl.required_interfaces.contains(&assignment.interface) {
                return Some(tmpl);
            }
        }
        None
    }

    /// Aplica template a um spec, retornando fragmento de esquemático
    pub fn apply_template(&self, spec: &SynthesizedSpec) -> String {
        let mut parts = Vec::new();
        for assignment in &spec.assignments {
            if let Some(tmpl) = self.find_best_match(assignment) {
                parts.push(tmpl.schematic_fragment.clone());
            }
        }
        parts.join("\n\n")
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

impl Default for TemplateLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;

    #[test]
    fn test_template_library_builtins() {
        let lib = TemplateLibrary::new();
        assert!(!lib.is_empty(), "Should have built-in templates");
        assert!(lib.templates.iter().any(|t| t.name.contains("rp2350")),
            "Should have RP2350 template");
    }

    #[test]
    fn test_find_best_match() {
        let lib = TemplateLibrary::new();
        let assignment = ComponentAssignment {
            block_id: "eth_0".into(),
            component: "W5500".into(),
            interface: "ethernet".into(),
            config: Default::default(),
        };
        let found = lib.find_best_match(&assignment);
        assert!(found.is_some(), "Should find ethernet template");
        assert_eq!(found.unwrap().name, "ethernet-rgmii");
    }
}
