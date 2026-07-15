use base_core::spec::types::SynthesizedSpec;

/// Gera o código do bootloader em C (self-contained, sem SDK externo).
pub struct BootloaderGenerator;

impl BootloaderGenerator {
    pub fn generate(&self, spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("/* B.A.S.E. Generated Bootloader — freestanding C */\n");
        code.push_str("#include <stdint.h>\n");
        code.push_str("#include <stddef.h>\n");
        code.push_str("#include <stdbool.h>\n\n");

        code.push_str("/* Soft platform layer (no Pico SDK dependency) */\n");
        code.push_str("static volatile uint32_t g_clk_hz = 150000000u;\n");
        code.push_str("static uint8_t g_sram_shadow[4096];\n\n");

        code.push_str("static void system_clock_init(void);\n");
        code.push_str("static void dram_init(void);\n");
        code.push_str("static void mmu_init(void);\n");
        code.push_str("static void load_firmware(void);\n");
        code.push_str("static void jump_to_entry(void);\n");
        code.push_str("bool mem_test(void);\n\n");

        code.push_str(&self.generate_main());
        code.push_str(&self.generate_clock_init(spec));
        code.push_str(&self.generate_dram_init(spec));
        code.push_str(&self.generate_mmu_init(spec));
        code.push_str(&self.generate_mmio_table(spec));
        code.push_str(&self.generate_firmware_loader());
        code.push_str(&self.generate_mem_test());

        code
    }

