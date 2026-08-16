//! Symbol table helpers — resolve CallRel/JmpRel targets to names.

use std::collections::HashMap;

use crate::sir::{Module, Op};

/// Address → symbol name (best effort; first wins).
pub type SymbolMap = HashMap<u64, String>;

/// Fill `symbol` on CallRel/JmpRel when `target` hits the map.
pub fn resolve_symbols(module: &mut Module, symbols: &SymbolMap) -> usize {
    let mut hit = 0usize;
    for func in &mut module.functions {
        for bb in &mut func.blocks {
            for op in &mut bb.ops {
                match op {
                    Op::CallRel {
                        target: Some(t),
                        symbol,
                        ..
                    }
                    | Op::JmpRel {
                        target: Some(t),
                        symbol,
                        ..
                    } => {
                        if symbol.is_none() {
                            if let Some(name) = symbols.get(t) {
                                *symbol = Some(name.clone());
                                hit += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    hit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lift::lift_x86_32_at;

    #[test]
    fn resolve_call_symbol() {
        let mut m = lift_x86_32_at(&[0xE8, 0x00, 0x00, 0x00, 0x00, 0xC3], "c", 0x1000).unwrap();
        let mut map = SymbolMap::new();
        map.insert(0x1005, "helper".into());
        assert_eq!(resolve_symbols(&mut m, &map), 1);
        match &m.functions[0].blocks[0].ops[0] {
            Op::CallRel {
                symbol: Some(s), ..
            } => assert_eq!(s, "helper"),
            other => panic!("{other:?}"),
        }
    }
}
