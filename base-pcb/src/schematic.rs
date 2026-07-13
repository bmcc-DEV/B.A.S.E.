use crate::sexpr::{sexpr, SExpr};
use base_core::spec::types::{
    ComponentAssignment, FunctionalBlock, HardwareSpec, NetSegment, SynthesizedSpec,
};
use base_core::mapping::netlist::generate_netlist;
use base_core::component_db::ComponentDb;
use chrono::Local;

/// Gerador de esquemático KiCad (.kicad_sch)
pub struct SchematicGenerator {
    component_db: Option<ComponentDb>,
}

impl SchematicGenerator {
    pub fn new(db: Option<ComponentDb>) -> Self {
        Self { component_db: db }
    }

    /// Gera o conteúdo do arquivo .kicad_sch a partir de um SynthesizedSpec
    pub fn generate(&self, spec: &SynthesizedSpec) -> String {
        let mut body = Vec::new();

        // Title block
        let title_block = sexpr("title_block")
            .list(sexpr("title").atom("B.A.S.E. Generated"))
            .list(sexpr("date").atom(&Local::now().format("%Y-%m-%d").to_string()))
            .list(sexpr("rev").atom("1.0"))
            .list(sexpr("company").atom("B.A.S.E."));
        body.push(title_block);

        // Power symbols
        body.push(self.make_power_symbol("GND", 0, 0));
        body.push(self.make_power_symbol("VCC_3V3", 200, 0));
        body.push(self.make_power_symbol("VCC_5V", 400, 0));

        // Component symbols
        let mut ref_counter = 0u64;
        for assignment in &spec.assignments {
            let symbol = self.make_component_symbol(assignment, &mut ref_counter);
            body.push(symbol);
        }

        // Wires (nets)
        if let Some(ref netlist) = spec.netlist {
            for net in netlist {
                let wire = self.make_wire(net);
                body.push(wire);
            }
        }

        // Nets declaration
        let net_names = self.collect_net_names(&spec.assignments);
        for (code, name) in net_names.iter().enumerate() {
            body.push(
                sexpr("net")
                    .atom(&format!("(code {})", code))
                    .atom(&format!("(name \"{}\")", name)),
            );
        }

        // Shell
        let header = sexpr("kicad_sch")
            .atom("(version 20231121)")
            .atom("(generator \"base-pcb\")");
        let mut output = String::new();
        output.push_str(&header.to_string(0));
        output.push('\n');
        for expr in &body {
            output.push_str(&expr.to_string(1));
            output.push('\n');
        }
        output
    }

    fn make_power_symbol(&self, name: &str, x: i64, y: i64) -> SExpr {
        sexpr("symbol")
            .atom(&format!("(lib_id \"power:{}\")", name))
            .atom(&format!("(at {} {})", x, y))
            .atom("(unit 1)")
            .atom("(in_bom yes)")
            .atom("(on_board yes)")
            .list(
                sexpr("property")
                    .atom("Reference")
                    .atom(&format!("#{}", name))
                    .list(sexpr("at").atom(&format!("{} {}", x, y.saturating_sub(100)))
                        .list(sexpr("effects").list(sexpr("font").atom("(size 1.27 1.27)")))),
            )
            .list(
                sexpr("pin")
                    .atom("1")
                    .atom(&format!("(xy {} {})", x, y)),
            )
    }

