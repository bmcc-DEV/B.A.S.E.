//! Forth-like Specter VM: dictionary + word execution over a study context.

use crate::stack::{DataStack, StackError};
use base_core::loop_::FeedbackLoop;
use base_core::spec::types::HardwareSpec;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error(transparent)]
    Stack(#[from] StackError),
    #[error("unknown word: {0}")]
    UnknownWord(String),
    #[error("halt")]
    Halt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Word {
    Observe,
    Score,
    Refine,
    StagnateQ,
    ConvergedQ,
    Halt,
    Dup,
    Drop,
}

impl Word {
    pub fn parse(token: &str) -> Option<Self> {
        match token.to_ascii_uppercase().as_str() {
            "OBSERVE" => Some(Self::Observe),
            "SCORE" => Some(Self::Score),
            "REFINE" => Some(Self::Refine),
            "STAGNATE?" => Some(Self::StagnateQ),
            "CONVERGED?" => Some(Self::ConvergedQ),
            "HALT" => Some(Self::Halt),
            "DUP" => Some(Self::Dup),
            "DROP" => Some(Self::Drop),
            _ => None,
        }
    }
}

/// Mutable study context shared by words.
pub struct StudyContext {
    pub loop_: FeedbackLoop,
    pub spec: HardwareSpec,
    pub last_pass_rate: f64,
    pub last_structural_changes: usize,
    pub step: usize,
    pub words_log: Vec<String>,
}

impl StudyContext {
    pub fn new(spec: HardwareSpec, threshold: f64, max_iterations: usize) -> Self {
        Self {
            loop_: FeedbackLoop::new(threshold, max_iterations),
            spec,
            last_pass_rate: 0.0,
            last_structural_changes: 0,
            step: 0,
            words_log: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Vm {
    pub stack: DataStack,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            stack: DataStack::new(),
        }
    }

    pub fn parse_program(src: &str) -> Result<Vec<Word>, VmError> {
        let mut out = Vec::new();
        for raw in src.split_whitespace() {
            // skip Forth comments \( ... \) simple line comments starting with \
            if raw.starts_with('\\') {
                break;
            }
            let w = Word::parse(raw).ok_or_else(|| VmError::UnknownWord(raw.to_string()))?;
            out.push(w);
        }
        Ok(out)
    }

    pub fn exec_word(&mut self, word: Word, ctx: &mut StudyContext) -> Result<(), VmError> {
        ctx.words_log.push(format!("{:?}", word));
        match word {
            Word::Observe => {
                let errors = ctx.loop_.analyze_errors_pub(&ctx.spec);
                self.stack.push(errors.len() as i64);
                Ok(())
            }
            Word::Score => {
                let errors = ctx.loop_.analyze_errors_pub(&ctx.spec);
                let rate = ctx.loop_.calculate_pass_rate_pub(&ctx.spec, &errors);
                ctx.last_pass_rate = rate;
                self.stack.push((rate * 1000.0).round() as i64);
                Ok(())
            }
            Word::Refine => {
                ctx.step += 1;
                let iter = ctx.loop_.iterate(&ctx.spec, ctx.step);
                ctx.spec = iter.spec.clone();
                ctx.last_pass_rate = iter.pass_rate;
                ctx.last_structural_changes = iter.structural_changes;
                self.stack.push(iter.structural_changes as i64);
                Ok(())
            }
            Word::StagnateQ => {
                self.stack
                    .push(if ctx.last_structural_changes == 0 && ctx.step > 0 {
                        1
                    } else {
                        0
                    });
                Ok(())
            }
            Word::ConvergedQ => {
                self.stack.push(
                    if ctx.last_pass_rate >= ctx.loop_.convergence_threshold {
                        1
                    } else {
                        0
                    },
                );
                Ok(())
            }
            Word::Halt => Err(VmError::Halt),
            Word::Dup => {
                let v = self.stack.peek()?;
                self.stack.push(v);
                Ok(())
            }
            Word::Drop => {
                let _ = self.stack.pop()?;
                Ok(())
            }
        }
    }

    pub fn exec_program(&mut self, program: &[Word], ctx: &mut StudyContext) -> Result<(), VmError> {
        for w in program {
            self.exec_word(*w, ctx)?;
        }
        Ok(())
    }
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
    fn parse_and_refine() {
        let prog = Vm::parse_program("OBSERVE SCORE REFINE SCORE").unwrap();
        let mut vm = Vm::new();
        let mut ctx = StudyContext::new(sample_spec(), 0.9, 10);
        vm.exec_program(&prog, &mut ctx).unwrap();
        assert!(ctx.step >= 1);
        assert!(ctx.spec.blocks[0].registers[0].name.is_some());
    }

    #[test]
    fn halt_word() {
        let mut vm = Vm::new();
        let mut ctx = StudyContext::new(sample_spec(), 0.9, 10);
        let err = vm.exec_word(Word::Halt, &mut ctx).unwrap_err();
        assert!(matches!(err, VmError::Halt));
    }
}
