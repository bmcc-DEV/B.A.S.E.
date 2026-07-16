//! X2 — goldens RP (event-graph + prove; verified, not overwritten).
use std::fs;
use std::path::PathBuf;

fn expected_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pilot/expected")
}

#[test]
fn rp_event_graph_goldens_exist_and_mention_uart() {
    let dir = expected_dir();
    let dot = fs::read_to_string(dir.join("event_graph.dot")).expect("event_graph.dot");
    let mmd = fs::read_to_string(dir.join("event_graph.mmd")).expect("event_graph.mmd");
    assert!(dot.contains("0x40034000"));
    assert!(dot.contains("uart_init_to_irq"));
    assert!(mmd.contains("IRQ 0x10"));
    assert!(mmd.contains("uart_tx_byte"));
}

#[test]
fn rp_prove_golden_is_sat_symbolic() {
    let text = fs::read_to_string(expected_dir().join("proof_report.golden.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["backend"], "symbolic");
    assert_eq!(v["contracts_proved"], 2);
    assert_eq!(v["all_satisfied"], true);
    let results = v["results"].as_array().expect("results");
    assert_eq!(results.len(), 2);
    for r in results {
        assert_eq!(r["proved"], true);
        assert_eq!(r["satisfiable"], true);
        assert_eq!(r["backend"], "symbolic");
    }
}
