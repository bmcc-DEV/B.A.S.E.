//! Lua policy loader for Specter study runs.

use anyhow::{Context, Result};
use mlua::Lua;
use serde::{Deserialize, Serialize};

/// Default embedded policy (threshold / max_steps).
pub const DEFAULT_POLICY_LUA: &str = r#"
return {
  threshold = 0.9,
  max_steps = 32,
  continuous = false,
}
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudyPolicy {
    pub threshold: f64,
    pub max_steps: usize,
    pub continuous: bool,
}

impl Default for StudyPolicy {
    fn default() -> Self {
        Self {
            threshold: 0.9,
            max_steps: 32,
            continuous: false,
        }
    }
}

pub fn load_policy_str(src: &str) -> Result<StudyPolicy> {
    let lua = Lua::new();
    let value: mlua::Value = lua.load(src).eval().map_err(|e| {
        anyhow::anyhow!("evaluating Lua study policy: {e}")
    })?;
    let table = value
        .as_table()
        .ok_or_else(|| anyhow::anyhow!("policy Lua must return a table"))?;

    let threshold: f64 = table.get("threshold").unwrap_or(0.9);
    let max_steps: usize = table.get::<u32>("max_steps").unwrap_or(32) as usize;
    let continuous: bool = table.get("continuous").unwrap_or(false);

    Ok(StudyPolicy {
        threshold: threshold.clamp(0.0, 1.0),
        max_steps: max_steps.max(1),
        continuous,
    })
}

pub fn load_policy(path: Option<&std::path::Path>) -> Result<StudyPolicy> {
    match path {
        Some(p) => {
            let src = std::fs::read_to_string(p)
                .with_context(|| format!("reading policy {}", p.display()))?;
            load_policy_str(&src)
        }
        None => load_policy_str(DEFAULT_POLICY_LUA),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_parses() {
        let p = load_policy_str(DEFAULT_POLICY_LUA).unwrap();
        assert!((p.threshold - 0.9).abs() < 1e-9);
        assert_eq!(p.max_steps, 32);
        assert!(!p.continuous);
    }

    #[test]
    fn custom_policy() {
        let p = load_policy_str(
            r#"return { threshold = 0.5, max_steps = 5, continuous = true }"#,
        )
        .unwrap();
        assert!((p.threshold - 0.5).abs() < 1e-9);
        assert_eq!(p.max_steps, 5);
        assert!(p.continuous);
    }
}
