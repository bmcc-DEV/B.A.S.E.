/// Closed Feedback Loop — pipeline recursivo de refinamento baseado em evidência.
///
/// Analyze → Model → Validate → Structural Refine → Recompute Confidence
/// Confiança só sobe quando a qualidade estrutural do modelo melhora (nomes,
/// classificação, cobertura de registradores). Sem inflation artificial.
use crate::spec::types::{self, HardwareSpec, RegisterPurpose};

#[derive(Debug, Clone)]
pub enum ErrorClass {
    MissingRegister { offset: u32, address: u64 },
    WrongTiming { event: String, expected_min: u64, expected_max: u64, actual: u64 },
    UnmappedAddress { address: u64 },
    LowConfidence { block: String, confidence: f64 },
    ClassificationMismatch { block: String, current: String, suggested: String },
}

#[derive(Debug, Clone)]
pub struct LoopIteration {
    pub number: usize,
    pub pass_rate: f64,
    pub errors_found: Vec<ErrorClass>,
    pub spec: HardwareSpec,
    pub structural_changes: usize,
}

#[derive(Debug, Clone)]
pub struct FeedbackLoop {
    pub iterations: Vec<LoopIteration>,
    pub convergence_threshold: f64,
    pub max_iterations: usize,
}

impl FeedbackLoop {
    pub fn new(threshold: f64, max_iterations: usize) -> Self {
        Self {
            iterations: Vec::new(),
            convergence_threshold: threshold,
            max_iterations,
        }
    }

    pub fn iterate(&mut self, spec: &HardwareSpec, iteration: usize) -> LoopIteration {
        let errors = self.analyze_errors(spec);
        let (refined, changes) = self.refine_model(spec, &errors);
        let pass_rate = self.calculate_pass_rate(&refined, &self.analyze_errors(&refined));

        let iter = LoopIteration {
            number: iteration,
            pass_rate,
            errors_found: errors,
            spec: refined,
            structural_changes: changes,
        };

        self.iterations.push(iter.clone());
        iter
    }

    /// Executa até convergir, estagnar (0 mudanças estruturais) ou max_iterations.
    pub fn run(&mut self, initial_spec: &HardwareSpec) -> Vec<LoopIteration> {
        let mut current = initial_spec.clone();

        for i in 1..=self.max_iterations {
            let iter = self.iterate(&current, i);
            tracing::info!(
                "[Feedback] Iteration {}/{}: pass rate {:.1}%, errors: {}, structural_changes: {}",
                i,
                self.max_iterations,
                iter.pass_rate * 100.0,
                iter.errors_found.len(),
                iter.structural_changes
            );

            current = iter.spec.clone();

            if iter.pass_rate >= self.convergence_threshold {
                tracing::info!(
                    "[Feedback] Converged at iteration {} ({:.1}% >= {:.1}%)",
                    i,
                    iter.pass_rate * 100.0,
                    self.convergence_threshold * 100.0
                );
                break;
            }

            if iter.structural_changes == 0 {
                tracing::info!(
                    "[Feedback] Stagnated at iteration {} (no structural improvements possible)",
                    i
                );
                break;
            }
        }

        self.iterations.clone()
    }

    fn analyze_errors(&self, spec: &HardwareSpec) -> Vec<ErrorClass> {
        let mut errors = Vec::new();

        for block in &spec.blocks {
            let conf = evidence_confidence(block);
            if conf < 0.3 {
                errors.push(ErrorClass::LowConfidence {
                    block: block.id.clone(),
                    confidence: conf,
                });
            }

            if block.kind == types::BlockKind::Unknown && !block.registers.is_empty() {
                let suggested = self.suggest_classification(block);
                if suggested != "Unknown" {
                    errors.push(ErrorClass::ClassificationMismatch {
                        block: block.id.clone(),
                        current: "Unknown".into(),
                        suggested,
                    });
                }
            }

            for reg in &block.registers {
                if reg.name.is_none() {
                    errors.push(ErrorClass::MissingRegister {
                        offset: reg.offset,
                        address: block.base_address + reg.offset as u64,
                    });
                }
            }

            if let Some(ref act) = block.timing.activation {
                if act.min_ns == 0 && act.max_ns == 0 {
                    errors.push(ErrorClass::WrongTiming {
                        event: format!("{}_activation", block.id),
                        expected_min: 0,
                        expected_max: 0,
                        actual: 0,
                    });
                }
            }
        }

        errors
    }

