use crate::component_db::ComponentDb;
use crate::spec::types::{NetSegment, SynthesizedSpec};

/// Gera netlist a partir de um SynthesizedSpec (conexões entre componentes)
pub fn generate_netlist(spec: &SynthesizedSpec, _db: &ComponentDb) -> Vec<NetSegment> {
    let mut segments = Vec::new();

    for assignment in &spec.assignments {
        // Interface padrão baseada no tipo de componente
        let signal = match assignment.interface.as_str() {
            "spi" => "SPI",
            "i2c" => "I2C",
            "uart" => "UART",
            "usb" => "USB",
            "dma" => "DMA",
            "gpio" => "GPIO",
            "memory_bus" => "MEM_BUS",
            _ => "GPIO",
        };

        // Conecta ao CPU/SoC principal
        segments.push(NetSegment {
            from: assignment.block_id.clone(),
            to: "soc_cpu".into(),
            signal: format!("{}_data", signal),
            protocol: assignment.interface.clone(),
        });

        // Conexão de interrupção
        segments.push(NetSegment {
            from: assignment.block_id.clone(),
            to: "irq_controller".into(),
            signal: "IRQ".into(),
            protocol: "irq".into(),
        });

        // Conexão de power
        segments.push(NetSegment {
            from: assignment.block_id.clone(),
            to: "power_rail_3v3".into(),
            signal: "VCC_3V3".into(),
            protocol: "power".into(),
        });

        segments.push(NetSegment {
            from: assignment.block_id.clone(),
            to: "gnd".into(),
            signal: "GND".into(),
            protocol: "power".into(),
        });
    }

    // Remove duplicatas
    segments.sort();
    segments.dedup();
    segments
}

/// Agrupa nets por protocolo
pub fn group_by_protocol(segments: &[NetSegment]) -> std::collections::HashMap<String, Vec<&NetSegment>> {
    let mut groups: std::collections::HashMap<String, Vec<&NetSegment>> = std::collections::HashMap::new();
    for seg in segments {
        groups.entry(seg.protocol.clone()).or_default().push(seg);
    }
    groups
}

/// Estima contagem de pinos necessária baseada na netlist
pub fn estimate_pin_count(segments: &[NetSegment]) -> u32 {
    let unique_signals: std::collections::HashSet<&str> = segments.iter().map(|s| s.signal.as_str()).collect();
    // Cada sinal = 1 pino (simplificado)
    unique_signals.len() as u32 * 2 // bidirecional
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component_db::ComponentDb;
    use crate::spec::types::{ComponentAssignment, SynthesisConstraints, HardwareSpec};

    fn mock_spec() -> SynthesizedSpec {
        SynthesizedSpec {
            original: HardwareSpec::empty(),
            assignments: vec![
                ComponentAssignment {
                    block_id: "gpu_0".into(),
                    component: "RP2350A".into(),
                    interface: "spi".into(),
                    config: serde_json::json!({}),
                },
                ComponentAssignment {
                    block_id: "audio_0".into(),
                    component: "PCM5102A".into(),
                    interface: "i2c".into(),
                    config: serde_json::json!({}),
                },
            ],
            netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        }
    }

    #[test]
    fn test_generate_netlist() {
        let db = ComponentDb::new();
        let spec = mock_spec();
        let netlist = generate_netlist(&spec, &db);

        assert!(!netlist.is_empty(), "Netlist should not be empty");
        assert!(netlist.iter().any(|s| s.signal == "SPI_data"), "Should have SPI connection");
        assert!(netlist.iter().any(|s| s.signal == "GND"), "Should have GND");
    }

    #[test]
    fn test_group_by_protocol() {
        let db = ComponentDb::new();
        let spec = mock_spec();
        let netlist = generate_netlist(&spec, &db);
        let groups = group_by_protocol(&netlist);

        assert!(groups.contains_key("spi"), "Should have spi group");
        assert!(groups.contains_key("power"), "Should have power group");
    }

    #[test]
    fn test_estimate_pin_count() {
        let db = ComponentDb::new();
        let spec = mock_spec();
        let netlist = generate_netlist(&spec, &db);
        let pins = estimate_pin_count(&netlist);
        assert!(pins > 0, "Should estimate some pins");
    }
}
