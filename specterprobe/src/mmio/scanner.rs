use crate::lift::types::{Function, InstKind, Instruction, Reg};
use crate::mmio::types::{AccessType, MmioAccess};
use std::collections::HashMap;

pub fn scan_functions(functions: &[Function]) -> Vec<MmioAccess> {
    let mut accesses = Vec::new();

    for func in functions {
        let func_name = func.name.clone();

        for block in &func.blocks {
            let mut reg_values: HashMap<String, u64> = HashMap::new();

            for insn in &block.instructions {
                track_reg_defs(insn, &mut reg_values);

                match &insn.kind {
                    InstKind::Store(sz, addr, _) => {
                        if let Some(absolute_addr) = resolve_addr(&addr.base, addr.offset, &reg_values) {
                            let confidence = if matches!(addr.base, Reg::Xzr | Reg::Wzr) {
                                0.9
                            } else if reg_values.contains_key(&reg_key(&addr.base)) {
                                0.7
                            } else {
                                0.4
                            };
                            accesses.push(MmioAccess {
                                address: absolute_addr,
                                size: size_bytes(*sz),
                                access_type: AccessType::Write,
                                instruction_addr: insn.address,
                                function_name: func_name.clone(),
                                confidence,
                            });
                        }
                    }
                    InstKind::Load(sz, _, addr) => {
                        if let Some(absolute_addr) = resolve_addr(&addr.base, addr.offset, &reg_values) {
                            let confidence = if matches!(addr.base, Reg::Xzr | Reg::Wzr) {
                                0.9
                            } else if reg_values.contains_key(&reg_key(&addr.base)) {
                                0.7
                            } else {
                                0.4
                            };
                            accesses.push(MmioAccess {
                                address: absolute_addr,
                                size: size_bytes(*sz),
                                access_type: AccessType::Read,
                                instruction_addr: insn.address,
                                function_name: func_name.clone(),
                                confidence,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    accesses
}

fn reg_key(r: &Reg) -> String {
    // W/X aliases: Capstone str/ldr use Xn as base; mov often writes Wn
    match r {
        Reg::X(n) | Reg::W(n) => format!("r{}", n),
        Reg::R(n) => format!("r{}", n),
        Reg::Fp => "fp".into(),
        Reg::Lr => "lr".into(),
        Reg::Sp => "sp".into(),
        Reg::Xzr | Reg::Wzr => "zr".into(),
        _ => format!("{:?}", r),
    }
}

fn resolve_addr(base: &Reg, offset: i64, reg_values: &HashMap<String, u64>) -> Option<u64> {
    match base {
        Reg::Xzr | Reg::Wzr => Some(offset as u64),
        Reg::Sp | Reg::Fp => None,
        _ => {
            let key = reg_key(base);
            if let Some(base_val) = reg_values.get(&key) {
                Some(base_val.wrapping_add(offset as u64))
            } else {
                None
            }
        }
    }
}

fn track_reg_defs(insn: &Instruction, reg_values: &mut HashMap<String, u64>) {
    match &insn.kind {
        InstKind::Adrp(dst, page) => {
            reg_values.insert(reg_key(dst), *page);
        }
        InstKind::Add(_sz, dst, src, op) => {
            let src_val = get_reg_value(src, reg_values);
            let imm_val = get_imm_op(op);
            if let (Some(sv), Some(iv)) = (src_val, imm_val) {
                reg_values.insert(reg_key(dst), sv.wrapping_add(iv));
            }
        }
        InstKind::Mov(_sz, dst, op) => {
            if let Some(val) = get_imm_op(op) {
                reg_values.insert(reg_key(dst), val);
            }
        }
        InstKind::Sub(_sz, dst, src, op) => {
            let src_val = get_reg_value(src, reg_values);
            let imm_val = get_imm_op(op);
            if let (Some(sv), Some(iv)) = (src_val, imm_val) {
                reg_values.insert(reg_key(dst), sv.wrapping_sub(iv));
            }
        }
        _ => {}
    }
}

fn get_reg_value(r: &Reg, reg_values: &HashMap<String, u64>) -> Option<u64> {
    match r {
        Reg::Xzr | Reg::Wzr => Some(0),
        Reg::Sp => Some(0xffff_ffff_ffff_e000), // approximate SP
        _ => reg_values.get(&reg_key(r)).copied(),
    }
}

fn get_imm_op(op: &crate::lift::types::ArmOperand) -> Option<u64> {
    match op {
        crate::lift::types::ArmOperand::Imm(val) => Some(*val as u64),
        crate::lift::types::ArmOperand::Reg(r) => match r {
            Reg::Xzr | Reg::Wzr => Some(0),
            _ => None,
        },
        _ => None,
    }
}

fn size_bytes(sz: crate::lift::types::Size) -> u8 {
    match sz {
        crate::lift::types::Size::B8 => 1,
        crate::lift::types::Size::B16 => 2,
        crate::lift::types::Size::B32 => 4,
        crate::lift::types::Size::B64 => 8,
        crate::lift::types::Size::B128 => 16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lift::lift_binary;
    use std::fs;
    use std::path::PathBuf;

    /// Bytes from `python3 examples/pilot/gen_fw.py`
    fn uart_fw() -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pilot/fw.bin");
        fs::read(path).unwrap_or_else(|_| {
            hex::decode("a001209021008052010000b9020440b901008052011800b921088052010000b9c0035fd6")
                .expect("static fw hex")
        })
    }

    #[test]
    fn pilot_uart_blob_resolves_0x40034000() {
        let fw = uart_fw();
        assert_eq!(fw.len(), 36);
        let lift = lift_binary(&fw);
        assert!(!lift.functions.is_empty());
        let acc = scan_functions(&lift.functions);
        let addrs: Vec<u64> = acc.iter().map(|a| a.address).collect();
        assert!(
            addrs.contains(&0x40034000) && addrs.contains(&0x40034004) && addrs.contains(&0x40034018),
            "expected Capstone MMIO UART regs, got {:?}",
            addrs
        );
    }

    /// Bytes from `python3 examples/pilot_stm32/gen_fw.py` (V1 Capstone).
    fn stm32_usart1_fw() -> Vec<u8> {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/pilot_stm32/fw.bin");
        fs::read(&path).unwrap_or_else(|e| panic!("read {:?}: {e}", path))
    }

    #[test]
    fn pilot_stm32_usart1_resolves_0x40013800() {
        let fw = stm32_usart1_fw();
        assert!(!fw.is_empty());
        let lift = lift_binary(&fw);
        assert!(!lift.functions.is_empty());
        let acc = scan_functions(&lift.functions);
        let addrs: Vec<u64> = acc.iter().map(|a| a.address).collect();
        assert!(
            addrs.contains(&0x40013800)
                && addrs.contains(&0x40013804)
                && addrs.contains(&0x4001380c),
            "expected Capstone MMIO USART1 regs, got {:?}",
            addrs
        );
    }
}