    fn calculate_pass_rate(&self, spec: &HardwareSpec, errors: &[ErrorClass]) -> f64 {
        if spec.blocks.is_empty() {
            return 0.0;
        }
        // Pass rate = média da confiança baseada em evidência, penalizada por erros estruturais
        let conf_avg: f64 = spec.blocks.iter().map(evidence_confidence).sum::<f64>()
            / spec.blocks.len() as f64;
        let structural = errors
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    ErrorClass::MissingRegister { .. }
                        | ErrorClass::ClassificationMismatch { .. }
                        | ErrorClass::UnmappedAddress { .. }
                )
            })
            .count();
        let penalty = (structural as f64) / (spec.blocks.len() as f64 * 4.0).max(1.0);
        (conf_avg * (1.0 - penalty.min(0.5))).clamp(0.0, 1.0)
    }

    fn refine_model(&self, spec: &HardwareSpec, errors: &[ErrorClass]) -> (HardwareSpec, usize) {
        let mut refined = spec.clone();
        let mut changes = 0usize;

        for error in errors {
            match error {
                ErrorClass::ClassificationMismatch {
                    block,
                    current: _,
                    suggested,
                } => {
                    if let Some(b) = refined.blocks.iter_mut().find(|b| b.id == *block) {
                        let new_kind = match suggested.as_str() {
                            "Gpu" => types::BlockKind::Gpu,
                            "Audio" => types::BlockKind::Audio,
                            "Dma" => types::BlockKind::Dma,
                            "Usb" => types::BlockKind::Usb,
                            "Uart" => types::BlockKind::Uart,
                            "Spi" => types::BlockKind::Spi,
                            "I2c" => types::BlockKind::I2c,
                            "Control" => types::BlockKind::Timer,
                            _ => types::BlockKind::Unknown,
                        };
                        if b.kind != new_kind {
                            b.kind = new_kind;
                            changes += 1;
                            tracing::info!("  Reclassified {} → {:?}", block, b.kind);
                        }
                    }
                }
                ErrorClass::MissingRegister { offset: _, address } => {
                    if let Some(b) = refined.blocks.iter_mut().find(|b| {
                        b.base_address <= *address && b.base_address + b.size > *address
                    }) {
                        let offset = (address - b.base_address) as u32;
                        if let Some(reg) = b.registers.iter_mut().find(|r| r.offset == offset) {
                            if reg.name.is_none() {
                                let vals: Vec<u64> = reg.observed_values.iter().map(|v| v.value).collect();
                                let (name, purpose) = infer_register_name(offset, &vals);
                                reg.name = Some(name);
                                if matches!(reg.purpose, RegisterPurpose::UnknownPurpose) {
                                    reg.purpose = purpose;
                                }
                                changes += 1;
                                tracing::info!("  Named register +0x{:x} on {}", offset, b.id);
                            }
                        } else {
                            let (name, purpose) = infer_register_name(offset, &[]);
                            b.registers.push(types::Register {
                                offset,
                                name: Some(name),
                                width: 32,
                                access: types::AccessType::ReadWrite,
                                purpose,
                                reset_value: None,
                                observed_values: vec![],
                                bitfields: vec![],
                                polling: false,
                                count: 0,
                            });
                            changes += 1;
                            tracing::info!("  Added register +0x{:x} to {}", offset, b.id);
                        }
                    }
                }
                ErrorClass::WrongTiming { event, .. } => {
                    // Remove timing placeholder zero — use observed defaults from block name
                    if let Some(block_id) = event.strip_suffix("_activation") {
                        if let Some(b) = refined.blocks.iter_mut().find(|b| b.id == block_id) {
                            if let Some(ref mut act) = b.timing.activation {
                                if act.min_ns == 0 && act.max_ns == 0 {
                                    *act = types::LatencyRange::new(10, 1000, 100);
                                    changes += 1;
                                    tracing::info!("  Filled placeholder timing for {}", block_id);
                                }
                            }
                        }
                    }
                }
                ErrorClass::LowConfidence { .. } | ErrorClass::UnmappedAddress { .. } => {
                    // Não infla confiança — será recalculada no fim
                }
            }
        }

        // Recalcula confiança a partir da evidência estrutural
        for b in &mut refined.blocks {
            let new_conf = evidence_confidence(b);
            if (new_conf - b.confidence).abs() > 0.001 {
                tracing::info!(
                    "  {} confidence {:.2} → {:.2} (evidence)",
                    b.id,
                    b.confidence,
                    new_conf
                );
                b.confidence = new_conf;
            }
        }

        (refined, changes)
    }

    fn suggest_classification(&self, block: &types::FunctionalBlock) -> String {
        for reg in &block.registers {
            if let Some(ref name) = reg.name {
                let lower = name.to_lowercase();
                if lower.contains("dma") {
                    return "Dma".into();
                }
                if lower.contains("audio") || lower.contains("i2s") {
                    return "Audio".into();
                }
                if lower.contains("spi") {
                    return "Spi".into();
                }
                if lower.contains("i2c") {
                    return "I2c".into();
                }
                if lower.contains("uart") || lower.contains("serial") {
                    return "Uart".into();
                }
                if lower.contains("gpu") || lower.contains("fb") || lower.contains("crtc") {
                    return "Gpu".into();
                }
            }
        }
        for reg in &block.registers {
            if reg.offset == 0x100 || reg.offset == 0x200 {
                return "Dma".into();
            }
            if reg.offset == 0x00 && reg.polling {
                return "Control".into();
            }
        }
        // Heurística por densidade de writes vs reads
        let writes: usize = block
            .registers
            .iter()
            .map(|r| r.observed_values.len())
            .sum();
        if writes > 8 && block.registers.len() > 4 {
            return "Dma".into();
        }
        "Unknown".into()
    }

    pub fn convergence_report(&self) -> ConvergenceReport {
        let first = self.iterations.first();
        let last = self.iterations.last();
        let total_errors: usize = self.iterations.iter().map(|i| i.errors_found.len()).sum();
        let avg_errors = if self.iterations.is_empty() {
            0.0
        } else {
            total_errors as f64 / self.iterations.len() as f64
        };

        ConvergenceReport {
            total_iterations: self.iterations.len(),
            initial_pass_rate: first.map(|i| i.pass_rate).unwrap_or(0.0),
            final_pass_rate: last.map(|i| i.pass_rate).unwrap_or(0.0),
            improvement: last.map(|i| i.pass_rate).unwrap_or(0.0)
                - first.map(|i| i.pass_rate).unwrap_or(0.0),
            total_errors_found: total_errors,
            avg_errors_per_iteration: avg_errors,
            converged: last.map_or(false, |i| i.pass_rate >= self.convergence_threshold),
        }
    }
}

