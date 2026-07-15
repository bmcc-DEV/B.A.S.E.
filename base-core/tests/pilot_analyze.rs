//! R1 — pilot analyze stability tests.
use base_core::inference::extraction::{MmioAccess, MmioAccessType};
use base_core::inference::{generate_spec, generate_spec_with_evidence};
use base_core::loop_::{evidence_confidence, FeedbackLoop};
use base_core::spec::types::{BlockKind, HardwareSpec};
use std::fs;
use std::path::PathBuf;

fn pilot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pilot")
}

#[test]
fn pilot_mmio_json_loads_and_builds_uart_spec() {
    let path = pilot_dir().join("mmio.json");
    assert!(path.exists(), "examples/pilot/mmio.json must exist");
    let text = fs::read_to_string(&path).unwrap();
    let accesses: Vec<MmioAccess> = serde_json::from_str(&text).unwrap();
    assert_eq!(accesses.len(), 4);

    let (mut spec, evidence) = generate_spec_with_evidence(&accesses, "pilot/fw.bin");
    assert_eq!(evidence.entries.len(), 4);
    assert!(!spec.blocks.is_empty());

    // Simulate --classify uart
    for b in &mut spec.blocks {
        b.kind = BlockKind::Uart;
        b.confidence = evidence_confidence(b);
    }
    assert!(spec.blocks.iter().all(|b| b.kind == BlockKind::Uart));

    // Golden allowlist fields must serialize
    let yaml = spec.to_yaml().unwrap();
    for key in ["version", "source", "blocks", "confidence"] {
        assert!(yaml.contains(key), "missing field {}", key);
    }
    assert!(yaml.contains("40034000") || yaml.contains("1073954816"));
}

#[test]
fn empty_mmio_yields_empty_evidence_no_panic() {
    let (spec, evidence) = generate_spec_with_evidence(&[], "empty.bin");
    assert!(spec.blocks.is_empty());
    assert_eq!(spec.confidence, 0.0);
    assert!(evidence.entries.is_empty());
}

#[test]
fn reconstruct_stable_when_structurally_complete() {
    let mut spec = HardwareSpec::empty();
    spec.blocks.push(base_core::spec::types::FunctionalBlock {
        id: "uart_0".into(),
        kind: BlockKind::Uart,
        base_address: 0x40034000,
        size: 0x1000,
        registers: vec![base_core::spec::types::Register {
            offset: 0,
            name: Some("dr".into()),
            width: 32,
            access: base_core::spec::types::AccessType::ReadWrite,
            purpose: base_core::spec::types::RegisterPurpose::DataPort,
            reset_value: None,
            observed_values: vec![],
            bitfields: vec![],
            polling: false,
            count: 1,
        }],
        protocol: base_core::spec::types::Protocol {
            states: vec!["idle".into(), "busy".into()],
            transitions: vec![],
            entry_condition: None,
            exit_condition: None,
        },
        timing: base_core::spec::types::TimingProfile {
            activation: None,
            processing: None,
            interrupt_response: None,
            dma_setup: None,
            polling_interval: None,
        },
        dma: None,
        dependencies: vec![],
        confidence: 0.5,
    });
    let before = evidence_confidence(&spec.blocks[0]);
    let mut loop_ = FeedbackLoop::new(0.99, 3);
    let iter = loop_.iterate(&spec, 1);
    assert_eq!(iter.structural_changes, 0);
    assert!((iter.spec.blocks[0].confidence - before).abs() < 0.001);
}

#[test]
fn golden_fields_file_lists_required_keys() {
    let path = pilot_dir().join("expected/hardware_spec.fields.yaml");
    assert!(path.exists());
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains("required_top_level"));
    assert!(text.contains("blocks"));
}

#[test]
fn unused_generate_spec_still_works() {
    let a = MmioAccess {
        address: 0x40034000,
        value: Some(1),
        access_type: MmioAccessType::Write,
        function_name: "w".into(),
        instruction_addr: 0,
    };
    let _ = generate_spec(&[a], "t");
}
