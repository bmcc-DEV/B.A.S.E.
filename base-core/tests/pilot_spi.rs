//! T1 B2 — segundo bloco SPI no mesmo wedge RP (UART @ 0x40034000 + SPI0 @ 0x4003c000).
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

const UART_BASE: u64 = 0x4003_4000;
const SPI_BASE: u64 = 0x4003_c000;

fn pilot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pilot")
}

fn load_pilot_db() -> ComponentDb {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("component_db");
    let mut db = ComponentDb::new();
    let n = db.load_directory(&dir).expect("load component_db");
    assert!(n > 0);
    db
}

fn classify_by_page(spec: &mut base_core::spec::types::HardwareSpec) {
    for b in &mut spec.blocks {
        b.kind = match b.base_address & !0xfff {
            UART_BASE => BlockKind::Uart,
            SPI_BASE => BlockKind::Spi,
            _ => b.kind,
        };
        b.confidence = evidence_confidence(b);
    }
}

#[test]
fn pilot_dual_mmio_yields_uart_and_spi_blocks() {
    let path = pilot_dir().join("mmio_uart_spi.json");
    let text = fs::read_to_string(&path).expect("mmio_uart_spi.json");
    let accesses: Vec<MmioAccess> = serde_json::from_str(&text).unwrap();
    assert!(accesses.len() >= 8);

    let (mut spec, _ev) = generate_spec_with_evidence(&accesses, "pilot/fw.bin");
    classify_by_page(&mut spec);

    assert_eq!(
        spec.blocks.len(),
        2,
        "expected UART+SPI pages, blocks={:?}",
        spec.blocks
            .iter()
            .map(|b| (b.base_address, b.kind))
            .collect::<Vec<_>>()
    );
    let kinds: Vec<_> = spec.blocks.iter().map(|b| b.kind).collect();
    assert!(kinds.contains(&BlockKind::Uart));
    assert!(kinds.contains(&BlockKind::Spi));
}

#[test]
fn pilot_dual_design_assigns_uart_and_spi_on_rp() {
    let path = pilot_dir().join("mmio_uart_spi.json");
    let text = fs::read_to_string(&path).unwrap();
    let accesses: Vec<MmioAccess> = serde_json::from_str(&text).unwrap();
    let (mut spec, _) = generate_spec_with_evidence(&accesses, "pilot/fw.bin");
    classify_by_page(&mut spec);

    let db = load_pilot_db();
    let mapper = ComponentMapper::new(db.clone());
    let mut synthesized = mapper.map_spec_with_budget(&spec, Some(80.0));
    assert_eq!(
        synthesized.assignments.len(),
        2,
        "expected 2 assignments, got {:?}",
        synthesized.assignments
    );

    let interfaces: Vec<_> = synthesized
        .assignments
        .iter()
        .map(|a| a.interface.as_str())
        .collect();
    assert!(interfaces.contains(&"uart"), "{interfaces:?}");
    assert!(interfaces.contains(&"spi"), "{interfaces:?}");

    for a in &synthesized.assignments {
        let entry = db.by_name(&a.component).expect("part in DB");
        assert!(matches!(
            entry.category,
            base_core::component_db::ComponentCategory::Mcu
        ));
        assert_ne!(a.component.as_str(), "ECP5-12F");
    }

    let nl = generate_netlist(&synthesized, &db);
    assert!(nl.iter().any(|s| s.protocol == "uart"));
    assert!(nl.iter().any(|s| s.protocol == "spi"));
    synthesized.netlist = Some(nl);

    let design = ReferenceDesign::from_synthesized(&spec, &synthesized, &db);
    assert!(design.bom.estimated_cost > 0.0 && design.bom.estimated_cost <= 80.0);
    assert!(design.contracts.total >= 4);
    let ratio = design.contracts.satisfied as f64 / design.contracts.total as f64;
    assert!(
        ratio >= 0.70,
        "expected ≥70% contracts, got {}/{} violations={:?}",
        design.contracts.satisfied,
        design.contracts.total,
        design.contracts.violations
    );
}
