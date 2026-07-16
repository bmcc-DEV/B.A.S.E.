//! F1 — HardwareSpec (+ Evidence) → BIR → DigitalTwin replay.
//!
//! ≠ OS turnkey · shadow/BIR assist only.

use base_bir::types::{
    BirAccess, BirDevice, BirEvent, BirInterrupt, BirLatencyRange, BirRegister, BirTimingEntry,
    BirTrigger, BlockKind as BirBlockKind, IrqPolarity as BirIrqPolarity, IrqType as BirIrqType,
    TriggerKind,
};
use base_core::evidence::{EvidenceDb, EvidenceType};
use base_core::honesty;
use base_core::spec::types::{AccessType, BlockKind, HardwareSpec, IrqPolarity, IrqType};
use base_core::twin::{DigitalTwin, TwinStatus};
use serde::{Deserialize, Serialize};

fn map_access(a: AccessType) -> BirAccess {
    match a {
        AccessType::Read => BirAccess::Read,
        AccessType::Write | AccessType::WriteOnly | AccessType::WriteClear | AccessType::WriteToggle => {
            BirAccess::Write
        }
        AccessType::ReadWrite | AccessType::ReadDestruct => BirAccess::ReadWrite,
    }
}

fn map_kind(k: BlockKind) -> BirBlockKind {
    match k {
        BlockKind::Gpu => BirBlockKind::Gpu,
        BlockKind::Audio => BirBlockKind::Audio,
        BlockKind::Dma => BirBlockKind::Dma,
        BlockKind::Usb => BirBlockKind::Usb,
        BlockKind::Ethernet => BirBlockKind::Ethernet,
        BlockKind::Spi => BirBlockKind::Spi,
        BlockKind::I2c => BirBlockKind::I2c,
        BlockKind::Uart => BirBlockKind::Uart,
        BlockKind::Timer => BirBlockKind::Timer,
        BlockKind::InterruptController => BirBlockKind::InterruptController,
        BlockKind::MemoryController => BirBlockKind::MemoryController,
        BlockKind::Crypto => BirBlockKind::Crypto,
        BlockKind::VideoCodec => BirBlockKind::VideoCodec,
        BlockKind::Isp => BirBlockKind::Isp,
        BlockKind::Npu => BirBlockKind::Npu,
        BlockKind::Unknown => BirBlockKind::Unknown,
    }
}

fn map_irq_type(t: IrqType) -> BirIrqType {
    match t {
        IrqType::Level => BirIrqType::Level,
        IrqType::Edge => BirIrqType::Edge,
    }
}

fn map_irq_pol(p: IrqPolarity) -> BirIrqPolarity {
    match p {
        IrqPolarity::High => BirIrqPolarity::High,
        IrqPolarity::Low => BirIrqPolarity::Low,
    }
}

/// Converte o primeiro bloco (ou `block_id`) do Spec num [`BirDevice`].
pub fn spec_block_to_bir(spec: &HardwareSpec, block_id: Option<&str>) -> Option<BirDevice> {
    let block = if let Some(id) = block_id {
        spec.blocks.iter().find(|b| b.id == id)?
    } else {
        spec.blocks.first()?
    };

    let mut_kind = map_kind(block.kind);
    let _ = mut_kind; // reserved for future BIR kind field

    let mut device = BirDevice::new(&block.id);
    device.base_address = Some(block.base_address);
    device.version = spec.version;

    if block.registers.is_empty() {
        // Synthesize a DATA window so DigitalTwin has something to write.
        device.registers.push(BirRegister {
            name: "DATA".into(),
            offset: 0,
            access: BirAccess::ReadWrite,
            width: 32,
            reset_value: Some(0),
            bitfields: vec![],
        });
        device.registers.push(BirRegister {
            name: "CTRL".into(),
            offset: 4,
            access: BirAccess::ReadWrite,
            width: 32,
            reset_value: Some(0),
            bitfields: vec![],
        });
    } else {
        for r in &block.registers {
            let name = r
                .name
                .clone()
                .unwrap_or_else(|| format!("REG_{:x}", r.offset));
            device.registers.push(BirRegister {
                name,
                offset: r.offset,
                access: map_access(r.access),
                width: r.width,
                reset_value: r.reset_value,
                bitfields: vec![],
            });
        }
    }

    // Generic write-any event on first register (boot / observe).
    if let Some(first) = device.registers.first() {
        device.events.push(BirEvent {
            name: "guest_write".into(),
            trigger: BirTrigger {
                kind: TriggerKind::Write,
                register: first.name.clone(),
                bit_range: None,
                value: None,
            },
            timing: Some(BirLatencyRange::new(10, 100)),
        });
        device.timing.push(BirTimingEntry {
            name: "guest_write".into(),
            latency: BirLatencyRange::new(10, 100),
            per_unit: None,
        });
    }

    for irq in &spec.interrupts {
        if irq.owner == block.id || irq.owner.is_empty() {
            device.interrupts.push(BirInterrupt {
                name: format!("irq_{}", irq.vector),
                vector: irq.vector,
                irq_type: map_irq_type(irq.irq_type),
                polarity: map_irq_pol(irq.polarity),
            });
        }
    }

    Some(device)
}

