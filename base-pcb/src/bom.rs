use base_core::spec::types::{ComponentAssignment, SynthesizedSpec};

/// BOM (Bill of Materials) generator — CSV format
pub struct BomGenerator;

impl BomGenerator {
    /// Gera CSV do BOM a partir de assignments
    pub fn generate(&self, spec: &SynthesizedSpec) -> String {
        let mut csv = String::from("Ref,Qty,Part,Manufacturer,Interface,Block\n");

        let mut ref_counter = 0u64;
        for assignment in &spec.assignments {
            ref_counter += 1;
            let ref_name = format!("U{}", ref_counter);
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                ref_name,
                1,
                assignment.component,
                "TBD", // manufacturer from DB would need lookup
                assignment.interface,
                assignment.block_id,
            ));
        }

        // Passive components (always included)
        csv.push_str("C1,1,100nF,Ceramic,Decoupling,Power\n");
        csv.push_str("C2,1,10uF,Ceramic,Bulk,Power\n");
        csv.push_str("R1,1,10k,Resistor,Pull-up,Interface\n");

        csv
    }

    /// Gera BOM com contagem agregada (mesmo componente aparece 1x com qtd)
    pub fn generate_aggregated(&self, spec: &SynthesizedSpec) -> String {
        use std::collections::HashMap;

        let mut parts: HashMap<String, (u32, String)> = HashMap::new();
        let mut ref_counter = 0u64;

        for assignment in &spec.assignments {
            ref_counter += 1;
            let ref_name = format!("U{}", ref_counter);
            let entry = parts
                .entry(assignment.component.clone())
                .or_insert_with(|| (0, String::new()));
            entry.0 += 1;
            if !entry.1.is_empty() {
                entry.1.push(',');
            }
            entry.1.push_str(&ref_name);
        }

        let mut csv = String::from("Part,Qty,References,Interface\n");
        for (part, (qty, refs)) in &parts {
            csv.push_str(&format!("{},{},{},\n", part, qty, refs));
        }

        csv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;

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
    fn test_bom_csv() {
        let gen = BomGenerator;
        let spec = mock_spec();
        let csv = gen.generate(&spec);
        assert!(csv.contains("RP2350A"), "BOM should contain RP2350A");
        assert!(csv.contains("PCM5102A"), "BOM should contain PCM5102A");
        assert!(csv.starts_with("Ref"), "BOM should have header");
    }

    #[test]
    fn test_bom_aggregated() {
        let gen = BomGenerator;
        let spec = mock_spec();
        let csv = gen.generate_aggregated(&spec);
        assert!(csv.contains("RP2350A"), "Aggregated BOM should contain RP2350A");
        assert!(csv.contains("PCM5102A"), "Aggregated BOM should contain PCM5102A");
    }
}
