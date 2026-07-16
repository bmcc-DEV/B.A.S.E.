//! Platform inventory for OS port — CPU, GIC, timer, MMU, DRAM, UART, GPIO,
//! PMIC, eMMC/UFS, GPU/FB, Device Tree coverage.
//!
//! Evidence sources: DTB/DTBO (primary), flash.cfg hints, optional MMIO map.
//! Does **not** invent silicon; marks gaps as `missing` / `unknown`.

use serde::{Deserialize, Serialize};
use specter_probe::acquisition::dtb::parse_dtb;
use specter_probe::acquisition::DtbInfo;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryStatus {
    Found,
    Partial,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformComponent {
    pub class: String,
    pub status: DiscoveryStatus,
    pub compatible: Vec<String>,
    pub nodes: Vec<String>,
    pub bases: Vec<u64>,
    pub notes: String,
    pub rewrite_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInventory {
    pub claim: &'static str,
    pub generates_os: bool,
    pub model: Option<String>,
    pub root_compatible: Vec<String>,
    pub cpu: CpuDiscovery,
    pub components: Vec<PlatformComponent>,
    pub dtb_stats: DtbStats,
    pub os_port_readiness: OsPortReadiness,
    pub honesty: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuDiscovery {
    pub status: DiscoveryStatus,
    pub isa_hint: String,
    pub compatible: Vec<String>,
    pub cores_hint: Option<u32>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DtbStats {
    pub mmio_regions: usize,
    pub irqs: usize,
    pub gpios: usize,
    pub i2c_buses: usize,
    pub spi_buses: usize,
    pub dma_controllers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsPortReadiness {
    pub required: Vec<String>,
    pub found: Vec<String>,
    pub missing: Vec<String>,
    pub score: f64,
    pub guidance: Vec<String>,
}

const REQUIRED_CLASSES: &[&str] = &[
    "cpu",
    "gic",
    "arm_generic_timer",
    "mmu",
    "dram_controller",
    "uart",
    "gpio",
    "pmic",
    "storage_emmc_ufs",
    "gpu_framebuffer",
    "device_tree",
];

/// Extract raw FDT blobs from Android DTBO / vendor_boot / raw .dtb files.
pub fn extract_fdt_blobs(data: &[u8]) -> Vec<Vec<u8>> {
    let magic = [0xd0u8, 0x0d, 0xfe, 0xed];
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 8 <= data.len() {
        if data[i..i + 4] != magic {
            i += 1;
            continue;
        }
        let totalsize = u32::from_be_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]])
            as usize;
        if totalsize >= 8 && i + totalsize <= data.len() && totalsize < 16 * 1024 * 1024 {
            out.push(data[i..i + totalsize].to_vec());
            i += totalsize;
        } else {
            i += 4;
        }
        if out.len() >= 16 {
            break;
        }
    }
    out
}

pub fn build_platform_from_dtb_bytes(dtb: &[u8], flash_cfg: Option<&str>) -> anyhow::Result<PlatformInventory> {
    let info = parse_dtb(dtb)?;
    Ok(build_platform_from_dtb_info(&info, flash_cfg, true))
}

pub fn build_platform_from_path(path: &Path, flash_cfg: Option<&str>) -> anyhow::Result<PlatformInventory> {
    let data = std::fs::read(path)?;
    // raw DTB or container with embedded FDTs
    if data.len() >= 4 && data[0..4] == [0xd0, 0x0d, 0xfe, 0xed] {
        return build_platform_from_dtb_bytes(&data, flash_cfg);
    }
    let blobs = extract_fdt_blobs(&data);
    if blobs.is_empty() {
        anyhow::bail!("no FDT (0xd00dfeed) found in {}", path.display());
    }
    // Merge: parse largest blob as primary (usually fullest board DT)
    let primary = blobs.iter().max_by_key(|b| b.len()).unwrap();
    let mut inv = build_platform_from_dtb_bytes(primary, flash_cfg)?;
    inv.notes_push(format!(
        "parsed largest of {} embedded FDT blob(s) from {}",
        blobs.len(),
        path.display()
    ));
    // Supplement missing classes from smaller blobs
    for blob in blobs.iter().filter(|b| !std::ptr::eq(b.as_slice(), primary.as_slice())) {
        if let Ok(info) = parse_dtb(blob) {
            merge_dtb_into(&mut inv, &info);
        }
    }
    Ok(inv)
}

trait NotesExt {
    fn notes_push(&mut self, s: String);
}
impl NotesExt for PlatformInventory {
    fn notes_push(&mut self, s: String) {
        self.cpu.notes = format!("{} | {}", self.cpu.notes, s);
    }
}

pub fn build_platform_from_dtb_info(
    info: &DtbInfo,
    flash_cfg: Option<&str>,
    has_dtb: bool,
) -> PlatformInventory {
    let mut components = Vec::new();

    components.push(classify_gic(info));
    components.push(classify_timer(info));
    components.push(classify_mmu(info));
    components.push(classify_dram(info));
    components.push(classify_uart(info));
    components.push(classify_gpio(info));
    components.push(classify_pmic(info));
    components.push(classify_storage(info));
    components.push(classify_gpu_fb(info));
    components.push(PlatformComponent {
        class: "device_tree".into(),
        status: if has_dtb {
            DiscoveryStatus::Found
        } else {
            DiscoveryStatus::Missing
        },
        compatible: info.compatible.clone(),
        nodes: vec!["/".into()],
        bases: vec![],
        notes: info
            .model
            .clone()
            .unwrap_or_else(|| "DTB present".into()),
        rewrite_hint: "Ship/overlay DTB for target OS; keep as SoC truth source".into(),
    });

    if let Some(cfg) = flash_cfg {
        apply_flash_cfg_hints(&mut components, cfg);
    }

    let cpu = discover_cpu(info, flash_cfg);
    let found: Vec<String> = components
        .iter()
        .filter(|c| c.status == DiscoveryStatus::Found || c.status == DiscoveryStatus::Partial)
        .map(|c| c.class.clone())
        .collect();
    let mut found_set = found.clone();
    if cpu.status == DiscoveryStatus::Found || cpu.status == DiscoveryStatus::Partial {
        found_set.push("cpu".into());
    }
    let missing: Vec<String> = REQUIRED_CLASSES
        .iter()
        .filter(|c| !found_set.iter().any(|f| f == **c))
        .map(|s| (*s).to_string())
        .collect();
    let score = (REQUIRED_CLASSES.len() - missing.len()) as f64 / REQUIRED_CLASSES.len() as f64;

    let mut guidance = vec![
        "DTB is the primary source for OS bring-up maps — prefer it over heuristic MMIO.".into(),
        "Missing classes block honest 'OS port ready' claims.".into(),
        "Cross-check GIC/timer with ARM architected regs even if DT incomplete.".into(),
    ];
    if missing.iter().any(|m| m == "gic" || m == "arm_generic_timer") {
        guidance.push(
            "Without GIC+generic timer, early kernel boot on AArch64 will fail.".into(),
        );
    }
    if missing.iter().any(|m| m == "uart") {
        guidance.push("No UART in DT — earlyprintk/console will need manual bind.".into());
    }

    PlatformInventory {
        claim: "platform_inventory_assist",
        generates_os: false,
        model: info.model.clone(),
        root_compatible: info.compatible.clone(),
        cpu,
        components,
        dtb_stats: DtbStats {
            mmio_regions: info.mmio_regions.len(),
            irqs: info.irqs.len(),
            gpios: info.gpios.len(),
            i2c_buses: info.i2c_buses.len(),
            spi_buses: info.spi_buses.len(),
            dma_controllers: info.dma_controllers.len(),
        },
        os_port_readiness: OsPortReadiness {
            required: REQUIRED_CLASSES.iter().map(|s| (*s).to_string()).collect(),
            found: found_set,
            missing,
            score,
            guidance,
        },
        honesty: "Inventory from DTB/flash.cfg — ≠ full OS port; gaps stay Missing",
    }
}

fn discover_cpu(info: &DtbInfo, flash_cfg: Option<&str>) -> CpuDiscovery {
    let mut compat = Vec::new();
    let mut cores = 0u32;
    for r in &info.mmio_regions {
        for c in &r.compatible {
            let lc = c.to_lowercase();
            if lc.contains("arm,cortex")
                || lc.contains("arm,armv8")
                || lc.contains("arm,armv7")
                || (lc.starts_with("arm,") && lc.contains("cpu"))
            {
                compat.push(c.clone());
            }
        }
        if let Some(p) = &r.peripheral {
            if p.contains("cpu@") || p.ends_with("/cpu") || p.contains("/cpus/") {
                cores += 1;
            }
        }
    }
    // Also scan root compatible for SoC (implies ISA family)
    for c in &info.compatible {
        if c.to_lowercase().contains("unisoc")
            || c.to_lowercase().contains("sprd")
            || c.to_lowercase().contains("qcom")
            || c.to_lowercase().contains("arm,")
        {
            if !compat.contains(c) {
                compat.push(c.clone());
            }
        }
    }

    let mut isa = "unknown".to_string();
    let joined = compat.join(" ").to_lowercase();
    let has_a76 = joined.contains("cortex-a76");
    let has_a55 = joined.contains("cortex-a55");
    let has_a53 = joined.contains("cortex-a53");
    if has_a76 && has_a55 {
        isa = "ARMv8.2-A / Cortex-A76 + Cortex-A55 (big.LITTLE)".into();
    } else if has_a55 {
        isa = "ARMv8.2-A / Cortex-A55".into();
    } else if has_a53 {
        isa = "ARMv8-A / Cortex-A53".into();
    } else if has_a76 {
        isa = "ARMv8.2-A / Cortex-A76".into();
    } else if joined.contains("armv8") || joined.contains("cortex-a") {
        isa = "ARMv8-A (cortex family)".into();
    } else if flash_cfg.map(|s| s.contains("ums9620") || s.contains("Qogir")).unwrap_or(false) {
        // Unisoc T760 / ums9620: public specs → A76/A55 cluster
        isa = "ARMv8.2-A (Unisoc T760: Cortex-A76 + Cortex-A55) — from product id, confirm in DT".into();
        compat.push("hint:unisoc,t760".into());
    }

    let status = if isa.starts_with("ARMv8") && !isa.contains("confirm") {
        DiscoveryStatus::Found
    } else if isa.contains("ARMv8") {
        DiscoveryStatus::Partial
    } else if !compat.is_empty() {
        DiscoveryStatus::Partial
    } else {
        DiscoveryStatus::Missing
    };

    compat.sort();
    compat.dedup();

    CpuDiscovery {
        status,
        isa_hint: isa,
        compatible: compat,
        cores_hint: if cores > 0 { Some(cores) } else { None },
        notes: "Prefer /cpus nodes in DTB; product flash.cfg is only a hint".into(),
    }
}

fn filter_regions<'a>(info: &'a DtbInfo, pred: impl Fn(&str) -> bool) -> Vec<&'a specter_probe::acquisition::MmioRegion> {
    info.mmio_regions
        .iter()
        .filter(|r| {
            r.compatible.iter().any(|c| pred(&c.to_lowercase()))
                || r.peripheral
                    .as_ref()
                    .map(|p| pred(&p.to_lowercase()))
                    .unwrap_or(false)
        })
        .collect()
}

