//! HIL host REAL* — enumerate + mock flash never production.
use base_hil::{FlashReceipt, HilAgent, ProbePresence};

#[test]
fn hil_enumerate_simulated() {
    let p = HilAgent::enumerate_presence(0xcafe, 0x4007);
    assert!(matches!(
        p,
        ProbePresence::Simulated | ProbePresence::Detected
    ));
}

#[test]
fn hil_mock_flash_receipt_not_production() {
    let agent = HilAgent::with_mock_flash(ProbePresence::Detected);
    let receipt = agent.try_flash(b"fw").expect("mock flash");
    assert_eq!(
        receipt,
        FlashReceipt {
            bytes: 2,
            mode: "mock_dry_run",
        }
    );
    assert_ne!(receipt.mode, "production");
}
