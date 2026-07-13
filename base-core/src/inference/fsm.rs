use crate::inference::extraction::BlockCluster;
use crate::inference::protocol::{InferredProtocol, RegisterRole};
use crate::spec::types::{LatencyRange, Protocol, Transition, Trigger};
use std::collections::HashMap;

/// Estados que um bloco pode assumir
#[derive(Debug, Clone)]
pub struct InferredFsm {
    pub states: Vec<String>,
    pub transitions: Vec<Transition>,
    pub entry_state: String,
    pub exit_states: Vec<String>,
    pub latency_matrix: HashMap<(String, String), LatencyRange>,
}

/// Extrai FSM a partir de cluster + protocolo inferido
pub fn extract_fsm(block: &BlockCluster, protocol: &InferredProtocol) -> InferredFsm {
    let control_vals = find_control_values(block, protocol);
    let states = build_states(&control_vals, block);
    let transitions = find_transitions(block, &control_vals, &states);
    let clean = filter_noise(&transitions, 0.1);
    let latency_matrix = build_latency_matrix(block, &states);

    InferredFsm {
        entry_state: states.first().cloned().unwrap_or_else(|| "idle".into()),
        exit_states: find_exit_states(block, &states),
        states,
        transitions: clean,
        latency_matrix,
    }
}

fn find_control_values(block: &BlockCluster, protocol: &InferredProtocol) -> Vec<(u32, u64)> {
    let mut values = Vec::new();

    for reg in &block.registers {
        let role = protocol.register_roles.get(&reg.offset);
        let is_control = matches!(role, Some(RegisterRole::Control));

        if is_control || reg.offset == 0x00 {
            for &val in &reg.writes {
                if !values.iter().any(|(_, v)| *v == val) {
                    values.push((reg.offset, val));
                }
            }
        }
    }

    values.sort_by_key(|(_, v)| *v);
    values
}

fn build_states(control_vals: &[(u32, u64)], block: &BlockCluster) -> Vec<String> {
    let mut states = vec!["idle".to_string()];

    for (_offset, val) in control_vals {
        let name = match val {
            0 => continue, // idle já existe
            1 => "active".into(),
            2 => "sleep".into(),
            3 => "reset".into(),
            v => format!("state_{:x}", v),
        };
        if !states.contains(&name) {
            states.push(name);
        }
    }

    // Se não há estados adicionais, tenta inferir de polling
    if states.len() <= 1 && block.registers.iter().any(|r| r.reads.len() > 3) {
        states.push("running".into());
        states.push("done".into());
    }

    states.push("unknown".into());
    states
}

fn find_transitions(
    _block: &BlockCluster,
    control_vals: &[(u32, u64)],
    _states: &[String],
) -> Vec<Transition> {
    let mut transitions = Vec::new();

    for (i, (offset, val)) in control_vals.iter().enumerate() {
        let from = if i == 0 {
            "idle"
        } else {
            let prev_val = control_vals[i - 1].1;
            match prev_val {
                0 => "idle",
                1 => "active",
                2 => "sleep",
                3 => "reset",
                _ => "idle",
            }
        };

        let to = match val {
            0 => "idle",
            1 => "active",
            2 => "sleep",
            3 => "reset",
                _ => "idle",
        };

        transitions.push(Transition {
            from: from.into(),
            to: to.into(),
            trigger: Trigger {
                kind: "write".into(),
                register_offset: Some(*offset),
                value: Some(*val),
            },
            latency: None,
        });
    }

    if transitions.is_empty() {
        // Transição default idle → unknown
        transitions.push(Transition {
            from: "idle".into(),
            to: "unknown".into(),
            trigger: Trigger {
                kind: "first_access".into(),
                register_offset: None,
                value: None,
            },
            latency: None,
        });
    }

    transitions
}

fn filter_noise(transitions: &[Transition], _min_freq: f64) -> Vec<Transition> {
    // Remove transições duplicadas
    let mut seen = std::collections::HashSet::new();
    let mut clean = Vec::new();
    for t in transitions {
        let key = (t.from.as_str(), t.to.as_str());
        if seen.insert(key) {
            clean.push(t.clone());
        }
    }
    clean
}

fn build_latency_matrix(
    _block: &BlockCluster,
    _states: &[String],
) -> HashMap<(String, String), LatencyRange> {
    let _ = _states;
    // Placeholder — será preenchida com dados reais de trace
    HashMap::new()
}

fn find_exit_states(block: &BlockCluster, _states: &[String]) -> Vec<String> {
    // Se um registrador tem polling e para de ser acessado, é estado de saída
    let has_polling = block.registers.iter().any(|r| r.reads.len() > 3);

    if has_polling {
        vec!["done".into()]
    } else {
        vec!["idle".into()]
    }
}

/// Converte FSM inferida para o tipo Protocol do spec
pub fn fsm_to_protocol(fsm: &InferredFsm) -> Protocol {
    Protocol {
        states: fsm.states.clone(),
        transitions: fsm.transitions.clone(),
        entry_condition: Some(crate::spec::types::Condition {
            kind: "power_on".into(),
            detail: None,
        }),
        exit_condition: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::extraction::{BlockCluster, BlockType, RawRegister};
    use crate::inference::protocol::{infer_protocol, RegisterRole};
    use std::collections::HashMap;

    fn mock_fsm_block() -> BlockCluster {
        BlockCluster {
            base_address: 0x10000000,
            size: 0x1000,
            block_type: BlockType::RegisterFile,
            registers: vec![
                RawRegister { offset: 0x00, writes: vec![1, 0, 1], reads: vec![], instruction_addrs: vec![10, 20, 30], function_names: vec!["f".into(); 3] },
                RawRegister { offset: 0x04, writes: vec![], reads: vec![0, 0, 1, 1, 0], instruction_addrs: vec![40, 50, 60, 70, 80], function_names: vec!["f".into(); 5] },
            ],
            confidence: 0.8,
        }
    }

    #[test]
    fn test_fsm_extraction() {
        let block = mock_fsm_block();
        let mut roles = HashMap::new();
        roles.insert(0x00, RegisterRole::Control);
        roles.insert(0x04, RegisterRole::Status);

        let protocol = infer_protocol(&block);
        let fsm = extract_fsm(&block, &protocol);

        assert!(!fsm.states.is_empty(), "Should have states");
        assert!(fsm.states.contains(&"idle".to_string()), "Should have idle state");
    }

    #[test]
    fn test_fsm_to_protocol() {
        let fsm = InferredFsm {
            states: vec!["idle".into(), "active".into(), "unknown".into()],
            transitions: vec![Transition {
                from: "idle".into(),
                to: "active".into(),
                trigger: Trigger { kind: "write".into(), register_offset: Some(0), value: Some(1) },
                latency: None,
            }],
            entry_state: "idle".into(),
            exit_states: vec!["idle".into()],
            latency_matrix: HashMap::new(),
        };

        let protocol = fsm_to_protocol(&fsm);
        assert_eq!(protocol.states.len(), 3);
        assert_eq!(protocol.transitions.len(), 1);
    }
}
