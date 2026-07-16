//! Tipos tipados de flash HIL (T4/U3).

/// Motivo de recusa de flash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashDenied {
    NotDetected,
    /// Feature `hil_programmer` ausente, ou path não ligado.
    ProgrammerUnimplemented,
    /// Feature ligada, mas falta `BASE_HIL_ALLOW_FLASH=1`.
    AllowFlashRequired,
    /// `ALLOW_FLASH` set, mas `BASE_HIL_PROGRAMMER_CMD` ausente.
    ProgrammerCmdMissing,
    /// Comando externo falhou ou I/O.
    ProgrammerFailed,
}

impl std::fmt::Display for FlashDenied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlashDenied::NotDetected => write!(
                f,
                "HIL EXPERIMENTAL: flash requer ProbePresence::Detected (CMSIS-DAP/open probe); \
                 connect() default é simulado e não grava silício"
            ),
            FlashDenied::ProgrammerUnimplemented => write!(
                f,
                "HIL EXPERIMENTAL: path Detected sem feature hil_programmer — \
                 use with_mock_flash (dry-run) ou compile --features hil_programmer"
            ),
            FlashDenied::AllowFlashRequired => write!(
                f,
                "HIL EXPERIMENTAL: programador gated — set BASE_HIL_ALLOW_FLASH=1 \
                 (NOT production flash; also set BASE_HIL_PROGRAMMER_CMD)"
            ),
            FlashDenied::ProgrammerCmdMissing => write!(
                f,
                "HIL EXPERIMENTAL: BASE_HIL_ALLOW_FLASH set but BASE_HIL_PROGRAMMER_CMD missing \
                 (e.g. 'picotool load {{image}}') — NOT production"
            ),
            FlashDenied::ProgrammerFailed => write!(
                f,
                "HIL EXPERIMENTAL: external programmer command failed — NOT production flash"
            ),
        }
    }
}

/// Recibo de flash/dry-run — **nunca** use `mode == "production"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashReceipt {
    pub bytes: usize,
    /// `mock_dry_run` | `experimental_external_cmd` — nunca `"production"`.
    pub mode: &'static str,
}
