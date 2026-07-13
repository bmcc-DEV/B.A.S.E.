use base_core::spec::types::{FunctionalBlock, HardwareSpec, SynthesizedSpec};

/// Gera HAL de tradução MMIO entre hardware original e novo
pub struct HalGenerator;

impl HalGenerator {
    /// Gera arquivo hal_mmio.c com todas as traduções
    pub fn generate(&self, spec: &SynthesizedSpec, target: &str) -> String {
        let mut code = String::new();
        code.push_str("/* B.A.S.E. Generated HAL — MMIO Translation Layer */\n");
        code.push_str("/* Target: ");
        code.push_str(target);
        code.push_str(" */\n\n");
        code.push_str("#include <stdint.h>\n");
        code.push_str("#include <stdbool.h>\n\n");

        code.push_str(&self.generate_preamble());
        code.push_str(&self.generate_mmio_table(&spec.original));
        code.push_str(&self.generate_read_handler(&spec.original));
        code.push_str(&self.generate_write_handler(&spec.original));

        code
    }

    fn generate_preamble(&self) -> String {
        let mut code = String::new();
        code.push_str("// Register access macros\n");
        code.push_str("#define REG32(addr) (*(volatile uint32_t *)(addr))\n");
        code.push_str("#define REG8(addr)  (*(volatile uint8_t *)(addr))\n\n");
        code.push_str("#define MMIO_TRAP_SIZE 256\n\n");
        code
    }

    fn generate_mmio_table(&self, spec: &HardwareSpec) -> String {
        let mut code = String::new();
        code.push_str("// MMIO Translation Table\n");
        code.push_str("// Maps original addresses to new hardware\n");
        code.push_str("static const struct {\n");
        code.push_str("    uint32_t base;\n");
        code.push_str("    uint32_t size;\n");
        code.push_str("    uint32_t target_base;\n");
        code.push_str("    uint8_t strategy; // 0=MMU, 1=TRAP, 2=PIO\n");
        code.push_str("} mmio_translation[] = {\n");

        for block in &spec.blocks {
            let strategy = match block.kind {
                base_core::spec::types::BlockKind::Dma => 0,  // MMU-based
                base_core::spec::types::BlockKind::Gpu => 1,  // Trap
                base_core::spec::types::BlockKind::Audio => 2, // PIO
                _ => 1,
            };
            code.push_str(&format!(
                "    {{ 0x{:08x}, 0x{:04x}, 0x{:08x}, {} }}, // {}\n",
                block.base_address, block.size,
                block.base_address + 0x100000, // shift by 1MB for target
                strategy, block.id,
            ));
        }
        code.push_str("    { 0, 0, 0, 0 } /* sentinel */\n");
        code.push_str("};\n\n");
        code
    }

