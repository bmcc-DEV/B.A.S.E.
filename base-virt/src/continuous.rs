//! F3 — Diff contínuo: NDJSON (plugin) cresce → Twin↔guest + Ψ por tick.
//!
//! Offline: janelas cumulativas no ficheiro. Live: poll do outfile.
//! ≠ OS turnkey · ≠ HIL production.

use crate::source::{ingest_with_format, TraceFormat};
use crate::twin_guest::{compare_twin_guest, TwinGuestReport};
use base_core::evidence::EvidenceDb;
use base_core::honesty;
use base_core::spec::types::HardwareSpec;
use base_core::tension::TensionMetric;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ContinuousDiffConfig {
    /// Eventos por tick (crescimento da janela).
    pub window_events: usize,
    pub max_ticks: usize,
    /// Se > 0, faz poll do ficheiro até estabilizar ou timeout.
    pub poll_ms: u64,
    pub poll_timeout_sec: u64,
}

impl Default for ContinuousDiffConfig {
    fn default() -> Self {
        Self {
            window_events: 4,
            max_ticks: 32,
            poll_ms: 0,
            poll_timeout_sec: 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuousTick {
    pub index: usize,
    pub evidence_count: usize,
    pub hit_rate: f64,
    pub hits: usize,
    pub misses: usize,
    pub psi_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuousDiffReport {
    pub phase: String,
    pub ok: bool,
    pub ticks: Vec<ContinuousTick>,
    pub final_hit_rate: f64,
    pub final_psi: f64,
    pub total_evidence: usize,
    pub live_polled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_twin: Option<TwinGuestReport>,
    pub generates_os: bool,
    pub auto_fix_complete: bool,
    pub honesty: String,
    pub note: String,
}

fn evidence_prefix(full: &EvidenceDb, n: usize) -> EvidenceDb {
    let mut db = EvidenceDb::new(&format!("{}#c{}", full.source, n));
    for e in full.entries.iter().take(n) {
        db.add(e.clone());
    }
    db
}

fn tick_from(spec: &HardwareSpec, guest: &EvidenceDb, index: usize) -> ContinuousTick {
    let twin = compare_twin_guest(spec, guest);
    let psi = TensionMetric::compute(guest, spec, 0, 0, 0).overall_confidence;
    ContinuousTick {
        index,
        evidence_count: guest.count(),
        hit_rate: twin.hit_rate,
        hits: twin.hits,
        misses: twin.misses,
        psi_confidence: psi,
    }
}

/// Offline / pós-run: NDJSON completo em janelas cumulativas.
pub fn run_continuous_diff_file(
    spec: &HardwareSpec,
    trace_path: &Path,
    cfg: &ContinuousDiffConfig,
) -> anyhow::Result<ContinuousDiffReport> {
    let mut live_polled = false;
    let data = if cfg.poll_ms > 0 {
        live_polled = true;
        poll_until_stable(trace_path, cfg)?
    } else {
        std::fs::read(trace_path)?
    };

    let full = ingest_with_format(&data, "continuous", TraceFormat::Auto)?;
    let n = full.count();
    let step = cfg.window_events.max(1);
    let mut ticks = Vec::new();
    let mut last_twin = None;
    let mut end = step.min(n.max(1));
    let mut idx = 0usize;

    if n == 0 {
        return Ok(ContinuousDiffReport {
            phase: "continuous_diff".into(),
            ok: false,
            ticks,
            final_hit_rate: 0.0,
            final_psi: 0.0,
            total_evidence: 0,
            live_polled,
            last_twin: None,
            generates_os: honesty::GENERATES_OS,
            auto_fix_complete: honesty::AUTO_FIX_COMPLETE,
            honesty: honesty::NOTE.to_string(),
            note: "empty trace".into(),
        });
    }

    while idx < cfg.max_ticks && end <= n {
        let slice = evidence_prefix(&full, end);
        let twin = compare_twin_guest(spec, &slice);
        ticks.push(tick_from(spec, &slice, idx));
        last_twin = Some(twin);
        if end >= n {
            break;
        }
        end = (end + step).min(n);
        idx += 1;
    }

    let final_hit = ticks.last().map(|t| t.hit_rate).unwrap_or(0.0);
    let final_psi = ticks.last().map(|t| t.psi_confidence).unwrap_or(0.0);

    Ok(ContinuousDiffReport {
        phase: "continuous_diff".into(),
        ok: true,
        ticks,
        final_hit_rate: final_hit,
        final_psi,
        total_evidence: n,
        live_polled,
        last_twin,
        generates_os: honesty::GENERATES_OS,
        auto_fix_complete: honesty::AUTO_FIX_COMPLETE,
        honesty: honesty::NOTE.to_string(),
        note: "Continuous plugin NDJSON ↔ twin — ≠ OS turnkey".into(),
    })
}

fn poll_until_stable(path: &Path, cfg: &ContinuousDiffConfig) -> anyhow::Result<Vec<u8>> {
    let start = Instant::now();
    let timeout = Duration::from_secs(cfg.poll_timeout_sec.max(1));
    let mut last_len = 0usize;
    let mut stable_rounds = 0u32;
    loop {
        if path.exists() {
            let data = std::fs::read(path)?;
            if data.len() == last_len && !data.is_empty() {
                stable_rounds += 1;
                if stable_rounds >= 3 {
                    return Ok(data);
                }
            } else {
                stable_rounds = 0;
                last_len = data.len();
            }
        }
        if start.elapsed() >= timeout {
            if path.exists() {
                return Ok(std::fs::read(path)?);
            }
            anyhow::bail!("trace file not ready: {}", path.display());
        }
        std::thread::sleep(Duration::from_millis(cfg.poll_ms.max(50)));
    }
}

impl ContinuousDiffReport {
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn spec() -> HardwareSpec {
        let mut s = HardwareSpec::empty();
        s.blocks.push(FunctionalBlock {
            id: "m".into(),
            kind: BlockKind::Unknown,
            base_address: 0xA00000,
            size: 0x1000,
            registers: vec![],
            protocol: Protocol {
                states: vec![],
                transitions: vec![],
                entry_condition: None,
                exit_condition: None,
            },
            timing: TimingProfile {
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
        s
    }

    #[test]
    fn continuous_windows_grow() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, r#"{{"op":"mmio_write","addr":"0xA00000","value":"0x1"}}"#).unwrap();
        writeln!(f, r#"{{"op":"mmio_write","addr":"0xA00004","value":"0x2"}}"#).unwrap();
        writeln!(f, r#"{{"op":"mmio_read","addr":"0xA00000"}}"#).unwrap();
        writeln!(f, r#"{{"op":"mmio_read","addr":"0xA00004"}}"#).unwrap();
        writeln!(f, r#"{{"op":"mmio_write","addr":"0xA00008","value":"0x3"}}"#).unwrap();
        let cfg = ContinuousDiffConfig {
            window_events: 2,
            max_ticks: 8,
            poll_ms: 0,
            ..Default::default()
        };
        let report = run_continuous_diff_file(&spec(), f.path(), &cfg).unwrap();
        assert!(report.ok);
        assert!(report.ticks.len() >= 2);
        assert_eq!(report.total_evidence, 5);
        assert!(!report.generates_os);
        assert!(report.final_hit_rate > 0.9);
    }
}
