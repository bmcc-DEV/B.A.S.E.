//! Cruzamento USB live (`platform_devices`) ↔ inventário DTB (`PlatformInventory`).
//!
//! ≠ OS turnkey · `generates_os: false`.

use crate::platform::PlatformInventory;
use crate::usb_probe::UsbHwInventory;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CrossHitKind {
    /// Endereço absoluto bate (USB addr ∈ nó DTB ou bases)
    Address,
    /// Mesma classe OS-port (uart/storage/gpu/…) sem addr absoluto igual
    ClassOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossMatch {
    pub usb_device: String,
    pub usb_addr: Option<u64>,
    pub usb_class: String,
    pub dt_class: Option<String>,
    pub dt_nodes: Vec<String>,
    pub hit: CrossHitKind,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BringupStep {
    pub order: u32,
    pub class: String,
    pub priority: String,
    pub usb_devices: Vec<String>,
    pub dt_status: Option<String>,
    pub dt_nodes: Vec<String>,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDtCrossReport {
    pub ok: bool,
    pub usb_devices: usize,
    pub matches: Vec<CrossMatch>,
    pub usb_only: Vec<String>,
    pub dt_class_coverage: BTreeMap<String, String>,
    pub bringup: Vec<BringupStep>,
    pub port_target: String,
    pub generates_os: bool,
    pub auto_fix_complete: bool,
    pub honesty: String,
    pub note: String,
}

impl UsbDtCrossReport {
    pub fn to_yaml(&self) -> anyhow::Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    pub fn to_json_pretty(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# USB × DTB cross + bring-up\n\n");
        md.push_str(&format!("{}\n\n", base_core::HONESTY_BANNER));
        md.push_str(&format!(
            "- target: **{}**\n- usb_devices: {} · matches: {} · usb_only: {}\n",
            self.port_target,
            self.usb_devices,
            self.matches.len(),
            self.usb_only.len()
        ));
        md.push_str(&format!("- note: {}\n\n", self.note));

        md.push_str("## DT class coverage\n\n");
        md.push_str("| Class | Coverage |\n|-------|----------|\n");
        for (k, v) in &self.dt_class_coverage {
            md.push_str(&format!("| `{k}` | {v} |\n"));
        }

        md.push_str("\n## Bring-up order (wedge)\n\n");
        for s in &self.bringup {
            md.push_str(&format!(
                "### {}. {} ({})\n\n- DT: {:?}\n- USB: {}\n- Action: {}\n\n",
                s.order,
                s.class,
                s.priority,
                s.dt_status,
                if s.usb_devices.is_empty() {
                    "_(none)_".into()
                } else {
                    s.usb_devices
                        .iter()
                        .take(6)
                        .map(|d| format!("`{d}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                },
                s.action
            ));
        }

        md.push_str("## Address / class hits (sample)\n\n");
        for m in self.matches.iter().take(40) {
            md.push_str(&format!(
                "- `{}` → {:?} / dt={} — {}\n",
                m.usb_device,
                m.hit,
                m.dt_class.as_deref().unwrap_or("-"),
                m.note
            ));
        }
        if self.matches.len() > 40 {
            md.push_str(&format!("- … +{} more\n", self.matches.len() - 40));
        }

        if !self.usb_only.is_empty() {
            md.push_str(&format!(
                "\n## USB-only (no DT class/addr hit, first 30 of {})\n\n",
                self.usb_only.len()
            ));
            for d in self.usb_only.iter().take(30) {
                md.push_str(&format!("- `{d}`\n"));
            }
        }

        md.push_str("\n`generates_os: false` · ≠ TaurOS turnkey\n");
        md
    }
}

fn classify_usb_name(name: &str) -> String {
    let n = name.to_ascii_lowercase();
    if n.contains("serial") || n.contains("uart") {
        "uart".into()
    } else if n.contains("ufs") || n.contains("sdio") || n.contains("mmc") {
        "storage_emmc_ufs".into()
    } else if n.contains("gpu") || n.contains("dpu") || n.contains("display") {
        "gpu_framebuffer".into()
    } else if n.contains("gic") || n.contains("interrupt") {
        "gic".into()
    } else if n.contains("timer") {
        "arm_generic_timer".into()
    } else if n.contains("usb") || n.contains("dwc") {
        "usb".into()
    } else if n.contains("gpio") || n.contains("pinctrl") || n.contains("pinmux") {
        "gpio".into()
    } else if n.contains("i2c") {
        "i2c".into()
    } else if n.contains("spi") {
        "spi".into()
    } else if n.contains("clock") || n.contains("gate") {
        "clock".into()
    } else if n.contains("syscon") {
        "syscon".into()
    } else if n.contains("dma") {
        "dma".into()
    } else if n.contains("iommu") || n.contains("smmu") {
        "mmu".into()
    } else {
        "other".into()
    }
}

fn parse_usb_device(dev: &str) -> (Option<u64>, &str) {
    if let Some((hex, rest)) = dev.split_once('.') {
        if let Ok(addr) = u64::from_str_radix(hex, 16) {
            return (Some(addr), rest);
        }
    }
    (None, dev)
}

fn extract_addrs_from_node(node: &str) -> Vec<u64> {
    let mut out = Vec::new();
    // …@hex or …@0xhex
    if let Some(at) = node.rfind('@') {
        let tail = &node[at + 1..];
        let hex = tail.split(|c: char| !c.is_ascii_hexdigit()).next().unwrap_or("");
        if !hex.is_empty() {
            if let Ok(v) = u64::from_str_radix(hex, 16) {
                out.push(v);
            }
        }
    }
    out
}

fn map_usb_class_to_dt(usb_class: &str) -> Option<&'static str> {
    match usb_class {
        "uart" => Some("uart"),
        "storage_emmc_ufs" => Some("storage_emmc_ufs"),
        "gpu_framebuffer" => Some("gpu_framebuffer"),
        "gic" => Some("gic"),
        "arm_generic_timer" => Some("arm_generic_timer"),
        "gpio" => Some("gpio"),
        "mmu" => Some("mmu"),
        _ => None,
    }
}

/// Cruza inventário USB com platform inventory (DTB).
pub fn cross_usb_dt(usb: &UsbHwInventory, plat: &PlatformInventory) -> UsbDtCrossReport {
    let mut dt_addrs: BTreeMap<u64, Vec<(String, String)>> = BTreeMap::new(); // addr -> (class, node)
    let mut dt_by_class: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for c in &plat.components {
        for n in &c.nodes {
            dt_by_class
                .entry(c.class.clone())
                .or_default()
                .push(n.clone());
            for a in extract_addrs_from_node(n) {
                dt_addrs
                    .entry(a)
                    .or_default()
                    .push((c.class.clone(), n.clone()));
            }
        }
        for &b in &c.bases {
            if b != 0 {
                dt_addrs
                    .entry(b)
                    .or_default()
                    .push((c.class.clone(), format!("base:{:#x}", b)));
            }
        }
    }

    let mut matches = Vec::new();
    let mut matched_usb = std::collections::BTreeSet::new();

    for dev in &usb.platform_devices {
        let (addr, name) = parse_usb_device(dev);
        let usb_class = classify_usb_name(name);

        // Absolute address hit (ignore 0 — DT placeholders / 0.dtbo)
        if let Some(a) = addr {
            if a != 0 {
                if let Some(hits) = dt_addrs.get(&a) {
                    let dt_class = hits.first().map(|(c, _)| c.clone());
                    let nodes: Vec<String> = hits.iter().map(|(_, n)| n.clone()).collect();
                    matches.push(CrossMatch {
                        usb_device: dev.clone(),
                        usb_addr: addr,
                        usb_class: usb_class.clone(),
                        dt_class,
                        dt_nodes: nodes,
                        hit: CrossHitKind::Address,
                        note: "absolute addr match USB↔DT".into(),
                    });
                    matched_usb.insert(dev.clone());
                    continue;
                }
                // Relative: low 16–24 bits match DT offsets — only same OS-port class
                if let Some(dt_cls) = map_usb_class_to_dt(&usb_class) {
                    let low16 = a & 0xffff;
                    let low24 = a & 0xff_ffff;
                    let candidate = dt_addrs
                        .get(&low24)
                        .or_else(|| dt_addrs.get(&low16))
                        .and_then(|hits| {
                            let filtered: Vec<_> = hits
                                .iter()
                                .filter(|(c, _)| c == dt_cls)
                                .cloned()
                                .collect();
                            if filtered.is_empty() {
                                None
                            } else {
                                Some(filtered)
                            }
                        });
                    if let Some(hits) = candidate {
                        let nodes: Vec<String> = hits.iter().map(|(_, n)| n.clone()).collect();
                        matches.push(CrossMatch {
                            usb_device: dev.clone(),
                            usb_addr: addr,
                            usb_class: usb_class.clone(),
                            dt_class: Some(dt_cls.into()),
                            dt_nodes: nodes,
                            hit: CrossHitKind::Address,
                            note: format!(
                                "bus-relative match (USB {a:#x} low bits ↔ DT {dt_cls} offset)"
                            ),
                        });
                        matched_usb.insert(dev.clone());
                        continue;
                    }
                }
            }
        }

        if let Some(dt_cls) = map_usb_class_to_dt(&usb_class) {
            if let Some(nodes) = dt_by_class.get(dt_cls) {
                matches.push(CrossMatch {
                    usb_device: dev.clone(),
                    usb_addr: addr,
                    usb_class: usb_class.clone(),
                    dt_class: Some(dt_cls.into()),
                    dt_nodes: nodes.iter().take(4).cloned().collect(),
                    hit: CrossHitKind::ClassOnly,
                    note: "class co-presence (DT bases often bus-relative 0x0)".into(),
                });
                matched_usb.insert(dev.clone());
            }
        }
    }

    let usb_only: Vec<String> = usb
        .platform_devices
        .iter()
        .filter(|d| !matched_usb.contains(*d))
        .cloned()
        .collect();

    let mut dt_class_coverage = BTreeMap::new();
    for c in &plat.components {
        let usb_hit = matches.iter().any(|m| m.dt_class.as_deref() == Some(c.class.as_str()));
        let status = format!("{:?}{}", c.status, if usb_hit { " +USB" } else { "" });
        dt_class_coverage.insert(c.class.clone(), status);
    }

    let bringup = build_bringup(usb, plat, &matches);

    UsbDtCrossReport {
        ok: !matches.is_empty(),
        usb_devices: usb.platform_devices.len(),
        matches,
        usb_only,
        dt_class_coverage,
        bringup,
        port_target: "linux_wedge_uart_ufs_g35".into(),
        generates_os: false,
        auto_fix_complete: false,
        honesty: base_core::HONESTY_NOTE.to_string(),
        note: "Alvo: wedge Linux/AArch64 early bring-up (UART→timer/GIC→UFS). GPU/modem later. ≠ TaurOS turnkey."
            .into(),
    }
}

fn build_bringup(
    usb: &UsbHwInventory,
    plat: &PlatformInventory,
    matches: &[CrossMatch],
) -> Vec<BringupStep> {
    let steps_def: &[(&str, &str, &str, &str)] = &[
        (
            "uart",
            "P0",
            "uart",
            "Consola early: escolher um `*.serial` (ex. 20200000); clock+pinctrl; console=ttyS*",
        ),
        (
            "arm_generic_timer",
            "P0",
            "arm_generic_timer",
            "CNTFRQ/CNTV — sched_clock; USB timers 6404xxxx são periféricos, não arch timer",
        ),
        (
            "gic",
            "P0",
            "gic",
            "GIC no DTB (interrupt-controller@…); mapear GICD/GICR antes de IRQs de UART/UFS",
        ),
        (
            "storage_emmc_ufs",
            "P1",
            "storage_emmc_ufs",
            "Rootfs: `22000000.ufs` (+ sdio); rails PMIC; FlashType em flash.cfg",
        ),
        (
            "gpio",
            "P1",
            "gpio",
            "Pinctrl para UART/UFS; bancos 6417xxxx / 6420xxxx no USB",
        ),
        (
            "gpu_framebuffer",
            "P2",
            "gpu_framebuffer",
            "Opcional early: `23140000.gpu` + DPU; Mali+IOMMU depois",
        ),
        (
            "usb",
            "P2",
            "usb",
            "Host/gadget: `25100000.usb3` / dwc3 — útil para debug, não bloqueia bring-up",
        ),
    ];

    let mut out = Vec::new();
    for (i, (class, prio, dt_class, action)) in steps_def.iter().enumerate() {
        let usb_devs: Vec<String> = usb
            .platform_devices
            .iter()
            .filter(|d| {
                let (_, name) = parse_usb_device(d);
                let uc = classify_usb_name(name);
                &uc == class
                    || (*class == "usb" && uc == "usb")
                    || (*class == "storage_emmc_ufs" && uc == "storage_emmc_ufs")
                    || (*class == "gpu_framebuffer" && uc == "gpu_framebuffer")
                    || (*class == "uart" && uc == "uart")
                    || (*class == "gpio" && uc == "gpio")
                    || (*class == "arm_generic_timer" && uc == "arm_generic_timer")
            })
            .cloned()
            .collect();

        let dt_comp = plat.components.iter().find(|c| c.class == *dt_class);
        let dt_status = dt_comp.map(|c| format!("{:?}", c.status));
        let dt_nodes = dt_comp
            .map(|c| c.nodes.iter().take(4).cloned().collect())
            .unwrap_or_default();

        let _ = matches; // coverage already in report
        out.push(BringupStep {
            order: (i + 1) as u32,
            class: (*class).into(),
            priority: (*prio).into(),
            usb_devices: usb_devs,
            dt_status,
            dt_nodes,
            action: (*action).into(),
        });
    }
    out
}

/// Load YAML files and cross.
pub fn cross_usb_dt_files(
    usb_yaml: &str,
    platform_yaml: &str,
) -> anyhow::Result<UsbDtCrossReport> {
    let usb: UsbHwInventory = serde_yaml::from_str(usb_yaml)?;
    // PlatformInventory uses &'static str fields — deserialize via Value + owned helper.
    let plat = platform_from_yaml(platform_yaml)?;
    Ok(cross_usb_dt(&usb, &plat))
}

/// Owned subset enough for crossing (avoids &'static str deserialize issues).
#[derive(Debug, Deserialize)]
struct PlatformYaml {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    components: Vec<PlatformComponentYaml>,
}

#[derive(Debug, Deserialize)]
struct PlatformComponentYaml {
    class: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    nodes: Vec<String>,
    #[serde(default)]
    bases: Vec<u64>,
}

fn platform_from_yaml(yaml: &str) -> anyhow::Result<PlatformInventory> {
    let y: PlatformYaml = serde_yaml::from_str(yaml)?;
    let components = y
        .components
        .into_iter()
        .map(|c| {
            let status = match c.status.to_ascii_lowercase().as_str() {
                "found" => crate::platform::DiscoveryStatus::Found,
                "partial" => crate::platform::DiscoveryStatus::Partial,
                "missing" => crate::platform::DiscoveryStatus::Missing,
                _ => crate::platform::DiscoveryStatus::Unknown,
            };
            crate::platform::PlatformComponent {
                class: c.class,
                status,
                compatible: vec![],
                nodes: c.nodes,
                bases: c.bases,
                notes: String::new(),
                rewrite_hint: String::new(),
            }
        })
        .collect();
    Ok(PlatformInventory {
        claim: "usb_dt_cross",
        generates_os: false,
        auto_fix_complete: false,
        model: y.model,
        root_compatible: vec![],
        cpu: crate::platform::CpuDiscovery {
            status: crate::platform::DiscoveryStatus::Unknown,
            isa_hint: String::new(),
            compatible: vec![],
            cores_hint: None,
            notes: String::new(),
        },
        components,
        dtb_stats: crate::platform::DtbStats {
            mmio_regions: 0,
            irqs: 0,
            gpios: 0,
            i2c_buses: 0,
            spi_buses: 0,
            dma_controllers: 0,
        },
        os_port_readiness: crate::platform::OsPortReadiness {
            required: vec![],
            found: vec![],
            missing: vec![],
            score: 0.0,
            guidance: vec![],
        },
        honesty: base_core::HONESTY_NOTE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{
        CpuDiscovery, DiscoveryStatus, DtbStats, OsPortReadiness, PlatformComponent,
    };
    use crate::usb_probe::UsbProbeMode;

    fn empty_plat() -> PlatformInventory {
        PlatformInventory {
            claim: "test",
            generates_os: false,
            auto_fix_complete: false,
            model: Some("test".into()),
            root_compatible: vec![],
            cpu: CpuDiscovery {
                status: DiscoveryStatus::Found,
                isa_hint: "a64".into(),
                compatible: vec![],
                cores_hint: Some(1),
                notes: String::new(),
            },
            components: vec![PlatformComponent {
                class: "uart".into(),
                status: DiscoveryStatus::Found,
                compatible: vec![],
                nodes: vec!["soc/ap-apb/serial@0".into(), "soc/mm/gpu@23140000".into()],
                bases: vec![0, 0x10000],
                notes: String::new(),
                rewrite_hint: String::new(),
            }],
            dtb_stats: DtbStats {
                mmio_regions: 0,
                irqs: 0,
                gpios: 0,
                i2c_buses: 0,
                spi_buses: 0,
                dma_controllers: 0,
            },
            os_port_readiness: OsPortReadiness {
                required: vec![],
                found: vec![],
                missing: vec![],
                score: 0.0,
                guidance: vec![],
            },
            honesty: "test",
        }
    }

    #[test]
    fn absolute_gpu_and_class_uart() {
        let mut usb = UsbHwInventory {
            ok: true,
            skipped: false,
            skip_reason: None,
            mode: UsbProbeMode::Adb,
            ..Default::default()
        };
        usb.platform_devices = vec![
            "23140000.gpu".into(),
            "20200000.serial".into(),
        ];
        // Fix plat: uart component shouldn't include gpu node — split
        let mut plat = empty_plat();
        plat.components = vec![
            PlatformComponent {
                class: "uart".into(),
                status: DiscoveryStatus::Found,
                compatible: vec![],
                nodes: vec!["soc/ap-apb/serial@0".into()],
                bases: vec![0],
                notes: String::new(),
                rewrite_hint: String::new(),
            },
            PlatformComponent {
                class: "gpu_framebuffer".into(),
                status: DiscoveryStatus::Found,
                compatible: vec![],
                nodes: vec!["soc/mm/gpu@23140000".into()],
                bases: vec![],
                notes: String::new(),
                rewrite_hint: String::new(),
            },
        ];
        let r = cross_usb_dt(&usb, &plat);
        assert!(r.ok);
        assert!(r.matches.iter().any(|m| m.hit == CrossHitKind::Address));
        assert_eq!(r.port_target, "linux_wedge_uart_ufs_g35");
        assert!(!r.generates_os);
        assert!(!r.bringup.is_empty());
    }
}
