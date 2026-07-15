//! R3 — analyze → synth → design no wedge UART.
use base_core::component_db::ComponentDb;
use base_core::design::ReferenceDesign;
use base_core::inference::extraction::MmioAccess;
use base_core::inference::generate_spec_with_evidence;
use base_core::loop_::evidence_confidence;
use base_core::mapping::mapper::ComponentMapper;
use base_core::mapping::netlist::generate_netlist;
use base_core::spec::types::BlockKind;
use std::fs;
use std::path::PathBuf;

fn pilot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pilot")
}

fn load_pilot_db() -> ComponentDb {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("component_db");
    let mut db = ComponentDb::new();
    let n = db.load_directory(&dir).expect("load component_db");
    assert!(n > 0, "component_db must contain YAML entries");
    db
}

#[test]
fn pilot_analyze_synth_design_picks_mcu_and_meets_contracts() {
    let path = pilot_dir().join("mmio.json");
    let text = fs::read_to_string(&path).unwrap();
    let accesses: Vec<MmioAccess> = serde_json::from_str(&text).unwrap();

    let (mut spec, _ev) = generate_spec_with_evidence(&accesses, "pilot/fw.bin");
    for b in &mut spec.blocks {
        b.kind = BlockKind::Uart;
        b.confidence = evidence_confidence(b);
    }
    assert!(!spec.blocks.is_empty());

    let db = load_pilot_db();
    let mapper = ComponentMapper::new(db.clone());
    let mut synthesized = mapper.map_spec_with_budget(&spec, Some(80.0));
    assert!(
        !synthesized.assignments.is_empty(),
        "expected UART → MCU assignment under $80 BOM"
    );
    let part = &synthesized.assignments[0].component;
    let entry = db.by_name(part).expect("assigned part in DB");
    assert!(
        matches!(
            entry.category,
            base_core::component_db::ComponentCategory::Mcu
        ),
        "UART wedge must pick MCU, got {:?} ({})",
        entry.category,
        part
    );
    assert_ne!(part.as_str(), "ECP5-12F");
    assert_eq!(synthesized.assignments[0].interface, "uart");

    // Netlist nominal (não elétrico — nós lógicos, não copper)
    let nl = generate_netlist(&synthesized, &db);
    assert!(!nl.is_empty());
    assert!(nl.iter().any(|s| s.protocol == "uart"));
    synthesized.netlist = Some(nl);

    let design = ReferenceDesign::from_synthesized(&spec, &synthesized, &db);
    assert!(
        matches!(
            db.by_name(&design.architecture.cpu.part)
                .map(|e| e.category),
            Some(base_core::component_db::ComponentCategory::Mcu)
        )
    );
    assert!(design.bom.estimated_cost > 0.0);
    assert!(design.bom.estimated_cost <= 80.0);
    assert!(!design.assignments.is_empty());
    assert!(design.contracts.total >= 2);
    let ratio = design.contracts.satisfied as f64 / design.contracts.total as f64;
    assert!(
        ratio >= 0.70,
        "expected ≥70% contracts, got {}/{} violations={:?}",
        design.contracts.satisfied,
        design.contracts.total,
        design.contracts.violations
    );
    assert!(design.validation.contracts_verified);

    let yaml = design.to_yaml().unwrap();
    assert!(!yaml.contains("ECP5-12F"));
    assert!(!yaml.contains("unassigned"));
}

#[test]
fn pilot_tight_budget_excludes_fpga_and_pricey_mcu() {
    let path = pilot_dir().join("mmio.json");
    let text = fs::read_to_string(&path).unwrap();
    let accesses: Vec<MmioAccess> = serde_json::from_str(&text).unwrap();
    let (mut spec, _) = generate_spec_with_evidence(&accesses, "pilot/fw.bin");
    for b in &mut spec.blocks {
        b.kind = BlockKind::Uart;
    }

    let db = load_pilot_db();
    let mapper = ComponentMapper::new(db.clone());

    // $0.50: abaixo de RP2040 ($0.70) e RP2350 ($1.50) → sem assignment MCU viável
    let syn = mapper.map_spec_with_budget(&spec, Some(0.50));
    assert!(
        syn.assignments.is_empty(),
        "budget $0.50 should exclude all UART MCUs with known price_1k, got {:?}",
        syn.assignments
    );

    // $80: MCU barato, nunca FPGA ($25+) sozinho preferido sobre MCU
    let syn80 = mapper.map_spec_with_budget(&spec, Some(80.0));
    let part = &syn80.assignments[0].component;
    let cat = db.by_name(part).unwrap().category;
    assert!(matches!(
        cat,
        base_core::component_db::ComponentCategory::Mcu
    ));
    assert_ne!(part.as_str(), "ECP5-12F");
}
