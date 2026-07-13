use crate::types::*;

/// Bridge: converte entre BIR e o HardwareSpec legado (base-core)
pub mod legacy_bridge {
    use super::*;
    use base_core::spec::types as legacy;

    pub fn from_legacy(spec: &legacy::HardwareSpec) -> Vec<BirDevice> {
        let mut devices = Vec::new();
        for block in &spec.blocks {
            let mut dev = BirDevice::new(&block.id);
            dev.base_address = Some(block.base_address);

            for reg in &block.registers {
                dev.registers.push(BirRegister {
                    name: reg.name.clone().unwrap_or_else(|| format!("REG_{:x}", reg.offset)),
                    offset: reg.offset,
                    access: match reg.access {
                        legacy::AccessType::Read => BirAccess::Read,
                        legacy::AccessType::Write => BirAccess::Write,
                        legacy::AccessType::ReadWrite => BirAccess::ReadWrite,
                        _ => BirAccess::ReadWrite,
                    },
                    width: reg.width,
                    reset_value: reg.reset_value,
                    bitfields: reg.bitfields.iter().map(|bf| BirBitfield {
                        offset: bf.offset, width: bf.width,
                        name: bf.name.clone(), values: bf.values.clone(),
                    }).collect(),
                });
            }

            for irq in &spec.interrupts {
                if irq.owner == block.id {
                    dev.interrupts.push(BirInterrupt {
                        name: format!("IRQ_{}", irq.vector), vector: irq.vector,
                        irq_type: match irq.irq_type {
                            legacy::IrqType::Level => IrqType::Level,
                            legacy::IrqType::Edge => IrqType::Edge,
                        },
                        polarity: match irq.polarity {
                            legacy::IrqPolarity::High => IrqPolarity::High,
                            legacy::IrqPolarity::Low => IrqPolarity::Low,
                        },
                    });
                }
            }

            if let Some(ref proc) = block.timing.processing {
                dev.timing.push(BirTimingEntry {
                    name: "processing".into(),
                    latency: BirLatencyRange::new(proc.min_ns, proc.max_ns),
                    per_unit: None,
                });
            }

            devices.push(dev);
        }
        devices
    }

    pub fn to_legacy(devices: &[BirDevice]) -> legacy::HardwareSpec {
        let mut spec = legacy::HardwareSpec::empty();
        for dev in devices {
            let lower = dev.name.to_lowercase();
            let kind = if lower.contains("gpu") { legacy::BlockKind::Gpu }
            else if lower.contains("audio") { legacy::BlockKind::Audio }
            else if lower.contains("dma") { legacy::BlockKind::Dma }
            else { legacy::BlockKind::Unknown };

            spec.blocks.push(legacy::FunctionalBlock {
                id: dev.name.clone(), kind,
                base_address: dev.base_address.unwrap_or(0), size: 0x1000,
                registers: dev.registers.iter().map(|r| legacy::Register {
                    offset: r.offset, name: Some(r.name.clone()), width: r.width,
                    access: match r.access {
                        BirAccess::Read => legacy::AccessType::Read,
                        BirAccess::Write => legacy::AccessType::Write,
                        _ => legacy::AccessType::ReadWrite,
                    },
                    purpose: legacy::RegisterPurpose::UnknownPurpose,
                    reset_value: r.reset_value, observed_values: vec![],
                    bitfields: vec![], polling: false, count: 0,
                }).collect(),
                protocol: legacy::Protocol {
                    states: vec!["idle".into()], transitions: vec![],
                    entry_condition: None, exit_condition: None,
                },
                timing: legacy::TimingProfile {
                    activation: None,
                    processing: dev.timing.first().map(|t| legacy::LatencyRange {
                        min_ns: t.latency.min_ns, max_ns: t.latency.max_ns,
                        avg_ns: (t.latency.min_ns + t.latency.max_ns) / 2,
                        p99_ns: None, samples: 1,
                    }),
                    interrupt_response: None, dma_setup: None, polling_interval: None,
                },
                dma: None, dependencies: vec![], confidence: 0.8,
            });
        }
        spec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_legacy_empty() {
        let spec = base_core::spec::types::HardwareSpec::empty();
        let devices = legacy_bridge::from_legacy(&spec);
        assert!(devices.is_empty());
    }

    #[test]
    fn test_roundtrip() {
        let mut spec = base_core::spec::types::HardwareSpec::empty();
        spec.blocks.push(base_core::spec::types::FunctionalBlock {
            id: "gpu_0".into(), kind: base_core::spec::types::BlockKind::Gpu,
            base_address: 0x10000000, size: 0x1000,
            registers: vec![base_core::spec::types::Register {
                offset: 0, name: Some("control".into()), width: 32,
                access: base_core::spec::types::AccessType::ReadWrite,
                purpose: base_core::spec::types::RegisterPurpose::Control,
                reset_value: None, observed_values: vec![], bitfields: vec![],
                polling: false, count: 0,
            }],
            protocol: base_core::spec::types::Protocol {
                states: vec![], transitions: vec![],
                entry_condition: None, exit_condition: None,
            },
            timing: base_core::spec::types::TimingProfile {
                activation: None, processing: None, interrupt_response: None,
                dma_setup: None, polling_interval: None,
            },
            dma: None, dependencies: vec![], confidence: 0.8,
        });
        let devices = legacy_bridge::from_legacy(&spec);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "gpu_0");
    }
}
