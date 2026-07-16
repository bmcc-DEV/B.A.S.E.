//! Y2 — goldens STM32 I2C1 (event-graph + prove; verified, not overwritten).
use std::fs;
use std::path::PathBuf;

fn expected_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pilot_stm32/expected_i2c")
}

#[test]
fn stm32_i2c_event_graph_goldens_exist_and_mention_i2c1() {
    let dir = expected_dir();
    let dot = fs::read_to_string(dir.join("event_graph.dot")).expect("event_graph.dot");
    let mmd = fs::read_to_string(dir.join("event_graph.mmd")).expect("event_graph.mmd");
    assert!(dot.contains("0x40005400") || dot.contains("0x40005410"));
    assert!(dot.contains("i2c1_init_to_irq"));
    assert!(mmd.contains("IRQ 0x1f"));
    assert!(mmd.contains("i2c1_xfer_byte"));
}

#[test]
fn stm32_i2c_prove_golden_is_sat_symbolic() {
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
    assert_eq!(results[0]["contract"], "i2c1_init_to_irq");
    assert_eq!(results[1]["contract"], "i2c1_xfer_byte");
}
