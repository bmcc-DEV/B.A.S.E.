//! Cliente QMP (QEMU Machine Protocol) — Specter Live E3.
//!
//! Unix socket + handshake `qmp_capabilities`. ≠ OS turnkey.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QmpError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("qmp: {0}")]
    Protocol(String),
    #[error("timeout waiting for qmp socket")]
    Timeout,
}

pub struct QmpClient {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl QmpClient {
    /// Liga a um socket Unix QMP (após `-qmp unix:PATH,server,nowait`).
    pub fn connect_unix(path: &Path) -> Result<Self, QmpError> {
        let stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let writer = stream.try_clone()?;
        let mut client = Self {
            reader: BufReader::new(stream),
            writer,
        };
        client.handshake()?;
        Ok(client)
    }

    /// Espera o socket aparecer e liga.
    pub fn connect_unix_wait(path: &Path, timeout: Duration) -> Result<Self, QmpError> {
        let start = Instant::now();
        loop {
            if path.exists() {
                match Self::connect_unix(path) {
                    Ok(c) => return Ok(c),
                    Err(QmpError::Io(_)) if start.elapsed() < timeout => {
                        std::thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }
            if start.elapsed() >= timeout {
                return Err(QmpError::Timeout);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn read_message(&mut self) -> Result<Value, QmpError> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            return Err(QmpError::Protocol("eof".into()));
        }
        Ok(serde_json::from_str(line.trim())?)
    }

    /// Lê até obter resposta (ignora eventos assíncronos).
    fn read_response(&mut self) -> Result<Value, QmpError> {
        loop {
            let v = self.read_message()?;
            if v.get("event").is_some() {
                continue;
            }
            return Ok(v);
        }
    }

    fn handshake(&mut self) -> Result<(), QmpError> {
        let greet = self.read_message()?;
        if greet.get("QMP").is_none() {
            return Err(QmpError::Protocol(format!(
                "expected QMP greeting, got {greet}"
            )));
        }
        let _ = self.execute("qmp_capabilities", None)?;
        Ok(())
    }

    pub fn execute(&mut self, cmd: &str, args: Option<Value>) -> Result<Value, QmpError> {
        let mut req = json!({ "execute": cmd });
        if let Some(a) = args {
            req["arguments"] = a;
        }
        writeln!(self.writer, "{req}")?;
        self.writer.flush()?;
        let resp = self.read_response()?;
        if let Some(err) = resp.get("error") {
            return Err(QmpError::Protocol(err.to_string()));
        }
        Ok(resp)
    }

    pub fn stop(&mut self) -> Result<Value, QmpError> {
        self.execute("stop", None)
    }

    pub fn cont(&mut self) -> Result<Value, QmpError> {
        self.execute("cont", None)
    }

    pub fn query_status(&mut self) -> Result<Value, QmpError> {
        self.execute("query-status", None)
    }

    pub fn inject_nmi(&mut self) -> Result<Value, QmpError> {
        self.execute("inject-nmi", None)
    }

    pub fn system_reset(&mut self) -> Result<Value, QmpError> {
        self.execute("system_reset", None)
    }

    pub fn quit(&mut self) -> Result<Value, QmpError> {
        self.execute("quit", None)
    }

    /// HMP passthrough (savevm/loadvm and friends).
    pub fn human_monitor(&mut self, command_line: &str) -> Result<Value, QmpError> {
        self.execute(
            "human-monitor-command",
            Some(json!({ "command-line": command_line })),
        )
    }

    /// Snapshot save via HMP `savevm TAG`.
    pub fn savevm(&mut self, tag: &str) -> Result<Value, QmpError> {
        self.human_monitor(&format!("savevm {tag}"))
    }

    /// Snapshot load via HMP `loadvm TAG`.
    pub fn loadvm(&mut self, tag: &str) -> Result<Value, QmpError> {
        self.human_monitor(&format!("loadvm {tag}"))
    }

    /// Optional: QEMU `inject-nmi` already exists; soft IRQ inject via HMP when supported.
    pub fn inject_irq_hmp(&mut self, irq: u32) -> Result<Value, QmpError> {
        // Best-effort; many machines ignore this — report QMP error to caller.
        self.human_monitor(&format!("irq {irq}"))
    }

    pub fn close(self) {
        let _ = self.writer.shutdown(Shutdown::Both);
    }
}

/// Probe mínimo: status → stop → status → cont (documenta controlo live).
pub fn probe_session(path: &Path) -> Result<Value, QmpError> {
    let mut c = QmpClient::connect_unix_wait(path, Duration::from_secs(10))?;
    let st0 = c.query_status()?;
    let _ = c.stop()?;
    let st1 = c.query_status()?;
    let _ = c.cont()?;
    let st2 = c.query_status()?;
    Ok(json!({
        "ok": true,
        "status_before": st0.get("return").cloned().unwrap_or(Value::Null),
        "status_stopped": st1.get("return").cloned().unwrap_or(Value::Null),
        "status_after_cont": st2.get("return").cloned().unwrap_or(Value::Null),
        "generates_os": false,
        "auto_fix_complete": false,
        "honesty": base_core::HONESTY_NOTE,
    }))
}

/// Probe F2: stop → savevm → loadvm → cont (requer guest com snapshot support).
pub fn probe_savevm(path: &Path, tag: &str) -> Result<Value, QmpError> {
    let mut c = QmpClient::connect_unix_wait(path, Duration::from_secs(10))?;
    let _ = c.stop()?;
    let save = c.savevm(tag);
    let load = if save.is_ok() {
        c.loadvm(tag)
    } else {
        Err(QmpError::Protocol("savevm failed".into()))
    };
    let _ = c.cont();
    Ok(json!({
        "ok": save.is_ok() && load.is_ok(),
        "tag": tag,
        "savevm": save.as_ref().map(|v| v.clone()).unwrap_or_else(|e| json!({"error": e.to_string()})),
        "loadvm": load.as_ref().map(|v| v.clone()).unwrap_or_else(|e| json!({"error": e.to_string()})),
        "generates_os": false,
        "auto_fix_complete": false,
        "honesty": base_core::HONESTY_NOTE,
        "note": "savevm/loadvm via human-monitor-command — machine-dependent",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::sync::mpsc;
    use tempfile::tempdir;

    #[test]
    fn handshake_and_stop_against_fake_qmp() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("qmp.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .write_all(
                    br#"{"QMP":{"version":{"qemu":{"major":10,"minor":0,"micro":0}}}}
"#,
                )
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.contains("qmp_capabilities"));
            stream.write_all(br#"{"return":{}}
"#).unwrap();
            line.clear();
            reader.read_line(&mut line).unwrap();
            assert!(line.contains("stop"));
            stream.write_all(br#"{"return":{}}
"#).unwrap();
            tx.send(()).unwrap();
        });

        let mut c = QmpClient::connect_unix(&sock).unwrap();
        c.stop().unwrap();
        rx.recv_timeout(Duration::from_secs(2)).unwrap();
    }
}
