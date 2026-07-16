/// Host Agent — template EXPERIMENTAL. Sem probe físico na CI.
use std::path::Path;

use crate::flash::{FlashDenied, FlashReceipt};
use crate::probe::ProbeFirmware;
use crate::programmer;
use crate::usb;

/// Env: força [`ProbePresence::Detected`] sem USB (só testes/offline).
pub const ENV_MOCK_DETECTED: &str = "BASE_HIL_MOCK_DETECTED";

/// VID canônico do stub RP2350 (`probe.rs`).
pub const DEFAULT_PROBE_VID: u16 = 0xCAFE;
/// PID canônico do stub RP2350 (`probe.rs`).
pub const DEFAULT_PROBE_PID: u16 = 0x4007;

/// Presença de hardware. Flash real só com [`ProbePresence::Detected`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbePresence {
    /// Sem USB/CMSIS-DAP — default do CI e de `connect`.
    Simulated,
    /// Probe reconhecido (mock via env/`with_presence`, ou USB com feature `hil_usb`).
    Detected,
}

/// Representa uma amostra capturada pelo probe
#[derive(Debug, Clone)]
pub struct HilSample {
    pub timestamp_ns: u64,
    pub address: u16,
    pub data: u8,
    pub flags: u8,
}

/// Agente host que se comunica com o probe HIL
pub struct HilAgent {
    presence: ProbePresence,
    /// Se true e Detected: `try_flash` devolve dry-run (sem silício).
    mock_flash: bool,
}

impl HilAgent {
    /// Enumeração: mock env → USB (`hil_usb`) → Simulated.
    ///
    /// Sem feature `hil_usb`, USB nunca é consultado (CI default).
    pub fn enumerate_presence(vid: u16, pid: u16) -> ProbePresence {
        if std::env::var_os(ENV_MOCK_DETECTED).is_some() {
            tracing::warn!(
                "[HIL][EXPERIMENTAL] {ENV_MOCK_DETECTED} set — treating {:04x}:{:04x} as Detected (no USB)",
                vid,
                pid
            );
            return ProbePresence::Detected;
        }
        if usb::usb_device_present(vid, pid) {
            return ProbePresence::Detected;
        }
        ProbePresence::Simulated
    }

    /// Abre canal com o probe. Default CI ⇒ [`ProbePresence::Simulated`].
    pub fn connect(vid: u16, pid: u16) -> Result<Self, String> {
        let presence = Self::enumerate_presence(vid, pid);
        tracing::info!(
            "[HIL][EXPERIMENTAL] Connecting to probe {:04x}:{:04x} → {:?}",
            vid,
            pid,
            presence
        );
        Ok(Self {
            presence,
            mock_flash: false,
        })
    }

    /// Construtor de teste / futuro path com probe real.
    pub fn with_presence(presence: ProbePresence) -> Self {
        Self {
            presence,
            mock_flash: false,
        }
    }

    /// Detected + dry-run de flash (ainda EXPERIMENTAL — zero silício).
    pub fn with_mock_flash(presence: ProbePresence) -> Self {
        Self {
            presence,
            mock_flash: true,
        }
    }

    pub fn presence(&self) -> ProbePresence {
        self.presence
    }

    pub fn can_flash(&self) -> bool {
        matches!(self.presence, ProbePresence::Detected)
    }

    /// Tentativa tipada de flash (T4/U3).
    ///
    /// Ordem: NotDetected → mock_dry_run → programador EXPERIMENTAL (`hil_programmer`) → Denied.
    /// **Nunca** devolve `mode == "production"`.
    pub fn try_flash(&self, image: &[u8]) -> Result<FlashReceipt, FlashDenied> {
        if !self.can_flash() {
            return Err(FlashDenied::NotDetected);
        }
        if self.mock_flash {
            tracing::warn!(
                "[HIL][EXPERIMENTAL] mock dry-run flash {} bytes — NO silicon written",
                image.len()
            );
            return Ok(FlashReceipt {
                bytes: image.len(),
                mode: "mock_dry_run",
            });
        }
        programmer::try_experimental_flash(image)
    }