    fn generate_main(&self) -> String {
        let mut code = String::new();
        code.push_str("void bootloader_main(void) {\n");
        code.push_str("    system_clock_init();\n");
        code.push_str("    dram_init();\n");
        code.push_str("    mmu_init();\n");
        code.push_str("    load_firmware();\n");
        code.push_str("    jump_to_entry();\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_clock_init(&self, spec: &SynthesizedSpec) -> String {
        let clock = spec.original.cpu.clock_mhz;
        let mut code = String::new();
        code.push_str("static void system_clock_init(void) {\n");
        code.push_str(&format!(
            "    /* Original design clock: {} MHz; target soft clock 150 MHz */\n",
            clock
        ));
        code.push_str("    g_clk_hz = 150000000u;\n");
        if clock > 0 {
            code.push_str(&format!(
                "    (void){}; /* retained for documentation */\n",
                clock
            ));
        }
        code.push_str("}\n\n");
        code
    }

    fn generate_dram_init(&self, spec: &SynthesizedSpec) -> String {
        let mem = &spec.original.memory;
        let size_mb = mem
            .regions
            .first()
            .map(|r| r.size / (1024 * 1024))
            .unwrap_or(0);
        let base = mem
            .regions
            .first()
            .map(|r| format!("{:08x}", r.base))
            .unwrap_or_else(|| "00000000".into());
        let mut code = String::new();
        code.push_str("static void dram_init(void) {\n");
        code.push_str(&format!(
            "    /* Target memory region: {} MB @ 0x{} */\n",
            size_mb, base
        ));
        code.push_str("    for (size_t i = 0; i < sizeof(g_sram_shadow); i++) {\n");
        code.push_str("        g_sram_shadow[i] = 0;\n");
        code.push_str("    }\n");
        code.push_str("    if (!mem_test()) {\n");
        code.push_str("        for (;;) { /* hang on memory failure */ }\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_mmu_init(&self, _spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("static void mmu_init(void) {\n");
        code.push_str("    /* Soft MMU: identity map recorded in mmio_map[] for HAL */\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_mmio_table(&self, spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("typedef struct {\n");
        code.push_str("    uint32_t original_base;\n");
        code.push_str("    uint32_t new_base;\n");
        code.push_str("    uint32_t size;\n");
        code.push_str("} mmio_map_entry_t;\n\n");
        code.push_str("static const mmio_map_entry_t mmio_map[] = {\n");

        for block in &spec.original.blocks {
            let size = if block.size == 0 { 0x1000 } else { block.size as u32 };
            code.push_str(&format!(
                "    {{ 0x{:08x}, 0x{:08x}, 0x{:04x} }}, /* {} */\n",
                block.base_address as u32,
                (block.base_address as u32).wrapping_add(0x1000),
                size,
                block.id,
            ));
        }

        code.push_str("    { 0, 0, 0 }\n");
        code.push_str("};\n\n");
        code.push_str("const mmio_map_entry_t *bootloader_mmio_map(void) { return mmio_map; }\n\n");
        code
    }

    fn generate_firmware_loader(&self) -> String {
        let mut code = String::new();
        code.push_str("/* Weak symbols — override when linking real payload */\n");
        code.push_str("__attribute__((weak)) uint8_t _binary_firmware_bin_start;\n");
        code.push_str("__attribute__((weak)) uint8_t _binary_firmware_bin_end;\n");
        code.push_str("static uint8_t _fw_load_buf[256];\n");
        code.push_str("uint8_t *_fw_load_addr = _fw_load_buf;\n\n");

        code.push_str("static void load_firmware(void) {\n");
        code.push_str("    uintptr_t start = (uintptr_t)&_binary_firmware_bin_start;\n");
        code.push_str("    uintptr_t end = (uintptr_t)&_binary_firmware_bin_end;\n");
        code.push_str("    if (end <= start) {\n");
        code.push_str("        /* No embedded payload — keep shadow buffer as stub image */\n");
        code.push_str("        _fw_load_buf[0] = 0x00;\n");
        code.push_str("        return;\n");
        code.push_str("    }\n");
        code.push_str("    size_t fw_size = (size_t)(end - start);\n");
        code.push_str("    if (fw_size > sizeof(_fw_load_buf)) fw_size = sizeof(_fw_load_buf);\n");
        code.push_str("    const uint8_t *src = (const uint8_t *)start;\n");
        code.push_str("    for (size_t i = 0; i < fw_size; i++) {\n");
        code.push_str("        _fw_load_buf[i] = src[i];\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");

        code.push_str("static void jump_to_entry(void) {\n");
        code.push_str("#ifdef HOST_BUILD\n");
        code.push_str("    /* Host smoke test: do not jump */\n");
        code.push_str("    (void)_fw_load_addr;\n");
        code.push_str("#else\n");
        code.push_str("    typedef void (*entry_t)(void);\n");
        code.push_str("    entry_t entry = (entry_t)(uintptr_t)_fw_load_addr;\n");
        code.push_str("    if (_fw_load_buf[0] != 0) {\n");
        code.push_str("        entry();\n");
        code.push_str("    }\n");
        code.push_str("#endif\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_mem_test(&self) -> String {
        let mut code = String::new();
        code.push_str("bool mem_test(void) {\n");
        code.push_str("    for (size_t i = 0; i < sizeof(g_sram_shadow); i++) {\n");
        code.push_str("        g_sram_shadow[i] = (uint8_t)(i & 0xFFu);\n");
        code.push_str("    }\n");
        code.push_str("    for (size_t i = 0; i < sizeof(g_sram_shadow); i++) {\n");
        code.push_str("        if (g_sram_shadow[i] != (uint8_t)(i & 0xFFu)) return false;\n");
        code.push_str("    }\n");
        code.push_str("    return true;\n");
        code.push_str("}\n");
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;

    fn mock_spec() -> SynthesizedSpec {
        SynthesizedSpec {
            original: HardwareSpec::empty(),
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
    fn test_bootloader_generation() {
        let gen = BootloaderGenerator;
        let spec = mock_spec();
        let code = gen.generate(&spec);
        assert!(code.contains("bootloader_main"));
        assert!(code.contains("system_clock_init"));
        assert!(code.contains("mem_test"));
        assert!(!code.contains("pll_init("));
        assert!(!code.contains("psram_init("));
    }
}
