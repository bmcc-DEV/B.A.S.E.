use base_core::spec::types::{SynthesizedSpec};

/// Gerador de tabela de interrupções
pub struct IrqGenerator;

impl IrqGenerator {
    /// Gera irq_table.c com mapeamento de interrupções
    pub fn generate(&self, spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("/* B.A.S.E. Generated IRQ Translation Table */\n");
        code.push_str("#include <stdint.h>\n");
        code.push_str("#include <stdbool.h>\n\n");

        code.push_str("// Original → Target IRQ mapping\n");
        code.push_str("static const struct {\n");
        code.push_str("    uint8_t original_vector;\n");
        code.push_str("    uint8_t target_irq;\n");
        code.push_str("    const char *owner;\n");
        code.push_str("    bool needs_ack;\n");
        code.push_str("} irq_translation[] = {\n");

        for irq in &spec.original.interrupts {
            code.push_str(&format!(
                "    {{ {}, {}, \"{}\", {} }},\n",
                irq.vector,
                irq.vector, // 1:1 mapping by default
                irq.owner,
                if irq.irq_type == base_core::spec::types::IrqType::Edge { "false" } else { "true" },
            ));
        }

        code.push_str("    { 0, 0, NULL, false } /* sentinel */\n");
        code.push_str("};\n\n");

        code.push_str("// Interrupt handler dispatch\n");
        code.push_str("void __attribute__((interrupt)) irq_handler(void) {\n");
        code.push_str("    uint32_t pending = __get_IPSR() & 0xFF;\n\n");
        code.push_str("    for (int i = 0; irq_translation[i].owner != NULL; i++) {\n");
        code.push_str("        if (irq_translation[i].target_irq == pending) {\n");
        code.push_str("            // Call block-specific handler\n");
        code.push_str("            if (irq_translation[i].needs_ack) {\n");
        code.push_str("                NVIC_ClearPendingIRQ(pending);\n");
        code.push_str("            }\n");
        code.push_str("            return;\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");

        code.push_str("// Initialize all interrupts\n");
        code.push_str("void irq_init(void) {\n");
        code.push_str("    for (int i = 0; irq_translation[i].owner != NULL; i++) {\n");
        code.push_str("        NVIC_SetPriority(irq_translation[i].target_irq, 1);\n");
        code.push_str("        NVIC_EnableIRQ(irq_translation[i].target_irq);\n");
        code.push_str("    }\n");
        code.push_str("}\n");
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;

    fn mock_spec() -> SynthesizedSpec {
        let mut spec = HardwareSpec::empty();
        spec.interrupts.push(InterruptSpec {
            vector: 16, owner: "gpu_0".into(),
            irq_type: IrqType::Level, polarity: IrqPolarity::High,
        });
        spec.interrupts.push(InterruptSpec {
            vector: 32, owner: "audio_0".into(),
            irq_type: IrqType::Edge, polarity: IrqPolarity::High,
        });
        SynthesizedSpec {
            original: spec,
            assignments: vec![],
            netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        }
    }

    #[test]
    fn test_irq_generation() {
        let gen = IrqGenerator;
        let spec = mock_spec();
        let code = gen.generate(&spec);
        assert!(code.contains("irq_translation"), "Should have translation table");
        assert!(code.contains("irq_handler"), "Should have handler");
        assert!(code.contains("irq_init"), "Should have init");
        assert!(code.contains("gpu_0"), "Should have GPU IRQ");
        assert!(code.contains("audio_0"), "Should have Audio IRQ");
    }
}
