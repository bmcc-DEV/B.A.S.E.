//! Integration: reconstruct stop_reason is REAL* (≠ auto-fix).
use base_core::inference::extraction::MmioAccess;
use base_core::inference::generate_spec_with_evidence;
use base_core::loop_::{FeedbackLoop, StopReason};
use base_core::spec::types::{BlockKind, HardwareSpec};
use std::fs;
use std::path::PathBuf;

fn pilot_spec() -> HardwareSpec {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/pilot/out/analyze/hardware_spec.yaml");
    if path.exists() {
        let yaml = fs::read_to_string(&path).unwrap();
        HardwareSpec::from_yaml(&yaml).unwrap()
    } else {
        let mmio = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pilot/mmio.json");
        let text = fs::read_to_string(mmio).unwrap();
        let accesses: Vec<MmioAccess> = serde_json::from_str(&text).unwrap();
        let (mut spec, _) = generate_spec_with_evidence(&accesses, "pilot");
        for b in &mut spec.blocks {
            b.kind = BlockKind::Uart;
        }
        spec
    }
}

#[test]
fn reconstruct_reports_stop_reason_not_autofix() {
    let spec = pilot_spec();
    let mut loop_ = FeedbackLoop::new(0.99, 32);
    let _ = loop_.run(&spec);
    let report = loop_.convergence_report();
    assert!(matches!(
        report.stop_reason,
        StopReason::Converged | StopReason::Stagnated | StopReason::MaxIterations
    ));
    assert_eq!(
        report.stagnated,
        report.stop_reason == StopReason::Stagnated
    );
}
