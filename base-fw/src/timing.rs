use base_core::spec::types::SynthesizedSpec;

/// Gerador de compensação de timing (soft busy-wait, sem SDK).
pub struct TimingCompensation;

impl TimingCompensation {
    pub fn generate(&self, spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("/* B.A.S.E. Generated Timing Compensation */\n");
        code.push_str("#include <stdint.h>\n");
        code.push_str("#include <stdbool.h>\n");
        code.push_str("#include <stddef.h>\n\n");

        code.push_str("static inline void delay_us(uint32_t us) {\n");
        code.push_str("    /* Calibrated busy-wait (approx on host; replace on silicon) */\n");
        code.push_str("    volatile uint32_t cycles = us * 10u;\n");
        code.push_str("    while (cycles--) { }\n");
        code.push_str("}\n\n");

        code.push_str("void timing_compensate(const char *block_id, uint32_t operation) {\n");
        code.push_str("    (void)block_id;\n");
        code.push_str("    (void)operation;\n");

        for block in &spec.original.blocks {
            if let Some(ref proc) = block.timing.processing {
                let delay = proc.avg_ns / 1000;
                code.push_str(&format!(
                    "    /* {}: original avg latency = {}ns */\n",
                    block.id, proc.avg_ns
                ));
                if delay > 0 {
                    code.push_str(&format!("    delay_us({});\n", delay));
                }
            }
        }

        code.push_str("}\n\n");

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

        code.push_str("    { NULL, 0, 0, 0 }\n");
        code.push_str("};\n");
        code.push_str("const void *timing_table(void) { return original_timing; }\n");
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
            protocol: Protocol {
                states: vec![],
                transitions: vec![],
                entry_condition: None,
                exit_condition: None,
            },
            timing: TimingProfile {
                activation: Some(LatencyRange::new(10, 100, 50)),
                processing: Some(LatencyRange::new(1000, 5000, 2000)),
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
    fn test_timing_generation() {
        let code = TimingCompensation.generate(&mock_spec());
        assert!(code.contains("delay_us"));
        assert!(!code.contains("hardware/timer.h"));
        assert!(!code.contains("busy_wait_us_32"));
    }
}
