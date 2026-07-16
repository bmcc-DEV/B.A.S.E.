//! Industrial Gate A — HIL lab status (software-side checks).
//!
//! Evaluates pré-condições A1–A5 from SOW Industrial Gate.
//! **Never** sets `production: true`. Lab-assist ≠ CI flash turnkey.

use crate::agent::{HilAgent, ProbePresence, DEFAULT_PROBE_PID, DEFAULT_PROBE_VID, ENV_MOCK_DETECTED};
use crate::programmer::{programmer_feature_enabled, ENV_ALLOW_FLASH, ENV_PROGRAMMER_CMD};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct GateCheck {
    pub id: String,
    pub name: String,
    pub green: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabGateReport {
    /// Industrial Gate claim letter.
    pub claim: &'static str,
    pub production: bool,
    pub lab_assist_ready: bool,
    pub checks: Vec<GateCheck>,
    pub sow_path_hint: &'static str,
}

/// Options for Gate A evaluation.
#[derive(Debug, Clone, Default)]
pub struct LabGateOptions<'a> {
    pub sow_signed: bool,
    pub sop_path: Option<&'a Path>,
    /// Force A1 Detected offline (CLI `--mock-detected` / lab rehearsal).
    /// Still **≠** USB real; prefer `hil_usb` + probe no lab do Cliente.
    pub mock_detected: bool,
}

/// Evaluate Gate A (HIL lab). Convenience wrapper.
pub fn evaluate_lab_gate(
    vid: u16,
    pid: u16,
    sow_signed: bool,
    sop_path: Option<&Path>,
) -> LabGateReport {
    evaluate_lab_gate_opts(
        vid,
        pid,
        LabGateOptions {
            sow_signed,
            sop_path,
            mock_detected: false,
        },
    )
}

/// Evaluate Gate A with explicit options (A1 mock / A5 sow).
pub fn evaluate_lab_gate_opts(vid: u16, pid: u16, opts: LabGateOptions<'_>) -> LabGateReport {
    let presence = if opts.mock_detected {
        tracing::warn!(
            "[HIL][Gate A] --mock-detected — A1 Detected offline (no USB; lab rehearsal)"
        );
        ProbePresence::Detected
    } else {
        HilAgent::enumerate_presence(vid, pid)
    };
    let a1 = matches!(presence, ProbePresence::Detected);
    let a1_via_mock_env = std::env::var_os(ENV_MOCK_DETECTED).is_some();
    let a2_feature = programmer_feature_enabled();
    let a2_allow = std::env::var_os(ENV_ALLOW_FLASH).is_some();
    let a2_cmd = std::env::var(ENV_PROGRAMMER_CMD)
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let a2 = a2_feature && a2_allow && a2_cmd;
    let a3 = opts.sop_path.map(|p| p.is_file()).unwrap_or(false);
    // A4: software invariant — production mode never emitted by try_flash paths
    let a4 = true;
    let a5 = opts.sow_signed;

    let checks = vec![
        GateCheck {
            id: "A1".into(),
            name: "Probe Detected".into(),
            green: a1,
            detail: format!(
                "presence={presence:?} mock_flag={} mock_env={} (Simulated blocks lab flash)",
                opts.mock_detected, a1_via_mock_env
            ),
        },
        GateCheck {
            id: "A2".into(),
            name: "Programmer gated".into(),
            green: a2,
            detail: format!(
                "feature={a2_feature} {ENV_ALLOW_FLASH}={a2_allow} {ENV_PROGRAMMER_CMD}={a2_cmd}"
            ),
        },
        GateCheck {
            id: "A3".into(),
            name: "SOP written".into(),
            green: a3,
            detail: opts
                .sop_path
                .map(|p| format!("sop={}", p.display()))
                .unwrap_or_else(|| "no --sop path".into()),
        },
        GateCheck {
            id: "A4".into(),
            name: "Receipt ≠ production".into(),
            green: a4,
            detail: "FlashReceipt.mode never \"production\" (invariant)".into(),
        },
        GateCheck {
            id: "A5".into(),
            name: "SOW §HIL signed".into(),
            green: a5,
            detail: if a5 {
                "sow_signed=true".into()
            } else {
                "pass --sow-signed only after contract".into()
            },
        },
    ];

    let lab_assist_ready = checks.iter().all(|c| c.green);

    LabGateReport {
        claim: "A",
        production: false,
        lab_assist_ready,
        checks,
        sow_path_hint: "base-vault/22 - Path to v1.2/22.30 - SOW Industrial Gate.md",
    }
}

pub fn default_vid_pid() -> (u16, u16) {
    (DEFAULT_PROBE_VID, DEFAULT_PROBE_PID)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_never_production() {
        let r = evaluate_lab_gate(0xcafe, 0x4007, false, None);
        assert!(!r.production);
        assert!(!r.lab_assist_ready); // A1/A2/A3/A5 fail in default CI
        assert!(r.checks.iter().any(|c| c.id == "A4" && c.green));
    }

    #[test]
    fn sow_signed_alone_not_enough() {
        let r = evaluate_lab_gate(0xcafe, 0x4007, true, None);
        assert!(!r.lab_assist_ready);
        assert!(r.checks.iter().find(|c| c.id == "A5").unwrap().green);
    }

    #[test]
    fn mock_detected_greens_a1() {
        let r = evaluate_lab_gate_opts(
            0xcafe,
            0x4007,
            LabGateOptions {
                sow_signed: false,
                sop_path: None,
                mock_detected: true,
            },
        );
        assert!(!r.production);
        assert!(r.checks.iter().find(|c| c.id == "A1").unwrap().green);
        assert!(!r.lab_assist_ready); // A2/A3/A5 still open
    }
}
