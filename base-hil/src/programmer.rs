//! Programador EXPERIMENTAL (feature `hil_programmer`).
//!
//! **Não** é flash de produção. Requer `BASE_HIL_ALLOW_FLASH=1` + comando externo.

use crate::flash::{FlashDenied, FlashReceipt};

/// Opt-in explícito para invocar comando externo (ainda ≠ production).
pub const ENV_ALLOW_FLASH: &str = "BASE_HIL_ALLOW_FLASH";

/// Template do comando (ex.: `picotool load {image}`). `{image}` = path do binário temporário.
pub const ENV_PROGRAMMER_CMD: &str = "BASE_HIL_PROGRAMMER_CMD";

/// Feature `hil_programmer` compilada?
pub fn programmer_feature_enabled() -> bool {
    cfg!(feature = "hil_programmer")
}

/// Tenta flash EXPERIMENTAL via comando externo.
#[cfg(feature = "hil_programmer")]
pub fn try_experimental_flash(image: &[u8]) -> Result<FlashReceipt, FlashDenied> {
    if std::env::var_os(ENV_ALLOW_FLASH).is_none() {
        tracing::warn!(
            "[HIL][EXPERIMENTAL] programmer path gated: set {ENV_ALLOW_FLASH}=1 to invoke external cmd \
             (NOT production flash)"
        );
        return Err(FlashDenied::AllowFlashRequired);
    }

    let template = match std::env::var(ENV_PROGRAMMER_CMD) {
        Ok(s) if !s.trim().is_empty() => s,
        _ => {
            tracing::warn!(
                "[HIL][EXPERIMENTAL] {ENV_ALLOW_FLASH} set but {ENV_PROGRAMMER_CMD} missing"
            );
            return Err(FlashDenied::ProgrammerCmdMissing);
        }
    };

    let img_path =
        std::env::temp_dir().join(format!("base_hil_flash_{}_{}.bin", std::process::id(), {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        }));
    if std::fs::write(&img_path, image).is_err() {
        return Err(FlashDenied::ProgrammerFailed);
    }

    let img_str = img_path.to_string_lossy().into_owned();
    let cmdline = template.replace("{image}", &img_str);

    tracing::warn!(
        "[HIL][EXPERIMENTAL] invoking external programmer (NOT production): {}",
        cmdline
    );

    let status = match run_shell(&cmdline) {
        Ok(s) => s,
        Err(_) => {
            let _ = std::fs::remove_file(&img_path);
            return Err(FlashDenied::ProgrammerFailed);
        }
    };
    let _ = std::fs::remove_file(&img_path);

    if !status.success() {
        tracing::error!(
            "[HIL][EXPERIMENTAL] programmer cmd failed status={:?}",
            status.code()
        );
        return Err(FlashDenied::ProgrammerFailed);
    }

    Ok(FlashReceipt {
        bytes: image.len(),
        mode: "experimental_external_cmd",
    })
}

#[cfg(feature = "hil_programmer")]
fn run_shell(cmdline: &str) -> Result<std::process::ExitStatus, std::io::Error> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(cmdline)
        .status()
}

#[cfg(not(feature = "hil_programmer"))]
pub fn try_experimental_flash(_image: &[u8]) -> Result<FlashReceipt, FlashDenied> {
    Err(FlashDenied::ProgrammerUnimplemented)
}

#[cfg(all(test, feature = "hil_programmer"))]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn deny_without_allow_flash() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var(ENV_ALLOW_FLASH);
        std::env::remove_var(ENV_PROGRAMMER_CMD);
        assert_eq!(
            try_experimental_flash(&[1, 2, 3]),
            Err(FlashDenied::AllowFlashRequired)
        );
    }

    #[test]
    fn deny_without_cmd() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var(ENV_ALLOW_FLASH, "1");
        std::env::remove_var(ENV_PROGRAMMER_CMD);
        let r = try_experimental_flash(&[1]);
        std::env::remove_var(ENV_ALLOW_FLASH);
        assert_eq!(r, Err(FlashDenied::ProgrammerCmdMissing));
    }

    #[test]
    fn external_cmd_success() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var(ENV_ALLOW_FLASH, "1");
        std::env::set_var(ENV_PROGRAMMER_CMD, "test -f {image}");
        let receipt = try_experimental_flash(&[9, 9, 9, 9]).unwrap();
        std::env::remove_var(ENV_ALLOW_FLASH);
        std::env::remove_var(ENV_PROGRAMMER_CMD);
        assert_eq!(receipt.bytes, 4);
        assert_eq!(receipt.mode, "experimental_external_cmd");
        assert_ne!(receipt.mode, "production");
    }
}