    fn make_component_symbol(
        &self,
        assignment: &ComponentAssignment,
        ref_counter: &mut u64,
    ) -> SExpr {
        *ref_counter += 1;
        let ref_name = format!("U{}", ref_counter);
        let x = (*ref_counter as i64 * 200) % 1000;
        let y = (*ref_counter as i64 / 5) * 200;

        // Look up component info from DB
        let (lib_id, value) = self
            .component_db
            .as_ref()
            .and_then(|db| db.by_name(&assignment.component))
            .map(|entry| {
                let lib = format!("{}:{}", entry.manufacturer.replace(' ', "_"), entry.part);
                (lib, entry.part.clone())
            })
            .unwrap_or_else(|| {
                (
                    format!("Connector:Generic"),
                    assignment.component.clone(),
                )
            });

        sexpr("symbol")
            .atom(&format!("(lib_id \"{}\")", lib_id))
            .atom(&format!("(at {} {})", x, y))
            .atom("(unit 1)")
            .atom("(in_bom yes)")
            .atom("(on_board yes)")
            .list(
                sexpr("property")
                    .atom("Reference")
                    .atom(&ref_name)
                    .list(sexpr("at").atom(&format!("{} {}", x, y.saturating_sub(100)))
                        .list(sexpr("effects").list(sexpr("font").atom("(size 1.27 1.27)")))),
            )
            .list(
                sexpr("property")
                    .atom("Value")
                    .atom(&value)
                    .list(sexpr("at").atom(&format!("{} {}", x, y.saturating_add(100)))
                        .list(sexpr("effects").list(sexpr("font").atom("(size 1.27 1.27)")))),
            )
            .list(
                sexpr("property")
                    .atom("Footprint")
                    .atom(&format!("Package:{}", lib_id))
                    .list(sexpr("at").atom(&format!("{} {}", x, y.saturating_add(200)))),
            )
    }

    fn make_wire(&self, net: &NetSegment) -> SExpr {
        // Simplified: wires are straight lines between components
        let x1 = 100;
        let y1 = 100;
        let x2 = 300;
        let y2 = 300;

        sexpr("wire")
            .atom(&format!("(pts (xy {} {}) (xy {} {}))", x1, y1, x2, y2))
            .atom("(stroke (width 0.254) (type default))")
            .atom("(layer \"F.Cu\")")
    }

    fn collect_net_names(&self, assignments: &[ComponentAssignment]) -> Vec<String> {
        let mut names: Vec<String> = assignments
            .iter()
            .flat_map(|a| {
                vec![
                    format!("{}_DATA", a.interface.to_uppercase()),
                    format!("{}_IRQ", a.block_id),
                    "VCC_3V3".to_string(),
                    "GND".to_string(),
                ]
            })
            .collect();
        names.sort();
        names.dedup();
        names
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
            netlist: Some(vec![
                NetSegment { from: "gpu_0".into(), to: "soc".into(), signal: "SPI_DATA".into(), protocol: "spi".into() },
                NetSegment { from: "audio_0".into(), to: "soc".into(), signal: "I2C_DATA".into(), protocol: "i2c".into() },
            ]),
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        }
    }

    #[test]
    fn test_schematic_generation() {
        let gen = SchematicGenerator::new(None);
        let spec = mock_spec();
        let sch = gen.generate(&spec);
        assert!(sch.contains("kicad_sch"), "Should have KiCad header");
        assert!(sch.contains("B.A.S.E."), "Should have generator name");
        assert!(sch.contains("RP2350A"), "Should contain RP2350A");
        assert!(sch.contains("PCM5102A"), "Should contain PCM5102A");
    }

    #[test]
    fn test_schematic_with_db() {
        let mut db = base_core::component_db::ComponentDb::new();
        db.add_entry(base_core::component_db::ComponentEntry {
            part: "RP2350A".into(),
            manufacturer: "Raspberry_Pi".into(),
            description: "MCU".into(),
            category: base_core::component_db::ComponentCategory::Mcu,
            package: Some("QFN-56".into()),
            features: base_core::component_db::ComponentFeatures {
                cpu: Some(base_core::component_db::CpuFeature { cores: 4, max_mhz: 150, architecture: None }),
                memory: None,
                peripherals: std::collections::HashMap::new(),
            },
            power: None,
            pins: None,
            availability: None,
        });
        let gen = SchematicGenerator::new(Some(db));
        let spec = mock_spec();
        let sch = gen.generate(&spec);
        assert!(sch.contains("Raspberry_Pi"), "Should use DB lib_id");
    }
}
