//! Question types — "what do we still not know?" (QRM)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    MissingP0,
    TwinMiss,
    UnresolvedAddr,
    NeedsLab,
    InconclusiveTension,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Question {
    pub kind: QuestionKind,
    pub subject: String,
    pub prompt: String,
}

impl Question {
    pub fn missing_p0(class: impl Into<String>) -> Self {
        let subject = class.into();
        Self {
            kind: QuestionKind::MissingP0,
            prompt: format!("P0 atlas still missing absolute base for `{subject}` — USB or DT reg?"),
            subject,
        }
    }

    pub fn unresolved_addr(class: impl Into<String>) -> Self {
        let subject = class.into();
        Self {
            kind: QuestionKind::UnresolvedAddr,
            prompt: format!("Address for `{subject}` is Unresolved — which source can resolve it?"),
            subject,
        }
    }

    pub fn twin_miss(block: impl Into<String>) -> Self {
        let subject = block.into();
        Self {
            kind: QuestionKind::TwinMiss,
            prompt: format!("Twin miss / guest-only block `{subject}` — Spec wrong or guest noise?"),
            subject,
        }
    }

    pub fn needs_lab(subject: impl Into<String>) -> Self {
        let subject = subject.into();
        Self {
            kind: QuestionKind::NeedsLab,
            prompt: format!("Lab evidence required for `{subject}` before closing claim (receipt)."),
            subject,
        }
    }

    pub fn inconclusive(note: impl Into<String>) -> Self {
        let subject = note.into();
        Self {
            kind: QuestionKind::InconclusiveTension,
            prompt: format!("Tension inconclusive — need more evidence: {subject}"),
            subject,
        }
    }
}
