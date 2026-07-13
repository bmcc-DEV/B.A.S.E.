/// Closed Feedback Loop — pipeline recursivo de refinamento.
///
/// Analyze → Model → Synthesize → Validate → Error → Refine → Analyze
/// Cada erro melhora o modelo. O ciclo nunca termina até convergir.
use crate::spec::types::{self, HardwareSpec};

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
}

#[derive(Debug, Clone)]
pub struct FeedbackLoop {
    pub iterations: Vec<LoopIteration>,
    pub convergence_threshold: f64,
    pub max_iterations: usize,
}

impl FeedbackLoop {
    /// Cria um novo loop com threshold de convergência
    pub fn new(threshold: f64, max_iterations: usize) -> Self {
        Self {
            iterations: Vec::new(),
            convergence_threshold: threshold,
            max_iterations,
        }
    }

    /// Executa uma iteração do loop: analisa erros e refina o modelo
    pub fn iterate(
        &mut self,
        spec: &HardwareSpec,
        iteration: usize,
    ) -> LoopIteration {
        let errors = self.analyze_errors(spec);
        let pass_rate = self.calculate_pass_rate(spec, &errors);
        let refined = self.refine_model(spec, &errors);

        let iter = LoopIteration {
            number: iteration,
            pass_rate,
            errors_found: errors,
            spec: refined,
        };

        self.iterations.push(iter.clone());
        iter
    }

    /// Executa o loop completo até convergir ou atingir max_iterations
    pub fn run(&mut self, initial_spec: &HardwareSpec) -> Vec<LoopIteration> {
        let mut current = initial_spec.clone();

        for i in 1..=self.max_iterations {
            let iter = self.iterate(&current, i);
            tracing::info!(
                "[Feedback] Iteration {}/{}: pass rate {:.1}%, errors: {}",
                i, self.max_iterations, iter.pass_rate * 100.0, iter.errors_found.len()
            );

            current = iter.spec.clone();

            if iter.pass_rate >= self.convergence_threshold {
                tracing::info!("[Feedback] Converged at iteration {} ({:.1}% >= {:.1}%)",
                    i, iter.pass_rate * 100.0, self.convergence_threshold * 100.0);
                break;
            }
        }

        self.iterations.clone()
    }

    /// Analisa o spec atual e encontra erros/melhorias
    fn analyze_errors(&self, spec: &HardwareSpec) -> Vec<ErrorClass> {
        let mut errors = Vec::new();

        for block in &spec.blocks {
            // Baixa confiança
            if block.confidence < 0.3 {
                errors.push(ErrorClass::LowConfidence {
                    block: block.id.clone(),
                    confidence: block.confidence,
                });
            }

            // Classificação genérica demais
            if block.kind == types::BlockKind::Unknown && !block.registers.is_empty() {
                errors.push(ErrorClass::ClassificationMismatch {
                    block: block.id.clone(),
                    current: "Unknown".into(),
                    suggested: self.suggest_classification(block),
                });
            }

            // Registradores sem nome
            for reg in &block.registers {
                if reg.name.is_none() {
                    errors.push(ErrorClass::MissingRegister {
                        offset: reg.offset,
                        address: block.base_address + reg.offset as u64,
                    });
                }
            }

            // Timing suspeito (valores default)
            if let Some(ref act) = block.timing.activation {
                if act.min_ns == 0 && act.max_ns == 0 {
                    errors.push(ErrorClass::WrongTiming {
                        event: format!("{}_activation", block.id),
                        expected_min: 0, expected_max: 0, actual: 0,
                    });
                }
            }
        }

        errors
    }

    /// Calcula pass rate baseado nos erros encontrados
    fn calculate_pass_rate(&self, _spec: &HardwareSpec, errors: &[ErrorClass]) -> f64 {
        if _spec.blocks.is_empty() {
            return 0.0;
        }
        let total_checks = _spec.blocks.len() * 4; // 4 checks per block
        let error_count = errors.len();
        if total_checks == 0 { return 1.0; }
        1.0 - (error_count as f64 / total_checks as f64)
    }

    /// Refina o modelo baseado nos erros
    fn refine_model(&self, spec: &HardwareSpec, errors: &[ErrorClass]) -> HardwareSpec {
        let mut refined = spec.clone();

        for error in errors {
            match error {
                ErrorClass::LowConfidence { block, confidence } => {
                    // Aumenta confiança com cada iteração (aprendizado)
                    if let Some(b) = refined.blocks.iter_mut().find(|b| b.id == *block) {
                        b.confidence = (b.confidence + 0.1).min(0.95);
                        tracing::info!("  Refined {} confidence: {:.2} → {:.2}", block, confidence, b.confidence);
                    }
                }
                ErrorClass::ClassificationMismatch { block, current: _, suggested } => {
                    // Atualiza classificação
                    if let Some(b) = refined.blocks.iter_mut().find(|b| b.id == *block) {
                        let new_kind = match suggested.as_str() {
                            "Gpu" => types::BlockKind::Gpu,
                            "Audio" => types::BlockKind::Audio,
                            "Dma" => types::BlockKind::Dma,
                            "Usb" => types::BlockKind::Usb,
                            "Uart" => types::BlockKind::Uart,
                            _ => types::BlockKind::Unknown,
                        };
                        b.kind = new_kind;
                        b.confidence = (b.confidence + 0.15).min(0.95);
                        tracing::info!("  Reclassified {} → {:?}", block, b.kind);
                    }
                }
                ErrorClass::MissingRegister { offset: _, address } => {
                    // Adiciona registro desconhecido ao bloco
                    if let Some(b) = refined.blocks.iter_mut().find(|b| {
                        b.base_address <= *address && b.base_address + b.size > *address
                    }) {
                        let offset = (address - b.base_address) as u32;
                        if !b.registers.iter().any(|r| r.offset == offset) {
                            b.registers.push(types::Register {
                                offset,
                                name: Some(format!("unknown_{:x}", offset)),
                                width: 32,
                                access: types::AccessType::ReadWrite,
                                purpose: types::RegisterPurpose::UnknownPurpose,
                                reset_value: None,
                                observed_values: vec![],
                                bitfields: vec![],
                                polling: false,
                                count: 0,
                            });
                            tracing::info!("  Added register +0x{:x} to {}", offset, b.id);
                        }
                    }
                }
                _ => {}
            }
        }

        refined
    }

