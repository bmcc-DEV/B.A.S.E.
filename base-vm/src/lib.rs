//! Specter VM — Forth-like behavioral study engine with Lua policy.
//!
//! Autonomous loop over [`base_core::loop_::FeedbackLoop`]. Always reports
//! `auto_fix_complete: false` (structural refine only — not full auto-fix).

pub mod policy;
pub mod stack;
pub mod study;
pub mod vm;

pub use policy::{load_policy, load_policy_str, StudyPolicy, DEFAULT_POLICY_LUA};
pub use stack::DataStack;
pub use study::{run_study, StudyReport, StudyStep};
pub use vm::{Vm, VmError, Word};