fn component_from_regions(
    class: &str,
    regions: &[&specter_probe::acquisition::MmioRegion],
    found_notes: &str,
    missing_notes: &str,
    hint: &str,
) -> PlatformComponent {
    if regions.is_empty() {
        return PlatformComponent {
            class: class.into(),
            status: DiscoveryStatus::Missing,
            compatible: vec![],
            nodes: vec![],
            bases: vec![],
            notes: missing_notes.into(),
            rewrite_hint: hint.into(),
        };
    }
    let mut compat = Vec::new();
    let mut nodes = Vec::new();
    let mut bases = Vec::new();
    for r in regions {
        for c in &r.compatible {
            if !compat.contains(c) {
                compat.push(c.clone());
            }
        }
        if let Some(p) = &r.peripheral {
            nodes.push(p.clone());
        }
        bases.push(r.address);
    }
    PlatformComponent {
        class: class.into(),
        status: DiscoveryStatus::Found,
        compatible: compat,
        nodes,
        bases,
        notes: found_notes.into(),
        rewrite_hint: hint.into(),
    }
}

fn classify_gic(info: &DtbInfo) -> PlatformComponent {
    let regs = filter_regions(info, |s| {
        s.contains("arm,gic")
            || s.contains("arm,gic-v3")
            || s.contains("arm,gic-v2")
            || s.contains("interrupt-controller")
            || s.contains("/gic")
    });
    component_from_regions(
        "gic",
        &regs,
        "GIC node(s) in DTB",
        "No GIC compatible in DTB",
        "Map GICD/GICR (v3) or GICD/GICC (v2) before IRQ bring-up",
    )
}

