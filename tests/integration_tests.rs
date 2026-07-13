//! End-to-end integration tests for B.A.S.E. pipeline.
//!
//! Tests the full pipeline inference → mapping → PCB → FW.

use base_core::component_db::ComponentDb;
use base_core::inference::extraction::{MmioAccess, MmioAccessType};
use base_core::inference::generate_spec;
use base_core::mapping::mapper::ComponentMapper;
use base_core::mapping::netlist::generate_netlist;

#[test]
fn test_pipeline_amiga_cd32_inference() {
    // Simulate Amiga CD32 chipset MMIO accesses
    let accesses = vec![
        // Video init
        MmioAccess { address: 0xDFF096, value: Some(0x0000), access_type: MmioAccessType::Write, function_name: "init".into(), instruction_addr: 10 },
        MmioAccess { address: 0xDFF180, value: Some(0x0000), access_type: MmioAccessType::Write, function_name: "palette".into(), instruction_addr: 20 },
        MmioAccess { address: 0xDFF182, value: Some(0x0FFF), access_type: MmioAccessType::Write, function_name: "palette".into(), instruction_addr: 22 },
        MmioAccess { address: 0xDFF0E0, value: Some(0x00200000), access_type: MmioAccessType::Write, function_name: "video".into(), instruction_addr: 30 },
        // Audio init
        MmioAccess { address: 0xDFF0A6, value: Some(124), access_type: MmioAccessType::Write, function_name: "audio".into(), instruction_addr: 40 },
        MmioAccess { address: 0xDFF0A8, value: Some(64), access_type: MmioAccessType::Write, function_name: "audio".into(), instruction_addr: 42 },
        // DMA enable
        MmioAccess { address: 0xDFF096, value: Some(0x8320), access_type: MmioAccessType::Write, function_name: "dma".into(), instruction_addr: 50 },
        // Polling
        MmioAccess { address: 0xDFF004, value: None, access_type: MmioAccessType::Read, function_name: "vblank".into(), instruction_addr: 60 },
        MmioAccess { address: 0xDFF004, value: None, access_type: MmioAccessType::Read, function_name: "vblank".into(), instruction_addr: 62 },
        MmioAccess { address: 0xDFF004, value: None, access_type: MmioAccessType::Read, function_name: "vblank".into(), instruction_addr: 64 },
        MmioAccess { address: 0xDFF004, value: None, access_type: MmioAccessType::Read, function_name: "vblank".into(), instruction_addr: 66 },
        // IRQ
        MmioAccess { address: 0xDFF01C, value: Some(0x0008), access_type: MmioAccessType::Read, function_name: "irq".into(), instruction_addr: 70 },
        MmioAccess { address: 0xDFF09C, value: Some(0x0008), access_type: MmioAccessType::Write, function_name: "irq".into(), instruction_addr: 72 },
    ];

    // Stage 1-4: Generate spec
    let spec = generate_spec(&accesses, "Amiga CD32 test");
    assert!(!spec.blocks.is_empty(), "Should detect at least one block");
    assert!(spec.confidence > 0.0, "Should have confidence > 0");

    // Check block detection
    let has_mmio_region = spec.blocks.iter().any(|b| b.base_address == 0xDFF000);
    assert!(has_mmio_region, "Should detect 0xDFF000 MMIO region");

    // Check register detection
    let total_regs: usize = spec.blocks.iter().map(|b| b.registers.len()).sum();
    assert!(total_regs > 0, "Should detect registers");
}

#[test]
fn test_pipeline_amiga_cd32_mapping() {
    // Load component DB
    let db_path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/base-core/component_db"));
    let mut db = ComponentDb::new();
    if db_path.exists() {
        let _ = db.load_directory(db_path);
    }

    // Create minimal spec
    let accesses = vec![
        MmioAccess { address: 0xDFF096, value: Some(0x0000), access_type: MmioAccessType::Write, function_name: "init".into(), instruction_addr: 10 },
    ];
    let spec = generate_spec(&accesses, "CD32 mapping test");

    // Map to components
    let mapper = ComponentMapper::new(db);
    let synthesized = mapper.map_spec(&spec);

    // Generate netlist
    let db2 = ComponentDb::new();
    let netlist = generate_netlist(&synthesized, &db2);

    // At minimum, the pipeline should not crash and produce valid output
    let _yaml = serde_yaml::to_string(&synthesized).expect("Should serialize");
}

#[test]
fn test_specterprobe_integration() {
    // Verify that specterprobe can lift and produce output that base-core can consume
    let data = vec![
        0x00, 0x04, 0x00, 0x91, // add x0, x0, #1
        0xC0, 0x03, 0x5F, 0xD6, // ret
    ];
    let output = specterprobe::lift::lift_binary(&data);
    assert!(output.total_instructions >= 2);
    assert!(!output.ir_text.is_empty());

    // The IR text should contain SSA variables (from our improved lift)
    assert!(output.ir_text.contains("%v"), "IR should use SSA variables");
}