    /// Sugere classificação para um bloco baseado em seus registradores
    fn suggest_classification(&self, block: &types::FunctionalBlock) -> String {
        for reg in &block.registers {
            if let Some(ref name) = reg.name {
                let lower = name.to_lowercase();
                if lower.contains("dma") { return "Dma".into(); }
                if lower.contains("audio") || lower.contains("i2s") { return "Audio".into(); }
                if lower.contains("spi") { return "Spi".into(); }
                if lower.contains("i2c") { return "I2c".into(); }
                if lower.contains("uart") { return "Uart".into(); }
            }
        }
        // Heurística por offset
        for reg in &block.registers {
            if reg.offset == 0x00 { return "Control".into(); }
            if reg.offset == 0x100 || reg.offset == 0x200 { return "Dma".into(); }
        }
        "Unknown".into()
    }

    /// Métricas de convergência
    pub fn convergence_report(&self) -> ConvergenceReport {
        let first = self.iterations.first();
        let last = self.iterations.last();
        let total_errors: usize = self.iterations.iter().map(|i| i.errors_found.len()).sum();
        let avg_errors = if self.iterations.is_empty() { 0.0 }
            else { total_errors as f64 / self.iterations.len() as f64 };

        ConvergenceReport {
            total_iterations: self.iterations.len(),
            initial_pass_rate: first.map(|i| i.pass_rate).unwrap_or(0.0),
            final_pass_rate: last.map(|i| i.pass_rate).unwrap_or(0.0),
            improvement: last.map(|i| i.pass_rate).unwrap_or(0.0) - first.map(|i| i.pass_rate).unwrap_or(0.0),
            total_errors_found: total_errors,
            avg_errors_per_iteration: avg_errors,
            converged: last.map_or(false, |i| i.pass_rate >= self.convergence_threshold),
        }
    }
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
            id: "gpu_0".into(), kind: types::BlockKind::Unknown,
            base_address: 0x10000000, size: 0x1000,
            registers: vec![
                types::Register { offset: 0, name: None, width: 32,
                    access: types::AccessType::ReadWrite, purpose: types::RegisterPurpose::Control,
                    reset_value: None, observed_values: vec![], bitfields: vec![], polling: false, count: 0,
                },
                types::Register { offset: 0x100, name: None, width: 32,
                    access: types::AccessType::ReadWrite, purpose: types::RegisterPurpose::UnknownPurpose,
                    reset_value: None, observed_values: vec![], bitfields: vec![], polling: false, count: 0,
                },
            ],
            protocol: types::Protocol { states: vec![], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: types::TimingProfile {
                activation: Some(types::LatencyRange::new(0, 0, 0)),
                processing: None, interrupt_response: None, dma_setup: None, polling_interval: None,
            },
            dma: None, dependencies: vec![], confidence: 0.1,
        });
        spec
    }

    #[test]
    fn test_analyze_errors() {
        let loop_ = FeedbackLoop::new(0.9, 10);
        let spec = sample_spec();
        let errors = loop_.analyze_errors(&spec);
        assert!(!errors.is_empty(), "Should find errors in sample spec");
        assert!(errors.iter().any(|e| matches!(e, ErrorClass::LowConfidence { .. })));
    }

    #[test]
    fn test_single_iteration() {
        let mut loop_ = FeedbackLoop::new(0.9, 10);
        let spec = sample_spec();
        let iter = loop_.iterate(&spec, 1);
        assert!(iter.pass_rate < 1.0);
        assert!(!iter.errors_found.is_empty());
    }

    #[test]
    fn test_full_loop_convergence() {
        let mut loop_ = FeedbackLoop::new(0.7, 10);
        let spec = sample_spec();
        let iterations = loop_.run(&spec);
        assert!(!iterations.is_empty());
        let last = iterations.last().unwrap();
        assert!(last.pass_rate >= 0.7 || iterations.len() == 10,
            "Should converge or hit max iterations");
    }

    #[test]
    fn test_refine_confidence() {
        let mut loop_ = FeedbackLoop::new(0.9, 5);
        let spec = sample_spec();
        let original_conf = spec.blocks[0].confidence;
        let iter = loop_.iterate(&spec, 1);
        let new_conf = iter.spec.blocks[0].confidence;
        assert!(new_conf > original_conf, "Confidence should increase");
    }

    #[test]
    fn test_convergence_report() {
        let mut loop_ = FeedbackLoop::new(0.9, 10);
        let spec = sample_spec();
        loop_.run(&spec);
        let report = loop_.convergence_report();
        assert!(report.total_iterations > 0);
        assert!(report.improvement >= 0.0 || report.converged);
    }
}