    fn generate_read_handler(&self, _spec: &HardwareSpec) -> String {
        let mut code = String::new();
        code.push_str("// MMIO Read Handler\n");
        code.push_str("// Reads from original address, translates to new hardware\n");
        code.push_str("uint32_t mmio_read(uint32_t addr) {\n");
        code.push_str("    // Search translation table\n");
        code.push_str("    for (int i = 0; mmio_translation[i].base != 0; i++) {\n");
        code.push_str("        if (addr >= mmio_translation[i].base &&\n");
        code.push_str("            addr < mmio_translation[i].base + mmio_translation[i].size) {\n");
        code.push_str("            uint32_t offset = addr - mmio_translation[i].base;\n");
        code.push_str("            uint32_t target = mmio_translation[i].target_base + offset;\n\n");
        code.push_str("            switch (mmio_translation[i].strategy) {\n");
        code.push_str("                case 0: // Direct MMU mapping\n");
        code.push_str("                    return REG32(target);\n");
        code.push_str("                case 1: // Trap handler\n");
        code.push_str("                    return handle_mmio_trap_read(target, offset);\n");
        code.push_str("                case 2: // PIO emulation\n");
        code.push_str("                    return pio_emulation_read(target);\n");
        code.push_str("                default:\n");
        code.push_str("                    return 0;\n");
        code.push_str("            }\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("    return 0;\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_write_handler(&self, spec: &HardwareSpec) -> String {
        let mut code = String::new();
        code.push_str("// MMIO Write Handler\n");
        code.push_str("void mmio_write(uint32_t addr, uint32_t value) {\n");
        code.push_str("    for (int i = 0; mmio_translation[i].base != 0; i++) {\n");
        code.push_str("        if (addr >= mmio_translation[i].base &&\n");
        code.push_str("            addr < mmio_translation[i].base + mmio_translation[i].size) {\n");
        code.push_str("            uint32_t offset = addr - mmio_translation[i].base;\n");
        code.push_str("            uint32_t target = mmio_translation[i].target_base + offset;\n\n");
        code.push_str("            switch (mmio_translation[i].strategy) {\n");
        code.push_str("                case 0:\n");
        code.push_str("                    REG32(target) = value;\n");
        code.push_str("                    return;\n");
        code.push_str("                case 1:\n");
        code.push_str("                    handle_mmio_trap_write(target, offset, value);\n");
        code.push_str("                    return;\n");
        code.push_str("                case 2:\n");
        code.push_str("                    pio_emulation_write(target, value);\n");
        code.push_str("                    return;\n");
        code.push_str("            }\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");

        // Generate trap handlers for each block
        code.push_str("// Block-specific handlers\n\n");
        for block in &spec.blocks {
            if matches!(block.kind, base_core::spec::types::BlockKind::Gpu) {
                code.push_str(&self.generate_gpu_handler(block));
            } else if matches!(block.kind, base_core::spec::types::BlockKind::Audio) {
                code.push_str(&self.generate_audio_handler(block));
            } else if matches!(block.kind, base_core::spec::types::BlockKind::Dma) {
                code.push_str(&self.generate_dma_handler(block));
            }
        }
        code
    }

    fn generate_gpu_handler(&self, block: &FunctionalBlock) -> String {
        let mut code = String::new();
        code.push_str(&format!(
            "// GPU Handler ({}) @ 0x{:08x}\n", block.id, block.base_address
        ));
        code.push_str("static void handle_gpu_write(uint32_t offset, uint32_t value) {\n");
        code.push_str("    switch (offset) {\n");

        for reg in &block.registers {
            let name = reg.name.as_deref().unwrap_or("unknown");
            code.push_str(&format!(
                "        case 0x{:04x}: /* {} */\n", reg.offset, name
            ));
            code.push_str("            gpu_reg_write(offset, value);\n");
            code.push_str("            break;\n");
        }

        code.push_str("        default:\n");
        code.push_str("            break;\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_audio_handler(&self, block: &FunctionalBlock) -> String {
        let mut code = String::new();
        code.push_str(&format!(
            "// Audio Handler ({}) — I2S via PIO\n", block.id
        ));
        code.push_str("static void handle_audio_write(uint32_t offset, uint32_t value) {\n");
        code.push_str("    // Map to I2S PIO state machine\n");
        code.push_str("    pio_sm_put_blocking(pio0, 0, value);\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_dma_handler(&self, block: &FunctionalBlock) -> String {
        let mut code = String::new();
        code.push_str(&format!(
            "// DMA Handler ({}) — RP2350 DMA channel\n", block.id
        ));
        code.push_str("static void handle_dma_write(uint32_t offset, uint32_t value) {\n");
        code.push_str("    // Map to RP2350 DMA controller\n");
        code.push_str("    dma_channel_config cfg = dma_channel_get_default_config(0);\n");
        code.push_str("    channel_config_set_transfer_data_size(&cfg, DMA_SIZE_32);\n");
        code.push_str("    dma_channel_configure(0, &cfg,\n");
        code.push_str("        dma_target,  // write address\n");
        code.push_str("        dma_source,  // read address\n");
        code.push_str("        dma_count,   // number of transfers\n");
        code.push_str("        true);       // start\n");
        code.push_str("}\n\n");
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;

    fn mock_spec() -> SynthesizedSpec {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(FunctionalBlock {
            id: "gpu_0".into(),
            kind: BlockKind::Gpu,
            base_address: 0x10000000,
            size: 0x1000,
            registers: vec![
                Register {
                    offset: 0, name: Some("control".into()), width: 32,
                    access: base_core::spec::types::AccessType::ReadWrite,
                    purpose: RegisterPurpose::Control, reset_value: None,
                    observed_values: vec![], bitfields: vec![], polling: false, count: 1,
                },
            ],
            protocol: Protocol { states: vec![], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: TimingProfile { activation: None, processing: None, interrupt_response: None, dma_setup: None, polling_interval: None },
            dma: None, dependencies: vec![], confidence: 0.8,
        });
        SynthesizedSpec {
            original: spec,
            assignments: vec![],
            netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        }
    }

    #[test]
    fn test_hal_generation() {
        let gen = HalGenerator;
        let spec = mock_spec();
        let code = gen.generate(&spec, "rp2350");
        assert!(code.contains("mmio_read"), "Should have read handler");
        assert!(code.contains("mmio_write"), "Should have write handler");
        assert!(code.contains("MMIO Translation Table"), "Should have table");
        assert!(code.contains("gpu_0"), "Should have GPU handler");
    }
}