fn reg_name_for_offset(device: &BirDevice, offset: u64) -> Option<String> {
    device
        .registers
        .iter()
        .find(|r| r.offset as u64 == offset)
        .map(|r| r.name.clone())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirTwinReplayReport {
    pub phase: String,
    pub ok: bool,
    pub device_name: String,
    pub base_address: u64,
    pub writes_applied: usize,
    pub writes_skipped: usize,
    pub twin_steps: u64,
    pub twin_time_ns: u64,
    pub twin_status: String,
    pub register_snapshot: std::collections::HashMap<String, u64>,
    pub generates_os: bool,
    pub auto_fix_complete: bool,
    pub honesty: String,
    pub note: String,
}

/// Constrói BIR + DigitalTwin e aplica writes do guest Evidence.
pub fn replay_bir_twin(
    spec: &HardwareSpec,
    evidence: &EvidenceDb,
    block_id: Option<&str>,
) -> anyhow::Result<(BirDevice, BirTwinReplayReport)> {
    let device = spec_block_to_bir(spec, block_id)
        .ok_or_else(|| anyhow::anyhow!("no functional block for BIR twin"))?;
    let base = device.base_address.unwrap_or(0);
    let mut twin = DigitalTwin::new(device.clone());
    twin.status = TwinStatus::Running;

    let mut applied = 0usize;
    let mut skipped = 0usize;

    for entry in &evidence.entries {
        if let EvidenceType::MmioWrite { address, value } = &entry.evidence_type {
            if *address < base || *address >= base + 0x10000 {
                // Allow within block size if known
                let in_block = spec.blocks.iter().any(|b| {
                    let id_ok = block_id.map(|id| b.id == id).unwrap_or(true);
                    id_ok && *address >= b.base_address && *address < b.base_address + b.size.max(1)
                });
                if !in_block {
                    skipped += 1;
                    continue;
                }
            }
            let offset = address.saturating_sub(base);
            let Some(name) = reg_name_for_offset(&device, offset) else {
                // Create ephemeral write via nearest register or skip
                skipped += 1;
                continue;
            };
            let val = value.unwrap_or(0);
            match twin.write_register(&name, val) {
                Ok(()) => applied += 1,
                Err(_) => skipped += 1,
            }
        }
    }

    let status = match &twin.status {
        TwinStatus::Idle => "idle".into(),
        TwinStatus::Running => "running".into(),
        TwinStatus::Halted => "halted".into(),
        TwinStatus::Error(e) => format!("error:{e}"),
    };

    let report = BirTwinReplayReport {
        phase: "bir_twin".into(),
        ok: true,
        device_name: device.name.clone(),
        base_address: base,
        writes_applied: applied,
        writes_skipped: skipped,
        twin_steps: twin.step_count,
        twin_time_ns: twin.time_ns,
        twin_status: status,
        register_snapshot: twin.registers.clone(),
        generates_os: honesty::GENERATES_OS,
        auto_fix_complete: honesty::AUTO_FIX_COMPLETE,
        honesty: honesty::NOTE.to_string(),
        note: "BIR DigitalTwin replay from guest Evidence — ≠ OS turnkey".into(),
    };

    Ok((device, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::evidence::EvidenceEntry;
    use base_core::spec::types::*;
    use std::collections::HashMap;

    fn mame_spec() -> HardwareSpec {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(FunctionalBlock {
            id: "mame0".into(),
            kind: BlockKind::Unknown,
            base_address: 0xA00000,
            size: 0x1000,
            registers: vec![
                Register {
                    offset: 0,
                    name: Some("DATA".into()),
                    width: 32,
                    access: AccessType::ReadWrite,
                    purpose: RegisterPurpose::DataPort,
                    reset_value: None,
                    observed_values: vec![],
                    bitfields: vec![],
                    polling: false,
                    count: 0,
                },
                Register {
                    offset: 4,
                    name: Some("CTRL".into()),
                    width: 32,
                    access: AccessType::ReadWrite,
                    purpose: RegisterPurpose::Control,
                    reset_value: None,
                    observed_values: vec![],
                    bitfields: vec![],
                    polling: false,
                    count: 0,
                },
            ],
            protocol: Protocol {
                states: vec![],
                transitions: vec![],
                entry_condition: None,
                exit_condition: None,
            },
            timing: TimingProfile {
                activation: None,
                processing: None,
                interrupt_response: None,
                dma_setup: None,
                polling_interval: None,
            },
            dma: None,
            dependencies: vec![],
            confidence: 0.5,
        });
        spec
    }

    #[test]
    fn bir_replay_applies_writes() {
        let mut db = EvidenceDb::new("g");
        db.add(EvidenceEntry {
            id: "e0".into(),
            evidence_type: EvidenceType::MmioWrite {
                address: 0xA00000,
                value: Some(0x12),
            },
            context: HashMap::new(),
        });
        db.add(EvidenceEntry {
            id: "e1".into(),
            evidence_type: EvidenceType::MmioWrite {
                address: 0xA00004,
                value: Some(0x34),
            },
            context: HashMap::new(),
        });
        let (dev, report) = replay_bir_twin(&mame_spec(), &db, None).unwrap();
        assert_eq!(dev.name, "mame0");
        assert_eq!(report.writes_applied, 2);
        assert_eq!(report.register_snapshot.get("DATA"), Some(&0x12));
        assert_eq!(report.register_snapshot.get("CTRL"), Some(&0x34));
        assert!(!report.generates_os);
    }
}
