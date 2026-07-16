//! Autonomous study runner — Forth words + Lua policy over FeedbackLoop.

use crate::policy::StudyPolicy;
use crate::vm::{StudyContext, Vm, VmError, Word};
use anyhow::Result;
use base_core::loop_::{StopReason, CONTINUOUS_ITERATION_CAP};
use base_core::spec::types::HardwareSpec;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StudyStep {
    pub number: usize,
    pub words: Vec<String>,
    pub pass_rate: f64,
    pub structural_changes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StudyReport {
    pub total_steps: usize,
    pub steps: Vec<StudyStep>,
    pub initial_pass_rate: f64,
    pub final_pass_rate: f64,
    pub stop_reason: StopReason,
    pub stagnated: bool,
    pub converged: bool,
    pub threshold: f64,
    pub max_steps: usize,
    pub continuous: bool,
    /// Always false — structural refine only, not full auto-fix.
    pub auto_fix_complete: bool,
    pub words_executed: Vec<String>,
}

/// Default per-step Forth program when `--program` is omitted.
pub const DEFAULT_STEP_PROGRAM: &str = "OBSERVE SCORE REFINE STAGNATE? CONVERGED?";

pub fn run_study(
    initial: &HardwareSpec,
    policy: &StudyPolicy,
    program_src: Option<&str>,
) -> Result<(HardwareSpec, StudyReport)> {
    let max_steps = if policy.continuous {
        policy.max_steps.max(CONTINUOUS_ITERATION_CAP)
    } else {
        policy.max_steps
    };

    let step_prog = Vm::parse_program(program_src.unwrap_or(DEFAULT_STEP_PROGRAM))?;
    let mut vm = Vm::new();
    let mut ctx = StudyContext::new(initial.clone(), policy.threshold, max_steps);

    // Baseline score
    let _ = vm.exec_word(Word::Score, &mut ctx);
    let initial_pass = ctx.last_pass_rate;
    ctx.words_log.clear();

    let mut steps = Vec::new();
    let mut stop = StopReason::MaxIterations;

    for _ in 1..=max_steps {
        let words_before = ctx.words_log.len();
        match vm.exec_program(&step_prog, &mut ctx) {
            Ok(()) => {}
            Err(VmError::Halt) => {
                stop = if ctx.last_pass_rate >= policy.threshold {
                    StopReason::Converged
                } else if ctx.last_structural_changes == 0 {
                    StopReason::Stagnated
                } else {
                    StopReason::MaxIterations
                };
                break;
            }
            Err(e) => return Err(e.into()),
        }

        let step_words: Vec<String> = ctx.words_log[words_before..].to_vec();
        steps.push(StudyStep {
            number: ctx.step,
            words: step_words,
            pass_rate: ctx.last_pass_rate,
            structural_changes: ctx.last_structural_changes,
        });

        if ctx.last_pass_rate >= policy.threshold {
            stop = StopReason::Converged;
            break;
        }
        if ctx.last_structural_changes == 0 {
            stop = StopReason::Stagnated;
            break;
        }
    }

    ctx.loop_.stop_reason = stop;

    let report = StudyReport {
        total_steps: steps.len(),
        steps,
        initial_pass_rate: initial_pass,
        final_pass_rate: ctx.last_pass_rate,
        stop_reason: stop,
        stagnated: stop == StopReason::Stagnated,
        converged: stop == StopReason::Converged,
        threshold: policy.threshold,
        max_steps,
        continuous: policy.continuous,
        auto_fix_complete: false,
        words_executed: ctx.words_log.clone(),
    };

    Ok((ctx.spec, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;

    fn sample_spec() -> HardwareSpec {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(FunctionalBlock {
            id: "uart0".into(),
            kind: BlockKind::Uart,
            base_address: 0x4003_4000,
            size: 0x1000,
            registers: vec![Register {
                offset: 0x00,
                name: None,
                width: 32,
                access: AccessType::ReadWrite,
                purpose: RegisterPurpose::UnknownPurpose,
                reset_value: None,
                observed_values: vec![],
                bitfields: vec![],
                polling: false,
                count: 0,
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
            confidence: 0.5,
        });
        spec
    }

    #[test]
    fn study_stops_with_reason() {
        let policy = StudyPolicy {
            threshold: 0.99,
            max_steps: 16,
            continuous: false,
        };
        let (spec, report) = run_study(&sample_spec(), &policy, None).unwrap();
        assert!(!report.auto_fix_complete);
        assert!(report.total_steps >= 1);
        assert!(matches!(
            report.stop_reason,
            StopReason::Converged | StopReason::Stagnated | StopReason::MaxIterations
        ));
        assert!(spec.blocks[0].registers[0].name.is_some());
    }
}
