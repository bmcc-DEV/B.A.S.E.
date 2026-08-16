//! Console runtime stubs — honesty only (≠ emulator · ≠ Jo Engine).

use crate::honesty::{RUNS_ON_DREAMCAST, RUNS_ON_SATURN};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleTarget {
    SaturnSh2,
    DreamcastSh4,
}

#[derive(Debug, Clone)]
pub struct RuntimeStatus {
    pub target: ConsoleTarget,
    pub runs: bool,
    pub note: &'static str,
}

pub fn runtime_status(target: ConsoleTarget) -> RuntimeStatus {
    match target {
        ConsoleTarget::SaturnSh2 => RuntimeStatus {
            target,
            runs: RUNS_ON_SATURN,
            note: "Encode SH-2 bytes only — no VDP1/VDP2 / CD / SMPC guest. Use external emulator (Mednafen/Yabause) manually.",
        },
        ConsoleTarget::DreamcastSh4 => RuntimeStatus {
            target,
            runs: RUNS_ON_DREAMCAST,
            note: "Encode SH-4 bytes only — no PowerVR / maple guest. Use external emulator (Flycast) manually.",
        },
    }
}

pub fn markdown_runtime() -> String {
    let s = runtime_status(ConsoleTarget::SaturnSh2);
    let d = runtime_status(ConsoleTarget::DreamcastSh4);
    format!(
        "## Runtime (stub)\n\n- Saturn SH-2: `runs_on_saturn: {}` — {}\n- Dreamcast SH-4: `runs_on_dreamcast: {}` — {}\n",
        s.runs, s.note, d.runs, d.note
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stubs_do_not_claim_run() {
        assert!(!runtime_status(ConsoleTarget::SaturnSh2).runs);
        assert!(!runtime_status(ConsoleTarget::DreamcastSh4).runs);
    }
}
