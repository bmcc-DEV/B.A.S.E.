//! Specter Live — VM comportamental (QEMU primário).
//!
//! Ingest NDJSON MMIO/IRQ → [`EvidenceDb`] → Ψ em janelas.
//! Plugin TCG (`plugin/`) + QMP (`qmp`) — ≠ OS turnkey · ≠ HIL production.

pub mod live;
pub mod qemu;
pub mod qmp;
pub mod session;
pub mod trace;

pub use live::{run_live_windows, LiveConfig, LiveWindowScore};
pub use qemu::{
    format_plugin_cli, launch_qemu, resolve_qemu_bin, spawn_qemu_live, QemuLaunchOpts,
    QemuLaunchResult, QemuLiveSession,
};
pub use qmp::{probe_session, QmpClient, QmpError};
pub use session::{VirtSessionReport, VirtSessionWindow};
pub use trace::{
    ingest_ndjson, ingest_ndjson_path, parse_ndjson_line, TraceEvent, TraceSourceError,
};
