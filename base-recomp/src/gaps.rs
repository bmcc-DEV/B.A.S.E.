//! Gap / Unknown ops → report lines (wedge-friendly for `base reason`).

use crate::sir::{Module, Op};

/// One gap line suitable for RECOMP_REPORT / reason handoff.
pub fn gap_lines(module: &Module) -> Vec<String> {
    let mut out = Vec::new();
    for func in &module.functions {
        for bb in &func.blocks {
            for op in &bb.ops {
                if let Op::Unknown {
                    offset,
                    bytes,
                    note,
                } = op
                {
                    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                    out.push(format!(
                        "- gap `{}` @`{}`+{offset:#x} bytes=`{hex}` — {note}",
                        func.name, bb.label
                    ));
                }
            }
        }
    }
    out
}

/// Markdown section listing lift gaps (empty string if none).
pub fn gaps_markdown(module: &Module) -> String {
    let lines = gap_lines(module);
    if lines.is_empty() {
        return String::new();
    }
    let mut md = String::from("## Lift gaps (→ reason wedges)\n\n");
    for l in lines {
        md.push_str(&l);
        md.push('\n');
    }
    md.push('\n');
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lift::lift_x86_32;

    #[test]
    fn gaps_from_int3() {
        let m = lift_x86_32(&[0xCC], "f").unwrap();
        let g = gap_lines(&m);
        assert_eq!(g.len(), 1);
        assert!(g[0].contains("cc"));
    }
}
