use base_core::spec::types::SynthesizedSpec;

/// Gerador de tabela de interrupções (soft, host-safe).
pub struct IrqGenerator;

impl IrqGenerator {
    pub fn generate(&self, spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("/* B.A.S.E. Generated IRQ Translation Table */\n");
        code.push_str("#include <stdint.h>\n");
        code.push_str("#include <stdbool.h>\n\n");

        code.push_str("static volatile uint32_t g_soft_pending;\n\n");

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
                irq.vector,
                irq.owner,
                if irq.irq_type == base_core::spec::types::IrqType::Edge {
                    "false"
                } else {
                    "true"
                },
            ));
        }

        code.push_str("    { 0, 0, (void*)0, false }\n");
        code.push_str("};\n\n");

        code.push_str("void irq_raise_soft(uint8_t vector) {\n");
        code.push_str("    g_soft_pending |= (1u << (vector & 31u));\n");
        code.push_str("}\n\n");

        code.push_str("void irq_handler(void) {\n");
        code.push_str("    uint32_t pending = g_soft_pending;\n");
        code.push_str("    for (int i = 0; irq_translation[i].owner != (void*)0; i++) {\n");
        code.push_str("        uint8_t vec = irq_translation[i].target_irq;\n");
        code.push_str("        if (pending & (1u << (vec & 31u))) {\n");
        code.push_str("            if (irq_translation[i].needs_ack) {\n");
        code.push_str("                g_soft_pending &= ~(1u << (vec & 31u));\n");
        code.push_str("            }\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");

        code.push_str("void irq_init(void) {\n");
        code.push_str("    g_soft_pending = 0;\n");
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
            vector: 16,
            owner: "gpu_0".into(),
            irq_type: IrqType::Level,
            polarity: IrqPolarity::High,
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
    fn test_irq_generation() {
        let code = IrqGenerator.generate(&mock_spec());
        assert!(code.contains("irq_handler"));
        assert!(code.contains("irq_init"));
        assert!(!code.contains("NVIC_"));
        assert!(!code.contains("__get_IPSR"));
    }
}
