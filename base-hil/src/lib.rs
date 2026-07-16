//! HIL Cluster — **host REAL\*** (enumerate + mock); production gated.
//!
//! - Compila e testa no host **sem** hardware.
//! - Enumerate USB: feature opt-in `hil_usb` (não no CI default).
//! - Programador: feature opt-in `hil_programmer` + envs (não production).
//! - Não flashea silício sem [`agent::ProbePresence::Detected`].
//! - Não entra no `base pipeline` default.

pub mod agent;
pub mod flash;
pub mod lab_gate;
pub mod probe;
pub mod programmer;
mod usb;

pub use agent::{HilAgent, HilSample, ProbePresence, DEFAULT_PROBE_PID, DEFAULT_PROBE_VID, ENV_MOCK_DETECTED};
pub use flash::{FlashDenied, FlashReceipt};
pub use lab_gate::{evaluate_lab_gate, evaluate_lab_gate_opts, LabGateOptions, LabGateReport};
pub use probe::ProbeFirmware;
pub use programmer::{programmer_feature_enabled, ENV_ALLOW_FLASH, ENV_PROGRAMMER_CMD};

