use base_core::spec::types::{FunctionalBlock, HardwareSpec, LatencyRange, SynthesizedSpec};

/// Gerador de compensação de timing
pub struct TimingCompensation;

impl TimingCompensation {
    /// Gera código C de delays adaptativos
    pub fn generate(&self, spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("/* B.A.S.E. Generated Timing Compensation */\n");
        code.push_str("#include <stdint.h>\n");
        code.push_str("#include <stdbool.h>\n");
        code.push_str("#include \"hardware/timer.h\"\n\n");

        code.push_str("// Timing compensation delays\n");
        code.push_str("// Injects artificial delays where new HW is faster than original\n\n");

        code.push_str("static inline void delay_us(uint32_t us) {\n");
        code.push_str("    busy_wait_us_32(us);\n");
        code.push_str("}\n\n");

        code.push_str("// Per-block timing compensation (if applicable)\n");
        code.push_str("void timing_compensate(const char *block_id, uint32_t operation) {\n");
        code.push_str("    (void)block_id;\n");
        code.push_str("    (void)operation;\n\n");

        for block in &spec.original.blocks {
            if let Some(ref proc) = block.timing.processing {
                let delay = proc.avg_ns / 1000; // ns → us
                code.push_str(&format!(
                    "    // {}: original avg latency = {}ns\n",
                    block.id, proc.avg_ns
                ));
                if delay > 0 {
                    code.push_str(&format!(
                        "    delay_us({});\n", delay
                    ));
                }
            }
        }

        code.push_str("}\n\n");

        // Gera tabela de latências originais para referência
        code.push_str("// Original system timing profile (for validation)\n");
        code.push_str("static const struct {\n");
        code.push_str("    const char *block;\n");
        code.push_str("    uint32_t min_ns;\n");
        code.push_str("    uint32_t max_ns;\n");
        code.push_str("    uint32_t avg_ns;\n");
        code.push_str("} original_timing[] = {\n");

        for block in &spec.original.blocks {
            if let Some(ref t) = block.timing.activation {
                code.push_str(&format!(
                    "    {{ \"{}\", {}, {}, {} }},\n",
                    block.id, t.min_ns, t.max_ns, t.avg_ns
                ));
            }
        }

        code.push_str("    { NULL, 0, 0, 0 } /* sentinel */\n");
        code.push_str("};\n");
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
            registers: vec![],
            protocol: Protocol { states: vec![], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: TimingProfile {
                activation: Some(LatencyRange::new(1000, 5000, 2000)),
                processing: Some(LatencyRange::new(2000, 10000, 4500)),
                interrupt_response: None,
                dma_setup: None,
                polling_interval: None,
            },
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
    fn test_timing_generation() {
        let gen = TimingCompensation;
        let spec = mock_spec();
        let code = gen.generate(&spec);
        assert!(code.contains("timing_compensate"), "Should have compensation function");
        assert!(code.contains("gpu_0"), "Should have gpu block timing");
        assert!(code.contains("original_timing"), "Should have timing table");
        assert!(code.contains("4500"), "Should have avg latency");
    }
}
