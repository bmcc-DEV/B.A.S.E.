use base_core::spec::types::SynthesizedSpec;

/// Gera o código do bootloader em C
pub struct BootloaderGenerator;

impl BootloaderGenerator {
    /// Gera bootloader completo para um target RP2350-like
    pub fn generate(&self, spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("/* B.A.S.E. Generated Bootloader */\n");
        code.push_str("#include <stdint.h>\n");
        code.push_str("#include <stdbool.h>\n\n");

        code.push_str("// Forward declarations\n");
        code.push_str("static void system_clock_init(void);\n");
        code.push_str("static void dram_init(void);\n");
        code.push_str("static void mmu_init(void);\n");
        code.push_str("static void load_firmware(void);\n");
        code.push_str("static void jump_to_entry(void);\n\n");

        code.push_str(&self.generate_main());
        code.push_str(&self.generate_clock_init(spec));
        code.push_str(&self.generate_dram_init(spec));
        code.push_str(&self.generate_mmu_init(spec));
        code.push_str(&self.generate_mmio_table(spec));
        code.push_str(&self.generate_firmware_loader());

        code
    }

    fn generate_main(&self) -> String {
        let mut code = String::new();
        code.push_str("void bootloader_main(void) {\n");
        code.push_str("    // Step 1: Configure system clocks\n");
        code.push_str("    system_clock_init();\n\n");
        code.push_str("    // Step 2: Initialize DRAM\n");
        code.push_str("    dram_init();\n\n");
        code.push_str("    // Step 3: Set up MMU translation tables\n");
        code.push_str("    mmu_init();\n\n");
        code.push_str("    // Step 4: Load original firmware\n");
        code.push_str("    load_firmware();\n\n");
        code.push_str("    // Step 5: Jump to entry point\n");
        code.push_str("    jump_to_entry();\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_clock_init(&self, spec: &SynthesizedSpec) -> String {
        let clock = spec.original.cpu.clock_mhz;
        let mut code = String::new();
        code.push_str("static void system_clock_init(void) {\n");
        code.push_str("    // Target: RP2350 PLL\n");
        code.push_str("    // Original clock: ");
        code.push_str(&clock.to_string());
        code.push_str(" MHz\n\n");
        code.push_str("    // Configure PLL_SYS for 150 MHz\n");
        code.push_str("    pll_init(PLL_SYS, 150 * MHZ);\n\n");
        code.push_str("    // Configure PLL_USB for 48 MHz\n");
        code.push_str("    pll_init(PLL_USB, 48 * MHZ);\n\n");
        code.push_str("    // Switch system clock to PLL_SYS\n");
        code.push_str("    clock_configure(clk_sys,\n");
        code.push_str("                    CLOCKS_CLK_SYS_CTRL_SRC_VALUE_CLKSRC_CLK_SYS_AUX,\n");
        code.push_str("                    CLOCKS_CLK_SYS_CTRL_AUXSRC_VALUE_CLKSRC_PLL_SYS);\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_dram_init(&self, spec: &SynthesizedSpec) -> String {
        let mem = &spec.original.memory;
        let size_mb = mem.regions.first().map(|r| r.size / (1024 * 1024)).unwrap_or(0);
        let mut code = String::new();
        code.push_str("static void dram_init(void) {\n");
        code.push_str("    // Target memory: ");
        code.push_str(&size_mb.to_string());
        code.push_str(" MB at 0x");
        code.push_str(&mem.regions.first().map(|r| format!("{:08x}", r.base)).unwrap_or_default());
        code.push_str("\n\n");
        code.push_str("    // Configure PSRAM via QSPI\n");
        code.push_str("    psram_init();\n\n");
        code.push_str("    // Memory test (quick verify)\n");
        code.push_str("    if (!mem_test()) {\n");
        code.push_str("        while(1); // hang on memory failure\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_mmu_init(&self, _spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("static void mmu_init(void) {\n");
        code.push_str("    // Section-based MMU translation tables\n");
        code.push_str("    // Maps original MMIO addresses to new hardware\n\n");
        code.push_str("    // Identity map first 1MB (flash + SRAM)\n");
        code.push_str("    mmu_map_page(0x00000000, 0x00000000, MMU_CACHE_WRITE_BACK);\n\n");
        code.push_str("    // Device memory (non-cacheable)\n");
        code.push_str("    mmu_map_range(PERIPHERAL_BASE, PERIPHERAL_BASE + 0x100000,\n");
        code.push_str("                  MMU_DEVICE);\n");
        code.push_str("}\n\n");
        code
    }

    fn generate_mmio_table(&self, spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("// MMIO Translation Table\n");
        code.push_str("// Original → New hardware mapping\n");
        code.push_str("static const struct {\n");
        code.push_str("    uint32_t original_base;\n");
        code.push_str("    uint32_t new_base;\n");
        code.push_str("    uint32_t size;\n");
        code.push_str("} mmio_map[] = {\n");

        for block in &spec.original.blocks {
            code.push_str(&format!(
                "    {{ 0x{:08x}, 0x{:08x}, 0x{:04x} }}, // {}\n",
                block.base_address,
                block.base_address + 0x1000, // shift by 4K for new mapping
                0x1000,
                block.id,
            ));
        }

        code.push_str("    { 0, 0, 0 } /* sentinel */\n");
        code.push_str("};\n\n");
        code
    }

    fn generate_firmware_loader(&self) -> String {
        let mut code = String::new();
        code.push_str("static void load_firmware(void) {\n");
        code.push_str("    extern uint8_t _binary_firmware_bin_start;\n");
        code.push_str("    extern uint8_t _binary_firmware_bin_end;\n");
        code.push_str("    extern uint8_t _fw_load_addr;\n\n");
        code.push_str("    uint32_t fw_size = &_binary_firmware_bin_end\n");
        code.push_str("                   - &_binary_firmware_bin_start;\n");
        code.push_str("    uint8_t *src = &_binary_firmware_bin_start;\n");
        code.push_str("    uint8_t *dst = &_fw_load_addr;\n\n");
        code.push_str("    // Copy firmware (Cortex-M33 / Thumb-2 compatible)\n");
        code.push_str("    while (fw_size--) {\n");
        code.push_str("        *dst++ = *src++;\n");
        code.push_str("    }\n");
        code.push_str("    __asm__ volatile (\"dsb\" ::: \"memory\");\n");
        code.push_str("}\n\n");
        code.push_str("static void __attribute__((noreturn)) jump_to_entry(void) {\n");
        code.push_str("    typedef void (*entry_t)(void);\n");
        code.push_str("    entry_t entry = (entry_t)(uintptr_t)&_fw_load_addr;\n");
        code.push_str("    __asm__ volatile (\"isb\" ::: \"memory\");\n");
        code.push_str("    __set_MSP(0x20040000);  // set stack pointer for firmware\n");
        code.push_str("    entry();\n");
        code.push_str("    while(1);\n");
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
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        }
    }

    #[test]
    fn test_bootloader_generation() {
        let gen = BootloaderGenerator;
        let spec = mock_spec();
        let code = gen.generate(&spec);
        assert!(code.contains("bootloader_main"), "Should have main entry");
        assert!(code.contains("system_clock_init"), "Should have clock init");
        assert!(code.contains("dram_init"), "Should have DRAM init");
        assert!(code.contains("mmu_init"), "Should have MMU init");
        assert!(code.contains("jump_to_entry"), "Should have jump");
        assert!(code.contains("MMIO Translation Table"), "Should have MMIO table");
    }
}
