//! Specter Live — VM comportamental (QEMU primário).
//!
//! Ingest → Ψ → study · Plugin/QMP · TraceSource · Twin↔guest · BIR twin (v1.6).
//! ≠ OS turnkey · ≠ HIL production.

pub mod bir_bridge;
pub mod continuous;
pub mod live;
pub mod qemu;
pub mod qmp;
pub mod session;
pub mod source;
pub mod study_live;
pub mod trace;
pub mod twin_guest;

pub use bir_bridge::{replay_bir_twin, spec_block_to_bir, BirTwinReplayReport};
pub use continuous::{
    run_continuous_diff_file, ContinuousDiffConfig, ContinuousDiffReport, ContinuousTick,
};
pub use live::{run_live_windows, LiveConfig, LiveWindowScore};
pub use qemu::{
    format_plugin_cli, launch_qemu, resolve_qemu_bin, spawn_qemu_live, QemuLaunchOpts,
    QemuLaunchResult, QemuLiveSession,
};
pub use qmp::{probe_savevm, probe_session, QmpClient, QmpError};
pub use session::{VirtSessionReport, VirtSessionWindow};
pub use source::{
    ingest_libretro, ingest_mame, ingest_path_with_format, ingest_with_format, LibretroSource,
    MameSource, NdjsonSource, TraceFormat, TraceSource,
};
pub use study_live::{load_evidence_flexible, run_live_study, LiveStudyReport};
pub use trace::{
    ingest_ndjson, ingest_ndjson_path, parse_ndjson_line, TraceEvent, TraceSourceError,
};
pub use twin_guest::{compare_twin_guest, TwinGuestReport};
