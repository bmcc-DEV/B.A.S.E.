//! R2 — pilot contracts / replay / prove
use base_bir::bir_to_sequence_contracts;
use base_bsl::compile;
use base_core::replay::{parse_saleae_csv, ReplayEngine};
use base_core::smt::SmtProver;
use base_core::temporal::SequenceContract;
use std::fs;
use std::path::PathBuf;

fn pilot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pilot")
}

fn load_contracts_yaml(path: &PathBuf) -> Vec<SequenceContract> {
    let text = fs::read_to_string(path).unwrap();
    serde_yaml::from_str(&text).unwrap()
}

#[test]
fn pilot_bsl_compiles_and_exports_saleae_contracts() {
    let src = fs::read_to_string(pilot_dir().join("pilot.bsl")).unwrap();
    let device = compile(&src).expect("pilot.bsl should compile");
    let v = device.validate();
    assert!(
        v.errors.is_empty(),
        "BIR validation errors: {:?}",
        v.errors
    );

    let temporal = bir_to_sequence_contracts(&device);
    assert!(!temporal.is_empty());
    assert!(temporal.iter().any(|c| {
        c.steps.iter().any(|s| s.event_type == "mmio_write")
            && c.steps.iter().any(|s| s.event_type == "irq")
    }));

    // YAML bridge → SequenceContract
    let yaml = serde_yaml::to_string(&temporal).unwrap();
    let contracts: Vec<SequenceContract> = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(contracts.len(), temporal.len());
}

#[test]
fn pilot_replay_pass_trace() {
    let csv = fs::read_to_string(pilot_dir().join("trace.csv")).unwrap();
    let events = parse_saleae_csv(&csv);
    assert_eq!(events.len(), 6);

    let contracts = load_contracts_yaml(&pilot_dir().join("contracts.yaml"));
    let engine = ReplayEngine::new(contracts);
    let result = engine.replay(&events);
    assert!(result.summary.total_sequences_found >= 1);
    assert_eq!(result.summary.failed, 0, "pass trace should have 0 violations");
}

#[test]
fn pilot_replay_fail_trace_detects_violations() {
    let csv = fs::read_to_string(pilot_dir().join("trace_fail.csv")).unwrap();
    let events = parse_saleae_csv(&csv);
    let contracts = load_contracts_yaml(&pilot_dir().join("contracts.yaml"));
    let engine = ReplayEngine::new(contracts);
    let result = engine.replay(&events);
    assert!(
        result.summary.failed > 0 || !result.violations.is_empty(),
        "fail trace should violate latency contracts"
    );
}

#[test]
fn pilot_replay_via_bir() {
    let src = fs::read_to_string(pilot_dir().join("pilot.bsl")).unwrap();
    let device = compile(&src).unwrap();
    let temporal = bir_to_sequence_contracts(&device);
    let yaml = serde_yaml::to_string(&temporal).unwrap();
    let contracts: Vec<SequenceContract> = serde_yaml::from_str(&yaml).unwrap();

    let csv = fs::read_to_string(pilot_dir().join("trace.csv")).unwrap();
    let events = parse_saleae_csv(&csv);
    let engine = ReplayEngine::new(contracts);
    let result = engine.replay(&events);
    assert!(
        result.summary.total_sequences_found >= 1,
        "BIR-derived contracts should match pass trace"
    );
    assert_eq!(
        result.summary.failed, 0,
        "pass trace must not violate BIR contracts: {:?}",
        result.violations
    );
}

#[test]
fn pilot_prove_sat_and_unsat() {
    let sat = load_contracts_yaml(&pilot_dir().join("contracts.yaml"));
    let report = SmtProver::prove_all(&sat);
    assert!(report.all_satisfied);
    assert_eq!(report.contracts_proved, sat.len());

    let unsat = load_contracts_yaml(&pilot_dir().join("contracts.unsat.yaml"));
    let bad = SmtProver::prove_all(&unsat);
    assert!(!bad.all_satisfied);
    assert_eq!(bad.contracts_proved, 0);
}
