/// Gerador de HAL com MMU page table (O(1) em vez de O(n) lookup)
pub struct MmuHalGenerator;

impl MmuHalGenerator {
    pub fn generate(spec: &base_core::spec::types::SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("/* B.A.S.E. Generated MMU-based HAL — O(1) translation */\n");
        code.push_str("#include <stdint.h>\n");
        code.push_str("#include <stdbool.h>\n\n");

        code.push_str("// MMU page table — 4K pages, 1:1 or translated mapping\n");
        code.push_str("// Generated from behavioral analysis of original firmware\n\n");

        code.push_str("#define MMU_PAGE_SHIFT 12\n");
        code.push_str("#define MMU_PAGE_SIZE  (1 << MMU_PAGE_SHIFT)\n");
        code.push_str("#define MMU_TABLE_SIZE (256 * 1024)  // L2 page table\n\n");

        code.push_str("// Page table entry\n");
        code.push_str("// bit[0]: valid, bit[1]: device memory, bits[31:12]: output address\n");
        code.push_str("static uint32_t mmu_l2_table[MMU_TABLE_SIZE / 4] __attribute__((section(\".mmu_table\")));\n\n");

        code.push_str("static void mmu_init_translation(void) {\n");
        code.push_str("    // Identity map first 16MB (flash + SRAM regions)\n");
        code.push_str("    for (uint32_t i = 0; i < 16 * 1024 * 1024 / MMU_PAGE_SIZE; i++) {\n");
        code.push_str("        mmu_l2_table[i] = (i << MMU_PAGE_SHIFT) | 1;  // valid, same address\n");
        code.push_str("    }\n\n");

        // Generate MMU entries for each block
        for block in &spec.original.blocks {
            let page = block.base_address >> 12;
            let new_base = 0x20000000 + (page * 0x1000);
            code.push_str(&format!(
                "    // {} @ 0x{:08x} → 0x{:08x}\n",
                block.id, block.base_address, new_base
            ));
            code.push_str(&format!(
                "    mmu_l2_table[{}] = 0x{:08x} | 3;  // valid + device\n",
                page, new_base
            ));
        }

        code.push_str("}\n\n");

        // Direct access macros (O(1))
        code.push_str("// O(1) MMIO access — no table lookup at runtime\n");
        code.push_str("// The MMU handles translation in hardware\n");
        code.push_str("#define MMIO_READ(addr)  (*(volatile uint32_t *)(addr))\n");
        code.push_str("#define MMIO_WRITE(addr, val) ((*(volatile uint32_t *)(addr)) = (val))\n\n");

        code.push_str("// Init HAL — call once at boot\n");
        code.push_str("void hal_init(void) {\n");
        code.push_str("    mmu_init_translation();\n");
        code.push_str("}\n");

        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;

    fn mock_spec() -> SynthesizedSpec {
        let mut spec = base_core::spec::types::HardwareSpec::empty();
        spec.blocks.push(FunctionalBlock {
            id: "uart_0".into(), kind: BlockKind::Uart,
            base_address: 0xA9BF0000, size: 0x1000,
            registers: vec![], protocol: Protocol { states: vec![], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: TimingProfile { activation: None, processing: None, interrupt_response: None, dma_setup: None, polling_interval: None },
            dma: None, dependencies: vec![], confidence: 0.8,
        });
        SynthesizedSpec {
            original: spec, assignments: vec![], netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        }
    }

    #[test]
    fn test_mmu_hal_generation() {
        let code = MmuHalGenerator::generate(&mock_spec());
        assert!(code.contains("mmu_init_translation"));
        assert!(code.contains("MMIO_READ"));
        assert!(code.contains("MMIO_WRITE"));
        assert!(code.contains("0xa9bf0000"));
        assert!(code.contains("hal_init"));
    }
}