fn classify_timer(info: &DtbInfo) -> PlatformComponent {
    let regs = filter_regions(info, |s| {
        s.contains("arm,armv8-timer")
            || s.contains("arm,armv7-timer")
            || s.contains("arm_sys_timer")
            || s.contains("arch_timer")
            || s.contains("generic-timer")
    });
    let mut c = component_from_regions(
        "arm_generic_timer",
        &regs,
        "ARM generic/architected timer in DTB",
        "No arm,armv8-timer (etc.) in DTB — may still exist as system reg",
        "Enable CNTFRQ/CNTV_CTL; required for sched_clock",
    );
    // Timer often has no MMIO reg — only interrupts. Detect via irq peripherals.
    if c.status == DiscoveryStatus::Missing {
        let hit = info.irqs.iter().any(|i| {
            let p = i.peripheral.to_lowercase();
            p.contains("timer") || p.contains("arch_timer")
        });
        if hit {
            c.status = DiscoveryStatus::Partial;
            c.notes = "timer IRQs present without reg — typical for arch timer".into();
            c.nodes = info
                .irqs
                .iter()
                .filter(|i| i.peripheral.to_lowercase().contains("timer"))
                .map(|i| i.peripheral.clone())
                .collect();
        }
    }
    c
}

fn classify_mmu(info: &DtbInfo) -> PlatformComponent {
    // MMU is CPU feature; look for iommu / smmu
    let regs = filter_regions(info, |s| {
        s.contains("iommu") || s.contains("smmu") || s.contains("arm,mmu")
    });
    let mut c = component_from_regions(
        "mmu",
        &regs,
        "IOMMU/SMMU nodes found (stage-2); CPU MMU is architected",
        "No SMMU/IOMMU in DTB — CPU MMU still assumed for AArch64",
        "AArch64 kernel needs TTBR/TCR; SMMU separate for DMA",
    );
    if c.status == DiscoveryStatus::Missing {
        // For AArch64 SoCs, MMU is implied
        if info.compatible.iter().any(|c| {
            let l = c.to_lowercase();
            l.contains("unisoc") || l.contains("sprd") || l.contains("qcom") || l.contains("arm")
        }) || !info.mmio_regions.is_empty()
        {
            c.status = DiscoveryStatus::Partial;
            c.notes =
                "CPU MMU implied for AArch64 bring-up; no SMMU node — document as Partial".into();
        }
    }
    c
}

