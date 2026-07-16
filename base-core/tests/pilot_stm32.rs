//! U1 — STM32F103 USART1 @ 0x40013800 + preferred manufacturer ST.
use base_core::component_db::ComponentDb;
use base_core::design::ReferenceDesign;
use base_core::inference::extraction::MmioAccess;
use base_core::inference::generate_spec_with_evidence;
use base_core::loop_::evidence_confidence;
use base_core::mapping::mapper::ComponentMapper;
use base_core::spec::types::BlockKind;
use std::fs;
use std::path::PathBuf;

/// Documented STM32F103 USART1 base (registers live here).
const USART1: u64 = 0x4001_3800;
/// 4K MMIO page containing USART1 (analyze clustering mask).
const USART1_PAGE: u64 = 0x4001_3000;

fn pilot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pilot_stm32")
}

fn load_db() -> ComponentDb {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("component_db");
    let mut db = ComponentDb::new();
    assert!(db.load_directory(&dir).unwrap() > 0);
    db
}

#[test]
fn stm32_usart1_mmio_builds_uart_block() {
    let text = fs::read_to_string(pilot_dir().join("mmio.json")).unwrap();
    let accesses: Vec<MmioAccess> = serde_json::from_str(&text).unwrap();
    let (mut spec, _) = generate_spec_with_evidence(&accesses, "pilot_stm32/fw.bin");
    for b in &mut spec.blocks {
        b.kind = BlockKind::Uart;
        b.confidence = evidence_confidence(b);
    }
    assert_eq!(spec.blocks.len(), 1);
    // Clustering is 4K-page: USART1 @ 0x40013800 → page base 0x40013000.
    assert_eq!(spec.blocks[0].base_address & !0xfff, USART1_PAGE);
    assert!(
        accesses.iter().any(|a| a.address == USART1),
        "fixture must touch real USART1 base"
    );
    assert_eq!(spec.blocks[0].kind, BlockKind::Uart);
}

#[test]
fn stm32_design_prefers_stmicro_f103() {
    let text = fs::read_to_string(pilot_dir().join("mmio.json")).unwrap();
    let accesses: Vec<MmioAccess> = serde_json::from_str(&text).unwrap();
    let (mut spec, _) = generate_spec_with_evidence(&accesses, "pilot_stm32/fw.bin");
    for b in &mut spec.blocks {
        b.kind = BlockKind::Uart;
        b.confidence = evidence_confidence(b);
    }

    let db = load_db();
    assert!(db.by_name("STM32F103C8").is_some());

    let mapper = ComponentMapper::new(db.clone());
    let without = mapper.map_spec_with_budget(&spec, Some(80.0));
    // Cost-first may pick RP2040; with ST preference must pick F103
    let with_st =
        mapper.map_spec_with_prefs(&spec, Some(80.0), Some("STMicroelectronics"));
    assert_eq!(with_st.assignments.len(), 1);
    assert_eq!(with_st.assignments[0].component, "STM32F103C8");
    assert_eq!(with_st.assignments[0].interface, "uart");
    let _ = without;

    let design = ReferenceDesign::from_hardware_spec_prefs(
        &spec,
        &db,
        Some(80.0),
        Some("STMicroelectronics"),
    );
    assert_eq!(design.architecture.cpu.part, "STM32F103C8");
    let ratio = design.contracts.satisfied as f64 / design.contracts.total.max(1) as f64;
    assert!(
        ratio >= 0.70,
        "expected ≥70% contracts, got {}/{}",
        design.contracts.satisfied,
        design.contracts.total
    );
}

#[test]
fn stm32f103_pins_declare_usart1() {
    let db = load_db();
    let entry = db.by_name("STM32F103C8").expect("STM32F103C8 in DB");
    let pins = entry.pins.as_ref().expect("V2 pins on STM32F103C8");
    assert!(
        pins.iter().any(|p| p.name == "PA9"
            && p.functions.iter().any(|f| f == "usart1_tx" || f == "uart0_tx")),
        "PA9 must carry USART1 TX"
    );
    assert!(
        pins.iter().any(|p| p.name == "PA10"
            && p.functions.iter().any(|f| f == "usart1_rx" || f == "uart0_rx")),
        "PA10 must carry USART1 RX"
    );
}