    /// Compat: mapeia [`Self::try_flash`] para `Result<(), String>`.
    pub fn flash_probe_firmware(&self, image: &[u8]) -> Result<(), String> {
        match self.try_flash(image) {
            Ok(receipt) => {
                tracing::info!(
                    "[HIL] flash receipt mode={} bytes={}",
                    receipt.mode,
                    receipt.bytes
                );
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// Lê amostras do probe (modo simulado)
    pub fn read_samples(&self, count: usize) -> Vec<HilSample> {
        (0..count)
            .map(|i| HilSample {
                timestamp_ns: i as u64 * 1000,
                address: 0x1000 + i as u16,
                data: (i & 0xFF) as u8,
                flags: 0,
            })
            .collect()
    }

    /// Converte amostras para DeviceTrace (formato do base-check)
    pub fn samples_to_trace(samples: &[HilSample]) -> base_check::tracer::DeviceTrace {
        let events: Vec<_> = samples
            .iter()
            .map(|s| base_check::tracer::TraceEvent {
                timestamp_ns: s.timestamp_ns,
                channel: format!("BUS_{:04x}", s.address),
                event_type: base_check::tracer::EventType::MmioWrite,
                address: s.address as u64,
                value: Some(s.data as u64),
            })
            .collect();

        base_check::tracer::DeviceTrace {
            source: "HIL Probe [EXPERIMENTAL]".into(),
            device_name: "HIL Capture".into(),
            events,
        }
    }

    /// Exporta amostras como CSV no formato Saleae
    pub fn export_csv(samples: &[HilSample], path: &Path) -> Result<(), std::io::Error> {
        let mut csv = String::from("Time[s],Channel,Type,Data\n");
        for s in samples {
            csv.push_str(&format!(
                "{:.9},BUS_{:04x},WRITE,0x{:04x}=0x{:02x}\n",
                s.timestamp_ns as f64 / 1_000_000_000.0,
                s.address,
                s.address,
                s.data
            ));
        }
        std::fs::write(path, csv)
    }

    /// Script de scaffold do projeto embutido (não chama CLI — `base hil` ainda não existe).
    pub fn generate_build_script() -> String {
        let mut script = String::new();
        script.push_str("#!/bin/bash\n");
        script.push_str("# B.A.S.E. HIL Probe — EXPERIMENTAL scaffold\n");
        script.push_str("# Não faz flash. Não faz parte do `base pipeline` default.\n\n");
        script.push_str("set -euo pipefail\n\n");
        script.push_str("PROBE_DIR=\"hil_probe\"\n");
        script.push_str("mkdir -p \"$PROBE_DIR/src\"\n\n");
        script.push_str("echo \"Escreva o stub com a lib host:\"\n");
        script.push_str("echo \"  use base_hil::probe::ProbeFirmware;\"\n");
        script.push_str("echo \"  std::fs::write(\\\"$PROBE_DIR/src/main.rs\\\", ProbeFirmware::generate());\"\n\n");
        script.push_str("cat > \"$PROBE_DIR/Cargo.toml\" << 'EOF'\n");
        script.push_str("[package]\n");
        script.push_str("name = \"hil-probe\"\n");
        script.push_str("version = \"0.1.0\"\n");
        script.push_str("edition = \"2021\"\n\n");
        script.push_str("# Dependências de target embutido — fora do CI default do workspace.\n");
        script.push_str("[dependencies]\n");
        script.push_str("rp235x-hal = { git = \"https://github.com/rp-rs/rp-hal\" }\n");
        script.push_str("usb-device = \"0.3\"\n");
        script.push_str("usbd-serial = \"0.2\"\n");
        script.push_str("panic-halt = \"0.2\"\n");
        script.push_str("cortex-m-rt = \"0.7\"\n");
        script.push_str("cortex-m = \"0.7\"\n");
        script.push_str("EOF\n\n");
        script.push_str("echo \"[EXPERIMENTAL] scaffold em $PROBE_DIR/\"\n");
        script.push_str("echo \"Build (manual, precisa target): cargo build --release --target thumbv8m.main-none-eabi\"\n");
        script.push_str("echo \"Flash: HilAgent::try_flash / with_mock_flash / hil_programmer — EXPERIMENTAL\"\n");
        script
    }

    /// Escreve o stub de firmware gerado (host) para um path.
    pub fn write_probe_stub(path: &Path) -> Result<(), std::io::Error> {
        std::fs::write(path, ProbeFirmware::generate())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_agent_connect_is_simulated() {
        let agent = HilAgent::connect(DEFAULT_PROBE_VID, DEFAULT_PROBE_PID).unwrap();
        assert_eq!(agent.presence(), ProbePresence::Simulated);
        assert!(!agent.can_flash());
    }

    #[test]
    fn test_enumerate_default_simulated() {
        assert_eq!(
            HilAgent::with_presence(ProbePresence::Simulated).presence(),
            ProbePresence::Simulated
        );
    }

    #[test]
    fn test_enumerate_without_hil_usb_feature_is_simulated() {
        #[cfg(not(feature = "hil_usb"))]
        {
            if std::env::var_os(ENV_MOCK_DETECTED).is_none() {
                assert_eq!(
                    HilAgent::enumerate_presence(DEFAULT_PROBE_VID, DEFAULT_PROBE_PID),
                    ProbePresence::Simulated
                );
            }
        }
    }

    #[test]
    fn test_flash_denied_without_probe() {
        let agent = HilAgent::connect(DEFAULT_PROBE_VID, DEFAULT_PROBE_PID).unwrap();
        assert_eq!(agent.try_flash(&[0u8; 4]), Err(FlashDenied::NotDetected));
        let err = agent.flash_probe_firmware(&[0u8; 4]).unwrap_err();
        assert!(err.contains("EXPERIMENTAL"));
        assert!(err.contains("Detected"));
    }

    #[test]
    fn test_detected_without_mock_flash_gated() {
        let agent = HilAgent::with_presence(ProbePresence::Detected);
        assert!(agent.can_flash());
        let r = agent.try_flash(&[0u8; 4]);
        #[cfg(not(feature = "hil_programmer"))]
        assert_eq!(r, Err(FlashDenied::ProgrammerUnimplemented));
        #[cfg(feature = "hil_programmer")]
        assert_eq!(r, Err(FlashDenied::AllowFlashRequired));
    }

    #[test]
    fn test_detected_mock_flash_dry_run() {
        let agent = HilAgent::with_mock_flash(ProbePresence::Detected);
        let receipt = agent.try_flash(&[1, 2, 3, 4]).unwrap();
        assert_eq!(receipt.bytes, 4);
        assert_eq!(receipt.mode, "mock_dry_run");
        assert_ne!(receipt.mode, "production");
        assert!(agent.flash_probe_firmware(&[0u8; 8]).is_ok());
    }

    #[test]
    fn test_mock_flash_still_denied_if_simulated() {
        let agent = HilAgent::with_mock_flash(ProbePresence::Simulated);
        assert_eq!(agent.try_flash(&[0u8; 1]), Err(FlashDenied::NotDetected));
    }

    #[test]
    fn test_read_samples() {
        let agent = HilAgent::connect(DEFAULT_PROBE_VID, DEFAULT_PROBE_PID).unwrap();
        let samples = agent.read_samples(10);
        assert_eq!(samples.len(), 10);
        assert_eq!(samples[0].address, 0x1000);
    }

    #[test]
    fn test_samples_to_trace() {
        let agent = HilAgent::connect(DEFAULT_PROBE_VID, DEFAULT_PROBE_PID).unwrap();
        let samples = agent.read_samples(5);
        let trace = HilAgent::samples_to_trace(&samples);
        assert_eq!(trace.events.len(), 5);
        assert!(trace.source.contains("HIL"));
        assert!(trace.source.contains("EXPERIMENTAL"));
    }

    #[test]
    fn test_export_csv() {
        let agent = HilAgent::connect(DEFAULT_PROBE_VID, DEFAULT_PROBE_PID).unwrap();
        let samples = agent.read_samples(3);
        let dir = tempdir().unwrap();
        let path = dir.path().join("capture.csv");
        HilAgent::export_csv(&samples, &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("BUS_1000"));
    }

    #[test]
    fn test_build_script_no_fake_cli() {
        let script = HilAgent::generate_build_script();
        assert!(script.contains("hil-probe"));
        assert!(script.contains("EXPERIMENTAL"));
        assert!(!script.contains("base hil "));
        assert!(script.contains("thumbv8m"));
    }

    #[test]
    fn test_write_probe_stub() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("main.rs");
        HilAgent::write_probe_stub(&path).unwrap();
        let fw = std::fs::read_to_string(&path).unwrap();
        assert!(fw.contains("HIL Probe"));
    }

    #[cfg(feature = "hil_usb")]
    #[test]
    #[ignore = "requires USB probe 0xCAFE:0x4007"]
    fn test_usb_enumerate_hardware() {
        assert_eq!(
            HilAgent::enumerate_presence(DEFAULT_PROBE_VID, DEFAULT_PROBE_PID),
            ProbePresence::Detected
        );
    }

    #[cfg(feature = "hil_programmer")]
    #[test]
    fn test_programmer_external_via_agent() {
        use crate::programmer::{ENV_ALLOW_FLASH, ENV_PROGRAMMER_CMD};
        use std::sync::Mutex;
        static LOCK: Mutex<()> = Mutex::new(());
        let _g = LOCK.lock().unwrap();
        std::env::set_var(ENV_ALLOW_FLASH, "1");
        std::env::set_var(ENV_PROGRAMMER_CMD, "test -f {image}");
        let agent = HilAgent::with_presence(ProbePresence::Detected);
        let receipt = agent.try_flash(&[1, 2, 3]).unwrap();
        std::env::remove_var(ENV_ALLOW_FLASH);
        std::env::remove_var(ENV_PROGRAMMER_CMD);
        assert_eq!(receipt.mode, "experimental_external_cmd");
        assert_ne!(receipt.mode, "production");
    }
}