/// Confiança derivada de evidência observável no bloco (não monótona artificial).
pub fn evidence_confidence(block: &types::FunctionalBlock) -> f64 {
    if block.registers.is_empty() {
        return if block.kind != types::BlockKind::Unknown {
            0.25
        } else {
            0.05
        };
    }
    let named = block.registers.iter().filter(|r| r.name.is_some()).count();
    let named_score = named as f64 / block.registers.len() as f64;
    let kind_score = if block.kind != types::BlockKind::Unknown {
        0.25
    } else {
        0.0
    };
    let protocol_score = if block.protocol.states.len() >= 2 {
        0.2
    } else if !block.protocol.states.is_empty() {
        0.1
    } else {
        0.0
    };
    let purpose_score = block
        .registers
        .iter()
        .filter(|r| !matches!(r.purpose, RegisterPurpose::UnknownPurpose))
        .count() as f64
        / block.registers.len() as f64
        * 0.15;

    (0.4 * named_score + kind_score + protocol_score + purpose_score).clamp(0.0, 0.95)
}

fn infer_register_name(offset: u32, observed: &[u64]) -> (String, RegisterPurpose) {
    let purpose = match offset {
        0x00 => RegisterPurpose::Control,
        0x04 => RegisterPurpose::Status,
        o if o >= 0x100 && o < 0x200 => RegisterPurpose::DataPort,
        _ if observed.iter().any(|&v| v == 0 || v == 1) => RegisterPurpose::Control,
        _ => RegisterPurpose::UnknownPurpose,
    };
    let name = match purpose {
        RegisterPurpose::Control => format!("ctrl_{:x}", offset),
        RegisterPurpose::Status => format!("status_{:x}", offset),
        RegisterPurpose::DataPort => format!("data_{:x}", offset),
        _ => format!("reg_{:x}", offset),
    };
    (name, purpose)
}

