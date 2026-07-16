//! Lançador QEMU — smoke + Specter Live (plugin TCG / QMP).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QemuLaunchResult {
    pub launched: bool,
    pub skipped: bool,
    pub skip_reason: Option<String>,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub bin: String,
    pub kernel: Option<String>,
    pub log_path: Option<String>,
    pub timeout_sec: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qmp_socket: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_outfile: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QemuLaunchOpts {
    pub bin: String,
    pub machine: String,
    pub cpu: String,
    pub memory: String,
    pub kernel: Option<PathBuf>,
    pub timeout_sec: u64,
    pub log_path: PathBuf,
    pub extra_args: Vec<String>,
    /// Path to `libbase_virt_ndjson.so` (TCG plugin).
    pub plugin: Option<PathBuf>,
    /// NDJSON outfile for the plugin (`outfile=` arg).
    pub plugin_outfile: Option<PathBuf>,
    /// Extra plugin args (e.g. `io_only=1`, `base=0x…`).
    pub plugin_args: Vec<String>,
    /// Unix QMP socket path (`-qmp unix:PATH,server,nowait`).
    pub qmp_socket: Option<PathBuf>,
}

impl Default for QemuLaunchOpts {
    fn default() -> Self {
        Self {
            bin: "qemu-system-aarch64".into(),
            machine: "virt".into(),
            cpu: "cortex-a72".into(),
            memory: "256M".into(),
            kernel: None,
            timeout_sec: 8,
            log_path: PathBuf::from("qemu.log"),
            extra_args: Vec::new(),
            plugin: None,
            plugin_outfile: None,
            plugin_args: vec!["io_only=1".into()],
            qmp_socket: None,
        }
    }
}

fn which_bin(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn build_plugin_arg(opts: &QemuLaunchOpts) -> Option<String> {
    let plugin = opts.plugin.as_ref()?;
    let mut parts = vec![plugin.display().to_string()];
    if let Some(out) = &opts.plugin_outfile {
        parts.push(format!("outfile={}", out.display()));
    }
    for a in &opts.plugin_args {
        parts.push(a.clone());
    }
    Some(parts.join(","))
}

fn prepare_command(opts: &QemuLaunchOpts, kernel: &Path) -> anyhow::Result<Command> {
    if let Some(parent) = opts.log_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    if let Some(qmp) = &opts.qmp_socket {
        if let Some(parent) = qmp.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let _ = std::fs::remove_file(qmp);
    }
    if let Some(out) = &opts.plugin_outfile {
        if let Some(parent) = out.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
    }

    let mut cmd = Command::new(&opts.bin);
    cmd.arg("-machine")
        .arg(&opts.machine)
        .arg("-cpu")
        .arg(&opts.cpu)
        .arg("-m")
        .arg(&opts.memory)
        .arg("-nographic")
        .arg("-kernel")
        .arg(kernel);

    if let Some(plugin_arg) = build_plugin_arg(opts) {
        cmd.arg("-plugin").arg(plugin_arg);
    }
    if let Some(qmp) = &opts.qmp_socket {
        cmd.arg("-qmp")
            .arg(format!("unix:{},server,nowait", qmp.display()));
    }
    cmd.args(&opts.extra_args);
    Ok(cmd)
}

/// Sessão viva (Child + caminhos) para QMP / plugin.
pub struct QemuLiveSession {
    pub child: Child,
    pub qmp_socket: Option<PathBuf>,
    pub plugin_outfile: Option<PathBuf>,
    pub log_path: PathBuf,
    pub bin: String,
    pub kernel: String,
}

/// Arranca QEMU sem timeout (caller faz QMP / wait / kill).
pub fn spawn_qemu_live(opts: &QemuLaunchOpts) -> anyhow::Result<Result<QemuLiveSession, QemuLaunchResult>> {
    if !which_bin(&opts.bin) {
        return Ok(Err(QemuLaunchResult {
            launched: false,
            skipped: true,
            skip_reason: Some(format!("{} not installed", opts.bin)),
            exit_code: None,
            timed_out: false,
            bin: opts.bin.clone(),
            kernel: opts.kernel.as_ref().map(|p| p.display().to_string()),
            log_path: None,
            timeout_sec: opts.timeout_sec,
            qmp_socket: opts.qmp_socket.as_ref().map(|p| p.display().to_string()),
            plugin: opts.plugin.as_ref().map(|p| p.display().to_string()),
            plugin_outfile: opts
                .plugin_outfile
                .as_ref()
                .map(|p| p.display().to_string()),
        }));
    }

    let kernel = opts
        .kernel
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("QEMU launch requires --kernel"))?;

    let log = std::fs::File::create(&opts.log_path)?;
    let log_err = log.try_clone()?;
    let mut cmd = prepare_command(opts, kernel)?;
    cmd.stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));

    let child = cmd.spawn()?;
    Ok(Ok(QemuLiveSession {
        child,
        qmp_socket: opts.qmp_socket.clone(),
        plugin_outfile: opts.plugin_outfile.clone(),
        log_path: opts.log_path.clone(),
        bin: opts.bin.clone(),
        kernel: kernel.display().to_string(),
    }))
}

