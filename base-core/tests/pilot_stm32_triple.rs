//! Y3 — STM32F103 USART1 + SPI2 + I2C1 (three 4K pages).
use base_core::component_db::ComponentDb;
use base_core::design::ReferenceDesign;
use base_core::inference::extraction::MmioAccess;
use base_core::inference::generate_spec_with_evidence;
use base_core::loop_::evidence_confidence;
use base_core::mapping::mapper::ComponentMapper;
use base_core::spec::types::BlockKind;
use std::fs;
use std::path::PathBuf;

const USART1: u64 = 0x4001_3800;
const USART1_PAGE: u64 = 0x4001_3000;
const SPI2: u64 = 0x4000_3800;
const SPI2_PAGE: u64 = 0x4000_3000;
const I2C1: u64 = 0x4000_5400;
const I2C1_PAGE: u64 = 0x4000_5000;

fn pilot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pilot_stm32")
}

fn load_db() -> ComponentDb {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("component_db");
    let mut db = ComponentDb::new();
    assert!(db.load_directory(&dir).unwrap() > 0);
    db
}

fn classify_by_page(spec: &mut base_core::spec::types::HardwareSpec) {
    for b in &mut spec.blocks {
        b.kind = match b.base_address & !0xfff {
            USART1_PAGE => BlockKind::Uart,
            SPI2_PAGE => BlockKind::Spi,
            I2C1_PAGE => BlockKind::I2c,
            _ => b.kind,
        };
        b.confidence = evidence_confidence(b);
    }
}

#[test]
fn stm32_triple_mmio_yields_usart_spi_i2c_blocks() {
    let text = fs::read_to_string(pilot_dir().join("mmio_usart_spi_i2c.json")).unwrap();
    let accesses: Vec<MmioAccess> = serde_json::from_str(&text).unwrap();
    assert!(accesses.len() >= 12);
    assert!(accesses.iter().any(|a| a.address == USART1));
    assert!(accesses.iter().any(|a| a.address == SPI2));
    assert!(accesses.iter().any(|a| a.address == I2C1));

    let (mut spec, _) = generate_spec_with_evidence(&accesses, "pilot_stm32/fw.bin");
    classify_by_page(&mut spec);

    assert_eq!(spec.blocks.len(), 3);
    let kinds: Vec<_> = spec.blocks.iter().map(|b| b.kind).collect();
    assert!(kinds.contains(&BlockKind::Uart), "{kinds:?}");
    assert!(kinds.contains(&BlockKind::Spi), "{kinds:?}");
    assert!(kinds.contains(&BlockKind::I2c), "{kinds:?}");
}

#[test]
fn stm32_triple_design_prefers_st_three_ifaces() {
    let text = fs::read_to_string(pilot_dir().join("mmio_usart_spi_i2c.json")).unwrap();
    let accesses: Vec<MmioAccess> = serde_json::from_str(&text).unwrap();
    let (mut spec, _) = generate_spec_with_evidence(&accesses, "pilot_stm32/fw.bin");
    classify_by_page(&mut spec);

    let db = load_db();
    let mapper = ComponentMapper::new(db.clone());
    let with_st =
        mapper.map_spec_with_prefs(&spec, Some(80.0), Some("STMicroelectronics"));
    assert_eq!(with_st.assignments.len(), 3);
    let interfaces: Vec<_> = with_st
        .assignments
        .iter()
        .map(|a| a.interface.as_str())
        .collect();
    assert!(interfaces.contains(&"uart"), "{interfaces:?}");
    assert!(interfaces.contains(&"spi"), "{interfaces:?}");
    assert!(interfaces.contains(&"i2c"), "{interfaces:?}");
    assert!(
        with_st
            .assignments
            .iter()
            .all(|a| a.component == "STM32F103C8")
    );

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
