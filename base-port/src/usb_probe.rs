//! Probe USB de telefone (ADB / fastboot / lsusb) — inventário vivo de HW.
//!
//! Read-only. ≠ flash · ≠ OS turnkey · `generates_os: false`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::{Command, Stdio};
use std::time::Duration;

const SENSITIVE_PROP_KEYS: &[&str] = &[
    "ro.serialno",
    "ro.boot.serialno",
    "ril.imei",
    "persist.radio.imei",
    "gsm.serial",
    "ro.ril.oem.imei",
    "ro.ril.oem.imeisv",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsbProbeMode {
    Adb,
    Fastboot,
    HostOnly,
    None,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsbProbeOptions {
    /// ADB serial (`-s`); empty = first `device`
    pub serial: Option<String>,
    pub skip_adb: bool,
    pub skip_fastboot: bool,
    pub skip_lsusb: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbHwInventory {
    pub ok: bool,
    pub skipped: bool,
    pub skip_reason: Option<String>,
    pub mode: UsbProbeMode,
    pub adb_serial: Option<String>,
    pub host_lsusb: Vec<String>,
    pub props: BTreeMap<String, String>,
    pub sysfs_classes: BTreeMap<String, Vec<String>>,
    pub dt_model: Option<String>,
    pub dt_compatibles: Vec<String>,
    /// `/sys/bus/platform/devices` (legível sem root) — nomes tipo `20200000.serial`
    #[serde(default)]
    pub platform_devices: Vec<String>,
    pub dumpsys_snippets: BTreeMap<String, String>,
    pub fastboot_vars: BTreeMap<String, String>,
    pub generates_os: bool,
    pub auto_fix_complete: bool,
    pub honesty: String,
    pub note: String,
}

impl Default for UsbHwInventory {
    fn default() -> Self {
        Self {
            ok: false,
            skipped: true,
            skip_reason: Some("not run".into()),
            mode: UsbProbeMode::None,
            adb_serial: None,
            host_lsusb: Vec::new(),
            props: BTreeMap::new(),
            sysfs_classes: BTreeMap::new(),
            dt_model: None,
            dt_compatibles: Vec::new(),
            platform_devices: Vec::new(),
            dumpsys_snippets: BTreeMap::new(),
            fastboot_vars: BTreeMap::new(),
            generates_os: false,
            auto_fix_complete: false,
            honesty: base_core::HONESTY_NOTE.to_string(),
            note: "USB phone probe — read-only; ≠ OS turnkey".into(),
        }
    }
}

impl UsbHwInventory {
    pub fn to_yaml(&self) -> anyhow::Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    pub fn to_json_pretty(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Markdown sem IMEI/serial em claro.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# USB HW probe\n\n");
        md.push_str(&format!("{}\n\n", base_core::HONESTY_BANNER));
        md.push_str(&format!(
            "- ok: {} · skipped: {} · mode: {:?}\n",
            self.ok, self.skipped, self.mode
        ));
        if let Some(r) = &self.skip_reason {
            md.push_str(&format!("- skip_reason: {r}\n"));
        }
        if let Some(s) = &self.adb_serial {
            md.push_str(&format!("- adb_serial: {}\n", redact_value(s)));
        }
        md.push_str(&format!("- host_lsusb entries: {}\n", self.host_lsusb.len()));
        for line in self.host_lsusb.iter().take(12) {
            md.push_str(&format!("  - `{line}`\n"));
        }
        md.push_str("\n## Props (redacted)\n\n");
        for (k, v) in &self.props {
            let show = if is_sensitive_key(k) {
                redact_value(v)
            } else {
                v.clone()
            };
            md.push_str(&format!("- `{k}` = `{show}`\n"));
        }
        if let Some(m) = &self.dt_model {
            md.push_str(&format!("\n## DT model\n\n`{m}`\n"));
        }
        if !self.dt_compatibles.is_empty() {
            md.push_str(&format!(
                "\n## DT compatibles ({} first)\n\n",
                self.dt_compatibles.len().min(40)
            ));
            for c in self.dt_compatibles.iter().take(40) {
                md.push_str(&format!("- `{c}`\n"));
            }
        } else {
            md.push_str(
                "\n## DT\n\n_(vazio — tipicamente Permission denied sem root; usar platform_devices)_\n",
            );
        }
        if !self.platform_devices.is_empty() {
            md.push_str(&format!(
                "\n## platform devices ({} first)\n\n",
                self.platform_devices.len().min(60)
            ));
            for d in self.platform_devices.iter().take(60) {
                md.push_str(&format!("- `{d}`\n"));
            }
        }
        if !self.sysfs_classes.is_empty() {
            md.push_str("\n## sysfs classes\n\n");
            for (cls, entries) in &self.sysfs_classes {
                md.push_str(&format!(
                    "- `{cls}`: {} ({})\n",
                    entries.len(),
                    entries.iter().take(8).cloned().collect::<Vec<_>>().join(", ")
                ));
            }
        }
        if !self.fastboot_vars.is_empty() {
            md.push_str("\n## fastboot vars\n\n");
            for (k, v) in &self.fastboot_vars {
                md.push_str(&format!("- `{k}` = `{v}`\n"));
            }
        }
        if !self.dumpsys_snippets.is_empty() {
            md.push_str("\n## dumpsys (truncated)\n\n");
            for (name, body) in &self.dumpsys_snippets {
                md.push_str(&format!("### {name}\n\n```\n{body}\n```\n\n"));
            }
        }
        md.push_str("\nCruzar com `platform_vendor_boot/` / port package LK.\n");
        md.push_str("\n`generates_os: false` · `auto_fix_complete: false`\n");
        md
    }
}

fn is_sensitive_key(k: &str) -> bool {
    let lower = k.to_ascii_lowercase();
    SENSITIVE_PROP_KEYS
        .iter()
        .any(|s| lower == *s || lower.contains("imei") || lower.contains("serialno"))
}

fn redact_value(v: &str) -> String {
    if v.is_empty() {
        return String::new();
    }
    let hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        v.hash(&mut h);
        h.finish()
    };
    format!("redacted:{:08x}", hash as u32)
}

fn which(bin: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {bin} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_capture(bin: &str, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = cmd.output().ok()?;
    if !out.status.success() && out.stdout.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn run_capture_timeout(bin: &str, args: &[&str], _timeout: Duration) -> Option<String> {
    // Simple capture; adb timeouts handled by caller keeping commands short.
    run_capture(bin, args)
}

fn parse_lsusb(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// First `adb devices` serial in state `device`.
fn adb_first_device(serial_pref: Option<&str>) -> Option<String> {
    if !which("adb") {
        return None;
    }
    let raw = run_capture("adb", &["devices"])?;
    let mut devices = Vec::new();
    for line in raw.lines().skip(1) {
        let mut parts = line.split_whitespace();
        let Some(ser) = parts.next() else { continue };
        let Some(state) = parts.next() else { continue };
        if state == "device" {
            devices.push(ser.to_string());
        }
    }
    if let Some(pref) = serial_pref {
        if devices.iter().any(|d| d == pref) {
            return Some(pref.to_string());
        }
        return None;
    }
    devices.into_iter().next()
}

fn adb_shell(serial: &str, shell_cmd: &str) -> Option<String> {
    run_capture_timeout(
        "adb",
        &["-s", serial, "shell", shell_cmd],
        Duration::from_secs(15),
    )
}

fn collect_props(serial: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let prefixes = [
        "ro.product.",
        "ro.board.",
        "ro.hardware",
        "ro.boot.hardware",
        "ro.build.product",
        "ro.build.version.release",
        "ro.soc.",
        "gsm.",
        "ril.",
        "persist.vendor.",
        "vendor.",
    ];
    let raw = adb_shell(serial, "getprop").unwrap_or_default();
    for line in raw.lines() {
        // [key]: [value]
        let line = line.trim();
        if !line.starts_with('[') {
            continue;
        }
        let Some(rest) = line.strip_prefix('[') else {
            continue;
        };
        let Some((key, after)) = rest.split_once("]: [") else {
            continue;
        };
        let value = after.strip_suffix(']').unwrap_or(after);
        if prefixes.iter().any(|p| key.starts_with(p) || key == "ro.hardware")
            || key.contains("imei")
            || key.contains("serialno")
        {
            map.insert(key.to_string(), value.to_string());
        }
    }
    // Always try a few explicit props
    for key in [
        "ro.product.model",
        "ro.product.device",
        "ro.product.board",
        "ro.board.platform",
        "ro.hardware",
        "ro.boot.hardware",
        "ro.soc.model",
        "ro.soc.manufacturer",
    ] {
        if map.contains_key(key) {
            continue;
        }
        if let Some(v) = adb_shell(serial, &format!("getprop {key}")) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                map.insert(key.to_string(), v);
            }
        }
    }
    map
}

fn collect_sysfs(serial: &str) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    let classes = [
        "tty",
        "mmc_host",
        "graphics",
        "input",
        "net",
        "power_supply",
        "leds",
        "drm",
        "spi_master",
        "i2c-dev",
    ];
    for cls in classes {
        let cmd = format!("ls /sys/class/{cls} 2>/dev/null | head -n 32");
        if let Some(raw) = adb_shell(serial, &cmd) {
            let entries: Vec<String> = raw
                .split_whitespace()
                .map(str::to_string)
                .filter(|s| !s.is_empty())
                .collect();
            if !entries.is_empty() {
                out.insert(cls.to_string(), entries);
            }
        }
    }
    out
}

fn collect_platform_devices(serial: &str) -> Vec<String> {
    let raw = adb_shell(
        serial,
        "ls /sys/bus/platform/devices 2>/dev/null | head -n 200",
    )
    .unwrap_or_default();
    raw.split_whitespace()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .collect()
}

fn collect_dt(serial: &str) -> (Option<String>, Vec<String>) {
    // Android: /proc/device-tree → /sys/firmware/devicetree/base (often root-only)
    let roots = [
        "/sys/firmware/devicetree/base",
        "/proc/device-tree",
    ];
    let mut model = None;
    for root in roots {
        if let Some(raw) = adb_shell(serial, &format!("cat {root}/model 2>/dev/null")) {
            let t = raw.trim().trim_end_matches('\0').to_string();
            if !t.is_empty() && !t.contains("Permission denied") {
                model = Some(t);
                break;
            }
        }
    }
    let mut compatibles = Vec::new();
    for root in roots {
        let cmd = format!(
            "for f in $(find {root} -name compatible 2>/dev/null | head -n 80); do tr '\\0' ' ' < \"$f\" 2>/dev/null; echo; done"
        );
        if let Some(raw) = adb_shell(serial, &cmd) {
            for line in raw.lines() {
                let t = line.trim().trim_end_matches('\0');
                if t.is_empty() || t.contains("Permission denied") {
                    continue;
                }
                for part in t.split_whitespace() {
                    if !part.is_empty() && !compatibles.iter().any(|c: &String| c == part) {
                        compatibles.push(part.to_string());
                    }
                }
                if compatibles.len() >= 80 {
                    break;
                }
            }
        }
        if !compatibles.is_empty() {
            break;
        }
    }
    (model, compatibles)
}

fn collect_dumpsys(serial: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let services = ["Battery", "sensorservice", "SurfaceFlinger"];
    for svc in services {
        if let Some(raw) = adb_shell(serial, &format!("dumpsys {svc} 2>/dev/null | head -n 40")) {
            let body = raw.trim().to_string();
            if !body.is_empty() {
                out.insert(svc.to_string(), truncate(&body, 2500));
            }
        }
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    format!("{}…", &s[..max])
}

fn fastboot_first_device() -> Option<String> {
    if !which("fastboot") {
        return None;
    }
    let raw = run_capture("fastboot", &["devices"])?;
    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        let Some(ser) = parts.next() else { continue };
        if parts.next().is_some() {
            return Some(ser.to_string());
        }
    }
    None
}

fn collect_fastboot_vars(serial: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let raw =
        run_capture("fastboot", &["-s", serial, "getvar", "all"]).unwrap_or_default();
    // stderr often holds getvar all — also try combining
    let err = {
        let mut cmd = Command::new("fastboot");
        cmd.args(["-s", serial, "getvar", "all"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.output()
            .ok()
            .map(|o| {
                let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
                s.push_str(&String::from_utf8_lossy(&o.stderr));
                s
            })
            .unwrap_or(raw)
    };
    for line in err.lines() {
        // (bootloader) key: value
        let line = line.trim();
        let line = line.strip_prefix("(bootloader) ").unwrap_or(line);
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim();
            let v = v.trim();
            if !k.is_empty() && k != "all" {
                map.insert(k.to_string(), v.to_string());
            }
        }
    }
    map
}

/// Corre o probe USB (ADB → fastboot → host-only).
pub fn run_usb_hw_probe(opts: &UsbProbeOptions) -> UsbHwInventory {
    let mut inv = UsbHwInventory::default();
    inv.skipped = false;
    inv.skip_reason = None;
    inv.generates_os = false;
    inv.auto_fix_complete = false;

    if !opts.skip_lsusb {
        if which("lsusb") {
            if let Some(raw) = run_capture("lsusb", &[]) {
                inv.host_lsusb = parse_lsusb(&raw);
            }
        } else {
            tracing::debug!("lsusb not installed");
        }
    }

    if !opts.skip_adb {
        if let Some(serial) = adb_first_device(opts.serial.as_deref()) {
            tracing::info!("[PORT] USB probe via ADB serial={}", redact_value(&serial));
            inv.mode = UsbProbeMode::Adb;
            inv.adb_serial = Some(serial.clone());
            inv.props = collect_props(&serial);
            inv.sysfs_classes = collect_sysfs(&serial);
            let (model, comps) = collect_dt(&serial);
            inv.dt_model = model;
            inv.dt_compatibles = comps;
            inv.platform_devices = collect_platform_devices(&serial);
            inv.dumpsys_snippets = collect_dumpsys(&serial);
            inv.ok = !inv.props.is_empty()
                || !inv.sysfs_classes.is_empty()
                || inv.dt_model.is_some()
                || !inv.dt_compatibles.is_empty()
                || !inv.platform_devices.is_empty();
            if inv.dt_compatibles.is_empty() && !inv.platform_devices.is_empty() {
                inv.note = "ADB OK; DT /proc inaccessible without root — platform_devices filled from sysfs"
                    .into();
            } else if !inv.ok {
                inv.ok = true;
                inv.note =
                    "ADB connected but sparse props/sysfs — check authorization / userdebug"
                        .into();
            }
            return inv;
        }
    }

    if !opts.skip_fastboot {
        if let Some(serial) = fastboot_first_device() {
            tracing::info!("[PORT] USB probe via fastboot");
            inv.mode = UsbProbeMode::Fastboot;
            inv.adb_serial = Some(serial.clone());
            inv.fastboot_vars = collect_fastboot_vars(&serial);
            inv.ok = !inv.fastboot_vars.is_empty() || !inv.host_lsusb.is_empty();
            if inv.fastboot_vars.is_empty() {
                inv.note = "fastboot device seen but getvar empty".into();
            }
            return inv;
        }
    }

    if !inv.host_lsusb.is_empty() {
        inv.mode = UsbProbeMode::HostOnly;
        inv.ok = false;
        inv.skipped = true;
        inv.skip_reason = Some(
            "lsusb only — no adb device / fastboot; plug phone with USB debugging or bootloader"
                .into(),
        );
        return inv;
    }

    inv.mode = UsbProbeMode::None;
    inv.ok = false;
    inv.skipped = true;
    inv.skip_reason = Some(
        "no adb/fastboot device and no lsusb output — CI/host without phone".into(),
    );
    inv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_without_phone_is_skipped_ok() {
        let inv = run_usb_hw_probe(&UsbProbeOptions {
            skip_adb: true,
            skip_fastboot: true,
            skip_lsusb: true,
            serial: None,
        });
        assert!(inv.skipped);
        assert!(!inv.ok);
        assert!(!inv.generates_os);
        assert_eq!(inv.mode, UsbProbeMode::None);
    }

    #[test]
    fn redact_hides_serial() {
        let r = redact_value("ABC123XYZ");
        assert!(r.starts_with("redacted:"));
        assert!(!r.contains("ABC123"));
    }

    #[test]
    fn markdown_redacts_imei_keys() {
        let mut inv = UsbHwInventory::default();
        inv.skipped = false;
        inv.ok = true;
        inv.mode = UsbProbeMode::Adb;
        inv.props
            .insert("ril.imei".into(), "123456789012345".into());
        inv.props
            .insert("ro.product.model".into(), "moto g35 5G".into());
        let md = inv.to_markdown();
        assert!(!md.contains("123456789012345"));
        assert!(md.contains("moto g35 5G"));
        assert!(md.contains("redacted:"));
    }
}
