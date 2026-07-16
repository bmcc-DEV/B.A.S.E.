//! Data stack for the Specter Forth-like VM.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StackError {
    #[error("stack underflow")]
    Underflow,
}

#[derive(Debug, Default, Clone)]
pub struct DataStack {
    cells: Vec<i64>,
}

impl DataStack {
    pub fn new() -> Self {
        Self { cells: Vec::new() }
    }

    pub fn push(&mut self, v: i64) {
        self.cells.push(v);
    }

    pub fn pop(&mut self) -> Result<i64, StackError> {
        self.cells.pop().ok_or(StackError::Underflow)
    }

    pub fn peek(&self) -> Result<i64, StackError> {
        self.cells.last().copied().ok_or(StackError::Underflow)
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop() {
        let mut s = DataStack::new();
        s.push(42);
        assert_eq!(s.pop().unwrap(), 42);
        assert!(s.pop().is_err());
    }
}
