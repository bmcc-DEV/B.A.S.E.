use crate::behavior::types::{
    AccessSequence, AccessType, Bitfield, DeviceModel, RegisterModel, SequencedAccess,
    StateMachine, Transition, Trigger,
};
use crate::mmio::types::{MmioAccess, MmioRegion};
use std::collections::{HashMap, HashSet};

pub fn calculate_confidence_raw(
    regs: &[RegisterModel],
    state_machine: &Option<StateMachine>,
    init_sequence: &[String],
) -> f64 {
    let mut score = 0.0f64;
    let mut factors = 0u32;

    if !regs.is_empty() {
        let reg_score = (regs.len() as f64 / 10.0).min(1.0);
        score += reg_score;
        factors += 1;
    }

    if !init_sequence.is_empty() {
        score += 0.8;
        factors += 1;
    }

    let has_bitfields = regs.iter().any(|r| !r.bitfields.is_empty());
    if has_bitfields {
        score += 0.3;
        factors += 1;
    }

    if let Some(ref sm) = *state_machine {
        if sm.states.len() > 3 {
            score += 0.5;
            factors += 1;
        }
    }

    let has_polling = regs.iter().any(|r| r.polling);
    if has_polling {
        score += 0.2;
        factors += 1;
    }

    if factors == 0 { 0.3 } else { (score / factors as f64).clamp(0.1, 0.99) }
}

pub fn build_device_models(regions: &[MmioRegion], _raw_accesses: &[MmioAccess]) -> Vec<DeviceModel> {
    let mut models = Vec::new();

    for region in regions {
        let base = region.base;
        let regs = build_register_map(region);

        let sequences = extract_sequences(region);

        let polling_offsets: HashSet<u32> = regs
            .iter()
            .filter(|r| r.polling)
            .map(|r| r.offset)
            .collect();

        let init_sequence = detect_init_sequence(&sequences, &polling_offsets);

        let state_machine = infer_state_machine(&regs, &sequences);

        let raw_confidence = region.confidence as f64;
        let calculated = calculate_confidence_raw(&regs, &state_machine, &init_sequence);
        let confidence = (raw_confidence * 0.4 + calculated * 0.6).clamp(0.0, 1.0) as f32;

        models.push(DeviceModel {
            base,
            name: region.classification.clone(),
            classification: region.classification.clone(),
            registers: regs,
            state_machine,
            init_sequence,
            sequences,
            confidence,
        });
    }

    models
}

fn build_register_map(region: &MmioRegion) -> Vec<RegisterModel> {
    let mut reg_map: HashMap<u32, RegisterModel> = HashMap::new();

    for access in &region.accesses {
        let offset = (access.address - region.base) as u32;
        let entry = reg_map.entry(offset).or_insert(RegisterModel {
            offset,
            name: None,
            access: if matches!(access.access_type, crate::mmio::types::AccessType::Read) {
                AccessType::Read
            } else {
                AccessType::Write
            },
            width: access.size,
            observed_writes: Vec::new(),
            observed_reads: Vec::new(),
            bitfields: Vec::new(),
            polling: false,
            count: 0,
            purpose: None,
        });

        entry.count += 1;

        match access.access_type {
            crate::mmio::types::AccessType::Read => entry.observed_reads.push(0),
            crate::mmio::types::AccessType::Write => entry.observed_writes.push(0),
        }
    }

    let mut regs: Vec<RegisterModel> = reg_map.into_values().collect();

    for reg in &mut regs {
        reg.polling = reg.count > 3 && matches!(reg.access, AccessType::Read);
        reg.purpose = guess_purpose(reg);
        reg.bitfields = detect_bitfields(reg);
    }

    regs.sort_by_key(|r| r.offset);
    regs
}

fn guess_purpose(reg: &RegisterModel) -> Option<String> {
    if reg.polling {
        return Some("status".into());
    }
    if reg.count == 1 && matches!(reg.access, AccessType::Write) {
        return Some("control".into());
    }
    if reg.count > 2 && matches!(reg.access, AccessType::Write) {
        return Some("config".into());
    }
    if matches!(reg.access, AccessType::Read) && reg.count <= 2 {
        return Some("version".into());
    }
    None
}

fn detect_bitfields(reg: &RegisterModel) -> Vec<Bitfield> {
    let mut all_values: Vec<u64> = reg.observed_writes.clone();
    all_values.extend(&reg.observed_reads);

    if all_values.len() < 2 {
        return Vec::new();
    }

    // XOR-based mask: quais bits alternam entre valores observados
    let mut xor_mask: u64 = 0;
    for i in 1..all_values.len() {
        xor_mask |= all_values[i - 1] ^ all_values[i];
    }

    // Mask tracking: quais bits são sempre escritos juntos
    let mut write_mask: u64 = 0;
    if reg.observed_writes.len() >= 2 {
        for i in 1..reg.observed_writes.len() {
            write_mask |= reg.observed_writes[i - 1] ^ reg.observed_writes[i];
        }
    }

    // Combina as duas heurísticas
    let combined = if write_mask != 0 {
        xor_mask & write_mask
    } else {
        xor_mask
    }.max(xor_mask);

    if combined == 0 {
        return Vec::new();
    }

    let mut fields = Vec::new();
    let mut bit: usize = 0;
    while bit < 64 {
        if (combined >> bit) & 1 == 1 {
            let mut w: u8 = 1;
            while (bit + (w as usize)) < 64 && ((combined >> (bit + (w as usize))) & 1) == 1 {
                w += 1;
            }
            let mut values = Vec::new();
            for &v in &all_values {
                let field_val = (v >> bit) & ((1u64 << w) - 1);
                if !values.iter().any(|(val, _)| *val == field_val) {
                    let label = heuristic_field_label(bit as u8, w, field_val);
                    values.push((field_val, label));
                }
            }
            fields.push(Bitfield {
                offset: bit as u8,
                width: w,
                name: None,
                values,
                observed_mask: (1u64 << w) - 1,
            });
            bit += w as usize;
        } else {
            bit += 1;
        }
    }

    fields
}

