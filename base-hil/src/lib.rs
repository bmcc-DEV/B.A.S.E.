//! HIL Cluster — **EXPERIMENTAL** template (host agent + gerador de stub de firmware).
//!
//! - Compila e testa no host **sem** hardware.
//! - Não flashea silício sem [`agent::ProbePresence::Detected`].
//! - Não entra no `base pipeline` default.

pub mod agent;
pub mod probe;

pub use agent::{
    FlashDenied, FlashReceipt, HilAgent, HilSample, ProbePresence, ENV_MOCK_DETECTED,
};
pub use probe::ProbeFirmware;