fn classify_dram(info: &DtbInfo) -> PlatformComponent {
    let regs = filter_regions(info, |s| {
        s.contains("memory-controller")
            || s.contains("ddr")
            || s.contains("dmc")
            || s.contains("umctl")
            || s.contains("memory@")
            || s == "memory"
    });
    component_from_regions(
        "dram_controller",
        &regs,
        "memory / DRAM controller nodes",
        "No DRAM controller in DTB — check /memory reg for size only",
        "Need mem size + controller for early boot; LK may already init DRAM",
    )
}

fn classify_uart(info: &DtbInfo) -> PlatformComponent {
    let regs = filter_regions(info, |s| {
        s.contains("uart") || s.contains("serial") || s.contains("8250") || s.contains("pl011")
    });
    component_from_regions(
        "uart",
        &regs,
        "UART/serial nodes",
        "No UART in DTB",
        "Bind console=ttyS/ttyAMA; need clock + pinctrl",
    )
}

fn classify_gpio(info: &DtbInfo) -> PlatformComponent {
    if !info.gpios.is_empty() {
        return PlatformComponent {
            class: "gpio".into(),
            status: DiscoveryStatus::Found,
            compatible: vec![],
            nodes: info
                .gpios
                .iter()
                .map(|g| format!("gpio_bank{}@{:x}", g.bank, g.base))
                .collect(),
            bases: info.gpios.iter().map(|g| g.base).collect(),
            notes: format!("{} GPIO bank(s) from DTB walk", info.gpios.len()),
            rewrite_hint: "Export banks to pinctrl for UART/MMC".into(),
        };
    }
    let regs = filter_regions(info, |s| s.contains("gpio") || s.contains("pinctrl"));
    component_from_regions(
        "gpio",
        &regs,
        "gpio/pinctrl nodes",
        "No GPIO in DTB",
        "Need at least one bank for bring-up straps / UART mux",
    )
}

