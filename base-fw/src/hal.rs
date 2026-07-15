use base_core::spec::types::{FunctionalBlock, HardwareSpec, SynthesizedSpec};

/// Gera HAL de tradução MMIO entre hardware original e novo (self-contained).
pub struct HalGenerator;

impl HalGenerator {
    pub fn generate(&self, spec: &SynthesizedSpec, target: &str) -> String {
        let mut code = String::new();
        code.push_str("/* B.A.S.E. Generated HAL — MMIO Translation Layer */\n");
        code.push_str("/* Target: ");
        code.push_str(target);
        code.push_str(" */\n\n");
        code.push_str("#include <stdint.h>\n");
        code.push_str("#include <stdbool.h>\n");
        code.push_str("#include <stddef.h>\n\n");

        code.push_str(&self.generate_preamble());
        code.push_str(&self.generate_mmio_table(&spec.original));
        code.push_str(&self.generate_stubs());
        code.push_str(&self.generate_read_handler());
        code.push_str(&self.generate_write_handler(&spec.original));

        code
    }

    fn generate_preamble(&self) -> String {
        let mut code = String::new();
        code.push_str("#ifdef HOST_BUILD\n");
        code.push_str("/* Host smoke: never dereference fabricated MMIO addresses */\n");
        code.push_str("static uint32_t g_shadow_regs[1024];\n");
        code.push_str("#define REG32(addr) (g_shadow_regs[((uintptr_t)(addr) >> 2) & 1023u])\n");
        code.push_str("#else\n");
        code.push_str("#define REG32(addr) (*(volatile uint32_t *)(uintptr_t)(addr))\n");
        code.push_str("#endif\n\n");
        code.push_str("#define MMIO_TRAP_SIZE 256\n");
        code.push_str("static uint32_t g_trap_shadow[MMIO_TRAP_SIZE];\n");
        code.push_str("static uint32_t g_pio_shadow[64];\n\n");
        code
    }

    fn generate_mmio_table(&self, spec: &HardwareSpec) -> String {
        let mut code = String::new();
        code.push_str("static const struct {\n");
        code.push_str("    uint32_t base;\n");
        code.push_str("    uint32_t size;\n");
        code.push_str("    uint32_t target_base;\n");
        code.push_str("    uint8_t strategy; /* 0=MMU, 1=TRAP, 2=PIO */\n");
        code.push_str("} mmio_translation[] = {\n");

        for block in &spec.blocks {
            let strategy = match block.kind {
                base_core::spec::types::BlockKind::Dma => 0,
                base_core::spec::types::BlockKind::Gpu => 1,
                base_core::spec::types::BlockKind::Audio => 2,
                _ => 1,
            };
            let size = if block.size == 0 { 0x1000 } else { block.size as u32 };
            code.push_str(&format!(
                "    {{ 0x{:08x}, 0x{:04x}, 0x{:08x}, {} }}, /* {} */\n",
                block.base_address as u32,
                size,
                (block.base_address as u32).wrapping_add(0x100000),
                strategy,
                block.id,
            ));
        }
        code.push_str("    { 0, 0, 0, 0 }\n");
        code.push_str("};\n\n");
        code
    }

