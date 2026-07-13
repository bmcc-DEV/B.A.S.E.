/// Gerador de Device Tree Linux (.dts) a partir de HardwareSpec
pub struct DevicetreeGenerator;

impl DevicetreeGenerator {
    pub fn generate(spec: &base_core::spec::types::SynthesizedSpec) -> String {
        let mut dts = String::new();
        dts.push_str("// B.A.S.E. Generated Device Tree for Linux\n");
        dts.push_str("// Source: behavioral analysis of LK bootloader\n\n");
        dts.push_str("/dts-v1/;\n\n");
        dts.push_str("/ {\n");
        dts.push_str("    model = \"B.A.S.E. — Behavioral ASIC Reconstruction\";\n");
        dts.push_str("    compatible = \"base,synthetic-platform\";\n");
        dts.push_str("    #address-cells = <2>;\n");
        dts.push_str("    #size-cells = <2>;\n\n");

        dts.push_str("    chosen {\n");
        dts.push_str("        bootargs = \"console=ttyS0,115200 earlycon root=/dev/mmcblk0\";\n");
        dts.push_str("    };\n\n");

        // CPU
        dts.push_str(&format!(
            "    cpu @0 {{\n        compatible = \"arm,cortex-a55\";\n        reg = <0x0 0x0>;\n    }};\n\n",
        ));

        // Memory
        dts.push_str("    memory @ 0x80000000 {\n");
        dts.push_str("        device_type = \"memory\";\n");
        dts.push_str("        reg = <0x0 0x80000000 0x0 0x80000000>;\n");
        dts.push_str("    };\n\n");

        // SOC peripherals
        dts.push_str("    soc {\n");
        dts.push_str("        compatible = \"simple-bus\";\n");
        dts.push_str("        #address-cells = <2>;\n");
        dts.push_str("        #size-cells = <2>;\n");
        dts.push_str("        ranges;\n\n");

        for block in &spec.original.blocks {
            let compat = match block.kind {
                base_core::spec::types::BlockKind::Uart => "ns16550a",
                base_core::spec::types::BlockKind::Gpu => "simple-framebuffer",
                base_core::spec::types::BlockKind::Dma => "arm,pl330",
                base_core::spec::types::BlockKind::Spi => "arm,pl022",
                base_core::spec::types::BlockKind::I2c => "arm,pl011",
                base_core::spec::types::BlockKind::Timer => "arm,armv7-timer",
                base_core::spec::types::BlockKind::Ethernet => "davicom,dm9000",
                base_core::spec::types::BlockKind::Usb => "generic-ehci",
                _ => "simple-bus",
            };

            dts.push_str(&format!(
                "        {} @ 0x{:08x} {{\n",
                block.id, block.base_address
            ));
            dts.push_str(&format!(
                "            compatible = \"{}\";\n", compat
            ));
            dts.push_str(&format!(
                "            reg = <0x0 0x{:08x} 0x0 0x{:x}>;\n",
                block.base_address, block.size
            ));

            // Interrupts
            for irq in &spec.original.interrupts {
                if irq.owner == block.id {
                    dts.push_str(&format!(
                        "            interrupts = <0x{:x} 0x{:x}>;\n",
                        irq.vector, match irq.irq_type {
                            base_core::spec::types::IrqType::Level => 4u8,
                            base_core::spec::types::IrqType::Edge => 1u8,
                        }
                    ));
                }
            }

            // Registers as sub-nodes for debugging
            for reg in &block.registers {
                if let Some(ref name) = reg.name {
                    dts.push_str(&format!(
                        "            {} = <0x{:08x}>; // +0x{:x}\n",
                        name, reg.reset_value.unwrap_or(0), reg.offset
                    ));
                }
            }

            dts.push_str("        };\n\n");
        }

        dts.push_str("    };\n");
        dts.push_str("};\n");
        dts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;

    fn mock_spec() -> SynthesizedSpec {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(FunctionalBlock {
            id: "uart0".into(), kind: BlockKind::Uart,
            base_address: 0xA9BF0000, size: 0x1000, registers: vec![], protocol: Protocol { states: vec![], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: TimingProfile { activation: None, processing: None, interrupt_response: None, dma_setup: None, polling_interval: None },
            dma: None, dependencies: vec![], confidence: 0.8,
        });
        spec.interrupts.push(InterruptSpec {
            vector: 32, owner: "uart0".into(), irq_type: IrqType::Level, polarity: IrqPolarity::High,
        });
        SynthesizedSpec { original: spec, assignments: vec![], netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        }
    }

    #[test]
    fn test_dts_generation() {
        let dts = DevicetreeGenerator::generate(&mock_spec());
        assert!(dts.contains("/dts-v1/"));
        assert!(dts.contains("uart0"));
        assert!(dts.contains("0xa9bf0000"));
        assert!(dts.contains("interrupts"));
    }

    #[test]
    fn test_dts_empty() {
        let spec = SynthesizedSpec {
            original: HardwareSpec::empty(), assignments: vec![], netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        };
        let dts = DevicetreeGenerator::generate(&spec);
        assert!(dts.contains("/dts-v1/"));
    }
}