fn classify_pmic(info: &DtbInfo) -> PlatformComponent {
    let regs = filter_regions(info, |s| {
        s.contains("pmic")
            || s.contains("regulator")
            || s.contains("max77")
            || s.contains("sc27")
            || s.contains("fan53")
            || s.contains("power-controller")
    });
    component_from_regions(
        "pmic",
        &regs,
        "PMIC/regulator nodes",
        "No PMIC/regulator in DTB",
        "Power rails for eMMC/UFS/UART often behind PMIC — critical for storage",
    )
}

fn classify_storage(info: &DtbInfo) -> PlatformComponent {
    let regs = filter_regions(info, |s| {
        s.contains("ufshc")
            || s.contains("ufs")
            || s.contains("mmc")
            || s.contains("sdhci")
            || s.contains("emmc")
            || s.contains("dw-mshc")
    });
    component_from_regions(
        "storage_emmc_ufs",
        &regs,
        "eMMC/UFS/SDHCI nodes",
        "No eMMC/UFS in DTB",
        "Rootfs needs block device; check flash.cfg FlashType",
    )
}

fn classify_gpu_fb(info: &DtbInfo) -> PlatformComponent {
    let regs = filter_regions(info, |s| {
        s.contains("mali")
            || s.contains("gpu")
            || s.contains("framebuffer")
            || s.contains("display")
            || s.contains("drm")
            || s.contains("dpu")
            || s.contains("dsi")
    });
    component_from_regions(
        "gpu_framebuffer",
        &regs,
        "GPU/display nodes",
        "No GPU/FB in DTB — console may be UART-only initially",
        "FB optional for early bring-up; Mali needs IOMMU later",
    )
}

fn apply_flash_cfg_hints(components: &mut [PlatformComponent], cfg: &str) {
    let lc = cfg.to_lowercase();
    for c in components.iter_mut() {
        if c.class == "storage_emmc_ufs" && c.status == DiscoveryStatus::Missing {
            if lc.contains("ufs") || lc.contains("emmc") || lc.contains("flashtype") {
                c.status = DiscoveryStatus::Partial;
                c.notes = format!("flash.cfg hint present; confirm DT. snippet scan ok");
            }
        }
    }
}

fn merge_dtb_into(inv: &mut PlatformInventory, info: &DtbInfo) {
    let extra = build_platform_from_dtb_info(info, None, true);
    for ec in extra.components {
        if let Some(slot) = inv.components.iter_mut().find(|c| c.class == ec.class) {
            if slot.status == DiscoveryStatus::Missing
                && (ec.status == DiscoveryStatus::Found || ec.status == DiscoveryStatus::Partial)
            {
                *slot = ec;
            } else if slot.status == DiscoveryStatus::Partial && ec.status == DiscoveryStatus::Found
            {
                *slot = ec;
            } else {
                // merge bases/nodes
                for b in ec.bases {
                    if !slot.bases.contains(&b) {
                        slot.bases.push(b);
                    }
                }
                for n in ec.nodes {
                    if !slot.nodes.contains(&n) {
                        slot.nodes.push(n);
                    }
                }
            }
        }
    }
    if inv.cpu.status != DiscoveryStatus::Found
        && (extra.cpu.status == DiscoveryStatus::Found
            || extra.cpu.status == DiscoveryStatus::Partial)
    {
        inv.cpu = extra.cpu;
    }
    // refresh readiness
    let found: Vec<String> = inv
        .components
        .iter()
        .filter(|c| c.status == DiscoveryStatus::Found || c.status == DiscoveryStatus::Partial)
        .map(|c| c.class.clone())
        .chain(
            if inv.cpu.status == DiscoveryStatus::Found
                || inv.cpu.status == DiscoveryStatus::Partial
            {
                vec!["cpu".into()]
            } else {
                vec![]
            }
            .into_iter(),
        )
        .collect();
    let missing: Vec<String> = REQUIRED_CLASSES
        .iter()
        .filter(|c| !found.iter().any(|f| f == **c))
        .map(|s| (*s).to_string())
        .collect();
    let score = (REQUIRED_CLASSES.len() - missing.len()) as f64 / REQUIRED_CLASSES.len() as f64;
    inv.os_port_readiness.found = found;
    inv.os_port_readiness.missing = missing;
    inv.os_port_readiness.score = score;
}

