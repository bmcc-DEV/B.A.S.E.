//! B.A.S.E. Software Reasoning — evidence-assisted reverse-engineering loop.
//!
//! Observation → questions → beliefs/hypotheses → triad gate → report.
//! No Transformers · no backprop · `generates_os: false`.

pub mod belief;
pub mod hypothesis;
pub mod question;
pub mod report;
pub mod session;
pub mod signals;
pub mod triad;

pub use belief::{BeliefEdge, BeliefGraph, BeliefNode, BeliefRelation};
pub use hypothesis::{Hypothesis, HypothesisSet};
pub use question::{Question, QuestionKind};
pub use report::ReasonReport;
pub use session::ReasoningSession;
pub use signals::{
    questions_from_inconclusive, questions_from_p0_missing, questions_from_twin_miss,
    questions_from_unresolved, questions_from_wedge_yaml, ReasonSignals,
};
pub use triad::{TriadGate, TriadResult, TriadVerdict};