/// Nomeia heuristicamente um campo baseado em offset, largura e valor observado
fn heuristic_field_label(offset: u8, width: u8, value: u64) -> String {
    match (offset, width, value) {
        (0, 1, 0) => "disabled".into(),
        (0, 1, 1) => "enabled".into(),
        (0, 2, 0) => "idle".into(),
        (0, 2, 1) => "active".into(),
        (0, 2, 2) => "error".into(),
        (_, 1, 0) => "clear".into(),
        (_, 1, 1) => "set".into(),
        _ => format!("val_{}", value),
    }
}

fn extract_sequences(region: &MmioRegion) -> Vec<AccessSequence> {
    let mut func_accesses: HashMap<String, Vec<SequencedAccess>> = HashMap::new();

    for access in &region.accesses {
        let offset = (access.address - region.base) as u32;
        let at = if matches!(access.access_type, crate::mmio::types::AccessType::Read) {
            AccessType::Read
        } else {
            AccessType::Write
        };

        func_accesses
            .entry(access.function_name.clone())
            .or_default()
            .push(SequencedAccess {
                offset,
                access_type: at,
                value: None,
                instruction_addr: access.instruction_addr,
            });
    }

    let mut sequences = Vec::new();
    for (func, mut accs) in func_accesses {
        accs.sort_by_key(|a| a.instruction_addr);
        sequences.push(AccessSequence {
            function: func,
            accesses: accs,
        });
    }

    sequences.sort_by(|a, b| a.accesses.first().map(|x| x.instruction_addr).unwrap_or(0)
        .cmp(&b.accesses.first().map(|x| x.instruction_addr).unwrap_or(0)));

    sequences
}

fn detect_init_sequence(
    sequences: &[AccessSequence],
    polling_offsets: &HashSet<u32>,
) -> Vec<String> {
    let mut init = Vec::new();

    for seq in sequences {
        for acc in &seq.accesses {
            if polling_offsets.contains(&acc.offset) {
                continue;
            }
            let line = match acc.access_type {
                AccessType::Write => format!("write(+0x{:x})", acc.offset),
                AccessType::Read => format!("read(+0x{:x})", acc.offset),
            };
            if !init.contains(&line) {
                init.push(line);
            }
        }
    }

    init
}

fn infer_state_machine(
    regs: &[RegisterModel],
    _sequences: &[AccessSequence],
) -> Option<StateMachine> {
    if regs.is_empty() {
        return None;
    }

    let has_polling = regs.iter().any(|r| r.polling);
    let has_writes = regs.iter().any(|r| matches!(r.access, AccessType::Write));

    let mut states = vec!["idle".to_string()];
    let mut transitions = Vec::new();

    if has_writes {
        states.push("init".to_string());
        transitions.push(Transition {
            from: "idle".into(),
            to: "init".into(),
            trigger: Trigger {
                kind: "first_write".into(),
                register_offset: None,
                value: None,
            },
        });
    }

    // Detecta estados adicionais baseados em padrões de escrita
    let control_offsets: Vec<u32> = regs.iter()
        .filter(|r| matches!(r.purpose, Some(ref p) if p == "control"))
        .map(|r| r.offset)
        .collect();

    for ctrl in &control_offsets {
        if let Some(reg) = regs.iter().find(|r| r.offset == *ctrl) {
            for &val in &reg.observed_writes {
                let state_name = match val {
                    0 => "idle".to_string(),
                    1 => "active".to_string(),
                    2 => "sleep".to_string(),
                    3 => "reset".to_string(),
                    _ => format!("state_{:x}", val),
                };
                if !states.contains(&state_name) {
                    states.push(state_name.clone());
                    transitions.push(Transition {
                        from: states[states.len().saturating_sub(2)].clone(),
                        to: state_name.clone(),
                        trigger: Trigger {
                            kind: "write".into(),
                            register_offset: Some(*ctrl),
                            value: Some(val),
                        },
                    });
                }
            }
        }
    }

    if has_polling {
        if !states.contains(&"running".to_string()) {
            states.push("running".to_string());
        }
        let has_running_transition = transitions.iter().any(|t| t.to == "running");
        if !has_running_transition {
            transitions.push(Transition {
                from: states[states.len().saturating_sub(2)].clone(),
                to: "running".into(),
                trigger: Trigger {
                    kind: "polling_start".into(),
                    register_offset: None,
                    value: None,
                },
            });
        }
    }

    // Detecta "done" state via polling que para
    if has_polling {
        let polling_regs: Vec<&RegisterModel> = regs.iter().filter(|r| r.polling).collect();
        for reg in &polling_regs {
            if reg.observed_reads.len() > 2 {
                let has_change = reg.observed_reads.windows(2).any(|w| w[0] != w[1]);
                if has_change {
                    states.push("done".to_string());
                    transitions.push(Transition {
                        from: "running".into(),
                        to: "done".into(),
                        trigger: Trigger {
                            kind: "polling_complete".into(),
                            register_offset: Some(reg.offset),
                            value: None,
                        },
                    });
                }
            }
        }
    }

    if !transitions.is_empty() {
        states.push("unknown".to_string());
    }

    Some(StateMachine { states, transitions })
}