impl PlatformInventory {
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# PLATFORM_INVENTORY — OS port prerequisites\n\n");
        md.push_str("> Descoberta a partir de **Device Tree** (+ hints). ≠ gera o OS.\n\n");
        if let Some(m) = &self.model {
            md.push_str(&format!("- model: **{m}**\n"));
        }
        md.push_str(&format!(
            "- root compatible: `{}`\n",
            self.root_compatible.join(", ")
        ));
        md.push_str(&format!(
            "- readiness score: **{:.0}%** (found {} / {})\n",
            self.os_port_readiness.score * 100.0,
            self.os_port_readiness.found.len(),
            self.os_port_readiness.required.len()
        ));
        md.push_str(&format!(
            "- generates_os: **{}**\n\n",
            self.generates_os
        ));

        md.push_str("## CPU\n\n");
        let cores = self
            .cpu
            .cores_hint
            .map(|n| n.to_string())
            .unwrap_or_else(|| "unknown".into());
        let mut cpu_compat = self.cpu.compatible.clone();
        cpu_compat.sort();
        cpu_compat.dedup();
        md.push_str(&format!(
            "- status: `{:?}`\n- ISA hint: **{}**\n- cores hint: {}\n- compatible: {}\n- notes: {}\n\n",
            self.cpu.status,
            self.cpu.isa_hint,
            cores,
            cpu_compat.join(", "),
            self.cpu.notes
        ));

        md.push_str("## Required OS-port components\n\n");
        md.push_str("| Class | Status | Bases | Notes |\n|-------|--------|-------|-------|\n");
        // CPU row
        md.push_str(&format!(
            "| cpu | {:?} | — | {} |\n",
            self.cpu.status, self.cpu.isa_hint
        ));
        for c in &self.components {
            let bases = if c.bases.is_empty() {
                "—".into()
            } else {
                c.bases
                    .iter()
                    .take(4)
                    .map(|b| format!("`0x{b:x}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            md.push_str(&format!(
                "| {} | {:?} | {} | {} |\n",
                c.class, c.status, bases, c.notes.replace('|', "/")
            ));
        }

        md.push_str("\n## Missing (blockers)\n\n");
        if self.os_port_readiness.missing.is_empty() {
            md.push_str("- (none in checklist — still ≠ OS complete)\n");
        } else {
            for m in &self.os_port_readiness.missing {
                md.push_str(&format!("- `{m}`\n"));
            }
        }

        md.push_str("\n## DTB stats\n\n");
        md.push_str(&format!(
            "- mmio_regions: {}\n- irqs: {}\n- gpios: {}\n- i2c: {}\n- spi: {}\n- dma: {}\n",
            self.dtb_stats.mmio_regions,
            self.dtb_stats.irqs,
            self.dtb_stats.gpios,
            self.dtb_stats.i2c_buses,
            self.dtb_stats.spi_buses,
            self.dtb_stats.dma_controllers
        ));

        md.push_str("\n## Guidance\n\n");
        for g in &self.os_port_readiness.guidance {
            md.push_str(&format!("- {g}\n"));
        }
        md.push_str("\n## Per-class rewrite hints\n\n");
        for c in &self.components {
            md.push_str(&format!("- **{}**: {}\n", c.class, c.rewrite_hint));
        }
        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_fdt_finds_magic() {
        let mut buf = vec![0u8; 32];
        buf[8..12].copy_from_slice(&[0xd0, 0x0d, 0xfe, 0xed]);
        buf[12..16].copy_from_slice(&20u32.to_be_bytes());
        let blobs = extract_fdt_blobs(&buf);
        assert_eq!(blobs.len(), 1);
        assert_eq!(blobs[0].len(), 20);
    }
}