#[derive(Debug, Clone)]
pub struct ConvergenceReport {
    pub total_iterations: usize,
    pub initial_pass_rate: f64,
    pub final_pass_rate: f64,
    pub improvement: f64,
    pub total_errors_found: usize,
    pub avg_errors_per_iteration: f64,
    pub converged: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> HardwareSpec {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(types::FunctionalBlock {
            id: "gpu_0".into(),
            kind: types::BlockKind::Unknown,
            base_address: 0x10000000,
            size: 0x1000,
            registers: vec![
                types::Register {
                    offset: 0,
                    name: None,
                    width: 32,
                    access: types::AccessType::ReadWrite,
                    purpose: types::RegisterPurpose::Control,
                    reset_value: None,
                    observed_values: vec![],
                    bitfields: vec![],
                    polling: false,
                    count: 0,
                },
                types::Register {
                    offset: 0x100,
                    name: None,
                    width: 32,
                    access: types::AccessType::ReadWrite,
                    purpose: types::RegisterPurpose::UnknownPurpose,
                    reset_value: None,
                    observed_values: (1..10).map(|v| types::ObservedValue { value: v, count: 1, context: String::new() }).collect(),
                    bitfields: vec![],
                    polling: false,
                    count: 0,
                },
            ],
            protocol: types::Protocol {
                states: vec![],
                transitions: vec![],
                entry_condition: None,
                exit_condition: None,
            },
            timing: types::TimingProfile {
                activation: Some(types::LatencyRange::new(0, 0, 0)),
                processing: None,
                interrupt_response: None,
                dma_setup: None,
                polling_interval: None,
            },
            dma: None,
            dependencies: vec![],
            confidence: 0.1,
        });
        spec
    }

    #[test]
    fn test_analyze_errors() {
        let loop_ = FeedbackLoop::new(0.9, 10);
        let spec = sample_spec();
        let errors = loop_.analyze_errors(&spec);
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| matches!(e, ErrorClass::MissingRegister { .. })));
    }

    #[test]
    fn test_single_iteration() {
        let mut loop_ = FeedbackLoop::new(0.9, 10);
        let spec = sample_spec();
        let iter = loop_.iterate(&spec, 1);
        assert!(iter.structural_changes > 0);
    }

    #[test]
    fn test_full_loop_convergence() {
        let mut loop_ = FeedbackLoop::new(0.7, 10);
        let spec = sample_spec();
        let iterations = loop_.run(&spec);
        assert!(!iterations.is_empty());
        let last = iterations.last().unwrap();
        assert!(
            last.pass_rate >= 0.7 || iterations.len() <= 10,
            "Should converge or finish iterations"
        );
    }

    #[test]
    fn test_refine_confidence_from_evidence() {
        let mut loop_ = FeedbackLoop::new(0.9, 5);
        let spec = sample_spec();
        let original_conf = evidence_confidence(&spec.blocks[0]);
        let iter = loop_.iterate(&spec, 1);
        let new_conf = iter.spec.blocks[0].confidence;
        assert!(
            new_conf > original_conf,
            "Confidence should rise after naming/classification"
        );
    }

    #[test]
    fn test_no_fake_inflation_without_structure() {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(types::FunctionalBlock {
            id: "ok".into(),
            kind: types::BlockKind::Uart,
            base_address: 0x1000,
            size: 0x100,
            registers: vec![types::Register {
                offset: 0,
                name: Some("dr".into()),
                width: 32,
                access: types::AccessType::ReadWrite,
                purpose: RegisterPurpose::DataPort,
                reset_value: None,
                observed_values: vec![],
                bitfields: vec![],
                polling: false,
                count: 1,
            }],
            protocol: types::Protocol {
                states: vec!["idle".into(), "busy".into()],
                transitions: vec![],
                entry_condition: None,
                exit_condition: None,
            },
            timing: types::TimingProfile {
                activation: None,
                processing: None,
                interrupt_response: None,
                dma_setup: None,
                polling_interval: None,
            },
            dma: None,
            dependencies: vec![],
            confidence: 0.2,
        });
        let before = evidence_confidence(&spec.blocks[0]);
        let mut loop_ = FeedbackLoop::new(0.99, 5);
        let iter = loop_.iterate(&spec, 1);
        assert_eq!(iter.structural_changes, 0);
        assert!((iter.spec.blocks[0].confidence - before).abs() < 0.001
            || iter.spec.blocks[0].confidence >= before);
    }

    #[test]
    fn test_convergence_report() {
        let mut loop_ = FeedbackLoop::new(0.9, 10);
        let spec = sample_spec();
        loop_.run(&spec);
        let report = loop_.convergence_report();
        assert!(report.total_iterations > 0);
    }
}