/// Corre QEMU com timeout; não exige guest saudável (smoke / live seed).
pub fn launch_qemu(opts: &QemuLaunchOpts) -> anyhow::Result<QemuLaunchResult> {
    let session = match spawn_qemu_live(opts)? {
        Ok(s) => s,
        Err(skipped) => return Ok(skipped),
    };

    let mut child = session.child;
    let timeout = Duration::from_secs(opts.timeout_sec.max(1));
    let start = std::time::Instant::now();
    let mut timed_out = false;
    let exit_code = loop {
        match child.try_wait()? {
            Some(status) => break status.code(),
            None => {
                if start.elapsed() >= timeout {
                    // Prefer graceful quit via QMP when available.
                    if let Some(sock) = &session.qmp_socket {
                        if let Ok(mut q) =
                            crate::qmp::QmpClient::connect_unix_wait(sock, Duration::from_millis(200))
                        {
                            let _ = q.quit();
                            let _ = child.wait();
                            timed_out = true;
                            break Some(124);
                        }
                    }
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break Some(124);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };

    Ok(QemuLaunchResult {
        launched: true,
        skipped: false,
        skip_reason: None,
        exit_code,
        timed_out,
        bin: session.bin,
        kernel: Some(session.kernel),
        log_path: Some(session.log_path.display().to_string()),
        timeout_sec: opts.timeout_sec,
        qmp_socket: session.qmp_socket.map(|p| p.display().to_string()),
        plugin: opts.plugin.as_ref().map(|p| p.display().to_string()),
        plugin_outfile: session.plugin_outfile.map(|p| p.display().to_string()),
    })
}

/// Resolve binário QEMU a partir do path opcional.
pub fn resolve_qemu_bin(explicit: Option<&Path>, default: &str) -> String {
    explicit
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| default.to_string())
}

/// Monta string `-plugin` a partir de paths (útil para docs / scripts).
pub fn format_plugin_cli(plugin: &Path, outfile: &Path, extra: &[&str]) -> String {
    let mut parts = vec![
        plugin.display().to_string(),
        format!("outfile={}", outfile.display()),
    ];
    for e in extra {
        parts.push((*e).to_string());
    }
    parts.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_qemu_skips() {
        let opts = QemuLaunchOpts {
            bin: "qemu-system-base-virt-missing-xyz".into(),
            kernel: Some(PathBuf::from("/tmp/nope.bin")),
            ..Default::default()
        };
        let r = launch_qemu(&opts).unwrap();
        assert!(r.skipped);
        assert!(!r.launched);
    }

    #[test]
    fn plugin_arg_joins() {
        let opts = QemuLaunchOpts {
            plugin: Some(PathBuf::from("/tmp/lib.so")),
            plugin_outfile: Some(PathBuf::from("/tmp/t.ndjson")),
            plugin_args: vec!["io_only=1".into(), "base=0x1000".into()],
            ..Default::default()
        };
        let s = build_plugin_arg(&opts).unwrap();
        assert!(s.contains("lib.so"));
        assert!(s.contains("outfile=/tmp/t.ndjson"));
        assert!(s.contains("io_only=1"));
    }
}