    fn generate_stubs(&self) -> String {
        let mut code = String::new();
        code.push_str("static uint32_t handle_mmio_trap_read(uint32_t target, uint32_t offset) {\n");
        code.push_str("    (void)target;\n");
        code.push_str("    return g_trap_shadow[offset % MMIO_TRAP_SIZE];\n");
        code.push_str("}\n\n");
        code.push_str("static void handle_mmio_trap_write(uint32_t target, uint32_t offset, uint32_t value) {\n");
        code.push_str("    (void)target;\n");
        code.push_str("    g_trap_shadow[offset % MMIO_TRAP_SIZE] = value;\n");
        code.push_str("}\n\n");
        code.push_str("static uint32_t pio_emulation_read(uint32_t target) {\n");
        code.push_str("    return g_pio_shadow[(target >> 2) & 63u];\n");
        code.push_str("}\n\n");
        code.push_str("static void pio_emulation_write(uint32_t target, uint32_t value) {\n");
        code.push_str("    g_pio_shadow[(target >> 2) & 63u] = value;\n");
        code.push_str("}\n\n");
        code.push_str("static void gpu_reg_write(uint32_t offset, uint32_t value) {\n");
        code.push_str("    g_trap_shadow[offset % MMIO_TRAP_SIZE] = value;\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_read_handler(&self) -> String {
        let mut code = String::new();
        code.push_str("uint32_t mmio_read(uint32_t addr) {\n");
        code.push_str("    for (int i = 0; mmio_translation[i].base != 0 || mmio_translation[i].size != 0; i++) {\n");
        code.push_str("        if (mmio_translation[i].size == 0) break;\n");
        code.push_str("        if (addr >= mmio_translation[i].base &&\n");
        code.push_str("            addr < mmio_translation[i].base + mmio_translation[i].size) {\n");
        code.push_str("            uint32_t offset = addr - mmio_translation[i].base;\n");
        code.push_str("            uint32_t target = mmio_translation[i].target_base + offset;\n");
        code.push_str("            switch (mmio_translation[i].strategy) {\n");
        code.push_str("                case 0: return REG32(target);\n");
        code.push_str("                case 1: return handle_mmio_trap_read(target, offset);\n");
        code.push_str("                case 2: return pio_emulation_read(target);\n");
        code.push_str("                default: return 0;\n");
        code.push_str("            }\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("    return 0;\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_write_handler(&self, spec: &HardwareSpec) -> String {
        let mut code = String::new();
        code.push_str("void mmio_write(uint32_t addr, uint32_t value) {\n");
        code.push_str("    for (int i = 0; mmio_translation[i].base != 0 || mmio_translation[i].size != 0; i++) {\n");
        code.push_str("        if (mmio_translation[i].size == 0) break;\n");
        code.push_str("        if (addr >= mmio_translation[i].base &&\n");
        code.push_str("            addr < mmio_translation[i].base + mmio_translation[i].size) {\n");
        code.push_str("            uint32_t offset = addr - mmio_translation[i].base;\n");
        code.push_str("            uint32_t target = mmio_translation[i].target_base + offset;\n");
        code.push_str("            switch (mmio_translation[i].strategy) {\n");
        code.push_str("                case 0: REG32(target) = value; return;\n");
        code.push_str("                case 1: handle_mmio_trap_write(target, offset, value); return;\n");
        code.push_str("                case 2: pio_emulation_write(target, value); return;\n");
        code.push_str("            }\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");

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
            "/* GPU Handler ({}) @ 0x{:08x} */\n",
            block.id, block.base_address
        ));
        code.push_str("static void handle_gpu_write(uint32_t offset, uint32_t value) {\n");
        code.push_str("    switch (offset) {\n");
        for reg in &block.registers {
            let name = reg.name.as_deref().unwrap_or("unknown");
            code.push_str(&format!("        case 0x{:04x}: /* {} */\n", reg.offset, name));
            code.push_str("            gpu_reg_write(offset, value);\n");
            code.push_str("            break;\n");
        }
        code.push_str("        default: gpu_reg_write(offset, value); break;\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");
        code.push_str("void gpu_dispatch_write(uint32_t offset, uint32_t value) {\n");
        code.push_str("    handle_gpu_write(offset, value);\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_audio_handler(&self, block: &FunctionalBlock) -> String {
        let mut code = String::new();
        code.push_str(&format!("/* Audio Handler ({}) — soft FIFO */\n", block.id));
        code.push_str("static void handle_audio_write(uint32_t offset, uint32_t value) {\n");
        code.push_str("    pio_emulation_write(offset, value);\n");
        code.push_str("}\n\n");
        code.push_str("void audio_dispatch_write(uint32_t offset, uint32_t value) {\n");
        code.push_str("    handle_audio_write(offset, value);\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_dma_handler(&self, block: &FunctionalBlock) -> String {
        let mut code = String::new();
        code.push_str(&format!("/* DMA Handler ({}) — soft descriptor */\n", block.id));
        code.push_str("static volatile uint32_t dma_soft_src;\n");
        code.push_str("static volatile uint32_t dma_soft_dst;\n");
        code.push_str("static volatile uint32_t dma_soft_count;\n");
        code.push_str("static void handle_dma_write(uint32_t offset, uint32_t value) {\n");
        code.push_str("    switch (offset & 0xFu) {\n");
        code.push_str("        case 0x0: dma_soft_src = value; break;\n");
        code.push_str("        case 0x4: dma_soft_dst = value; break;\n");
        code.push_str("        case 0x8: dma_soft_count = value; break;\n");
        code.push_str("        default: g_trap_shadow[offset % MMIO_TRAP_SIZE] = value; break;\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");
        code.push_str("void dma_dispatch_write(uint32_t offset, uint32_t value) {\n");
        code.push_str("    handle_dma_write(offset, value);\n");
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
            registers: vec![Register {
                offset: 0,
                name: Some("control".into()),
                width: 32,
                access: AccessType::ReadWrite,
                purpose: RegisterPurpose::Control,
                reset_value: None,
                observed_values: vec![],
                bitfields: vec![],
                polling: false,
                count: 1,
            }],
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
            confidence: 0.8,
        });
        SynthesizedSpec {
            original: spec,
            assignments: vec![],
            netlist: None,
            constraints: SynthesisConstraints {
                max_bom_cost: None,
                preferred_manufacturer: None,
                preferred_package: None,
            },
        }
    }

    #[test]
    fn test_hal_generation() {
        let gen = HalGenerator;
        let spec = mock_spec();
        let code = gen.generate(&spec, "rp2350");
        assert!(code.contains("mmio_read"));
        assert!(code.contains("mmio_write"));
        assert!(code.contains("handle_mmio_trap_read"));
        assert!(code.contains("gpu_0"));
        assert!(!code.contains("pio_sm_put_blocking"));
        assert!(!code.contains("dma_channel_configure"));
    }
}
