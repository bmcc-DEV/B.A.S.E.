//! Honesty gates for static recompilation.

use base_core::honesty::{AUTO_FIX_COMPLETE, GENERATES_OS};

/// Pipeline still incomplete as a product.
pub const STATIC_RECOMP_COMPLETE: bool = false;

/// Win32 / PE imports / SEH not complete.
pub const WIN32_ABI_COMPLETE: bool = false;

/// Never claim arbitrary PE execution.
pub const RUNS_ANY_PE: bool = false;

/// Sega Saturn guest runtime not provided.
pub const RUNS_ON_SATURN: bool = false;

/// Dreamcast guest runtime not provided.
pub const RUNS_ON_DREAMCAST: bool = false;

pub const BANNER: &str = "≠ Wine / ≠ Win32 completo / ≠ runtime Saturn·DC: `static_recomp_complete: false` · `win32_abi_complete: false` · `runs_any_pe: false` · `runs_on_saturn: false` · `runs_on_dreamcast: false`";

pub fn markdown_section() -> String {
    format!(
        "## Honesty (static recomp)\n\n- {}\n- `generates_os: {}` · `auto_fix_complete: {}`\n- `static_recomp_complete: {}` · `win32_abi_complete: {}` · `runs_any_pe: {}`\n- `runs_on_saturn: {}` · `runs_on_dreamcast: {}`\n",
        BANNER,
        GENERATES_OS,
        AUTO_FIX_COMPLETE,
        STATIC_RECOMP_COMPLETE,
        WIN32_ABI_COMPLETE,
        RUNS_ANY_PE,
        RUNS_ON_SATURN,
        RUNS_ON_DREAMCAST,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn honesty_flags_stay_false() {
        assert!(!STATIC_RECOMP_COMPLETE);
        assert!(!WIN32_ABI_COMPLETE);
        assert!(!RUNS_ANY_PE);
        assert!(!RUNS_ON_SATURN);
        assert!(!RUNS_ON_DREAMCAST);
        assert!(!GENERATES_OS);
    }
}
