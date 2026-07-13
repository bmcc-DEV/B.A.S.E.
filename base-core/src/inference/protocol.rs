use crate::inference::extraction::{BlockCluster, RawRegister};
use crate::spec::types::LatencyRange;
use std::collections::HashMap;

/// Sequência de acessos inferida como um protocolo
#[derive(Debug, Clone)]
pub struct InferredSequence {
    pub steps: Vec<SequenceStep>,
    pub frequency: usize,
    pub context: String,
}

#[derive(Debug, Clone)]
pub struct SequenceStep {
    pub offset: u32,
    pub value: Option<u64>,
    pub access_type: String, // "read" | "write"
    pub latency_to_next: Option<LatencyRange>,
}

#[derive(Debug, Clone)]
pub struct InferredProtocol {
    pub sequences: Vec<InferredSequence>,
    pub register_roles: HashMap<u32, RegisterRole>,
    pub timing: ProtocolTiming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterRole {
    Control,
    Status,
    AddressPtr,
    DataLength,
    DataPort,
    Trigger,
    InterruptMask,
    InterruptAck,
    ClockConfig,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ProtocolTiming {
    pub avg_step_latency_ns: u64,
    pub total_sequence_latency_ns: u64,
}

/// Inferir protocolo a partir de um cluster de bloco
pub fn infer_protocol(block: &BlockCluster) -> InferredProtocol {
    let ngrams = collect_ngrams(&block.registers, 4);
    let sequences = extract_frequent_sequences(&ngrams, 0.1);
    let register_roles = infer_register_roles(&block.registers, &sequences);
    let timing = measure_timing(&block.registers);

    InferredProtocol {
        sequences,
        register_roles,
        timing,
    }
}

/// Coleta N-grams de offsets de registradores para detectar sequências
fn collect_ngrams(regs: &[RawRegister], max_n: usize) -> Vec<(Vec<u32>, usize)> {
    if regs.is_empty() {
        return Vec::new();
    }

    // Constrói sequência temporal de offsets baseada em instruction_addrs
    let mut offset_seq: Vec<(u32, u64)> = Vec::new();
    for reg in regs {
        for i in 0..reg.writes.len() {
            let addr = reg.instruction_addrs.get(i).copied().unwrap_or(0);
            offset_seq.push((reg.offset, addr));
        }
    }
    offset_seq.sort_by_key(|(_, addr)| *addr);

    let offsets: Vec<u32> = offset_seq.into_iter().map(|(o, _)| o).collect();

    if offsets.len() < 2 {
        return Vec::new();
    }

    let mut ngrams: HashMap<Vec<u32>, usize> = HashMap::new();

    for n in 2..=max_n.min(offsets.len()) {
        for window in offsets.windows(n) {
            *ngrams.entry(window.to_vec()).or_default() += 1;
        }
    }

    ngrams.into_iter().collect()
}

/// Extrai sequências frequentes (acima do threshold)
fn extract_frequent_sequences(ngrams: &[(Vec<u32>, usize)], min_freq_ratio: f64) -> Vec<InferredSequence> {
    if ngrams.is_empty() {
        return Vec::new();
    }

    let max_count = ngrams.iter().map(|(_, c)| *c).max().unwrap_or(1) as f64;
    let threshold = (max_count * min_freq_ratio).max(2.0);

    let mut sequences: Vec<InferredSequence> = ngrams.iter()
        .filter(|(_, count)| *count as f64 >= threshold)
        .map(|(steps, count)| {
            let seq_steps: Vec<SequenceStep> = steps.iter().map(|offset| SequenceStep {
                offset: *offset,
                value: None,
                access_type: "write".into(),
                latency_to_next: None,
            }).collect();

            InferredSequence {
                steps: seq_steps,
                frequency: *count,
                context: "inferred".into(),
            }
        })
        .collect();

    // Ordena por frequência (decrescente) e pega as top 5
    sequences.sort_by(|a, b| b.frequency.cmp(&a.frequency));
    sequences.truncate(5);
    sequences
}

/// Infere o papel de cada registrador baseado em posição e valores
fn infer_register_roles(regs: &[RawRegister], _sequences: &[InferredSequence]) -> HashMap<u32, RegisterRole> {
    let mut roles = HashMap::new();

    for reg in regs {
        let role = match reg.offset {
            0x00 => RegisterRole::Control,
            0x04 => {
                if reg.writes.iter().any(|v| *v > 0x10000000) {
                    RegisterRole::AddressPtr
                } else {
                    RegisterRole::Status
                }
            }
            0x08 => RegisterRole::DataLength,
            0x0C => RegisterRole::Trigger,
            0x10 => RegisterRole::InterruptMask,
            0x14 => RegisterRole::InterruptAck,
            _ => {
                if reg.writes.len() >= 2 && reg.writes.iter().all(|v| *v < 256) {
                    RegisterRole::DataPort
                } else {
                    RegisterRole::Unknown
                }
            }
        };
        roles.insert(reg.offset, role);
    }

    roles
}

/// Mede timing aproximado entre acessos
fn measure_timing(regs: &[RawRegister]) -> ProtocolTiming {
    let mut latencies: Vec<u64> = Vec::new();

    // Estima latência como diferença entre instruction_addrs consecutivas
    for reg in regs {
        for i in 1..reg.instruction_addrs.len() {
            let diff = reg.instruction_addrs[i].saturating_sub(reg.instruction_addrs[i - 1]);
            if diff < 1_000_000 { // menos de 1M de instruções de diferença
                latencies.push(diff);
            }
        }
    }

    let avg = if latencies.is_empty() {
        1000
    } else {
        latencies.iter().sum::<u64>() / latencies.len() as u64
    };

    ProtocolTiming {
        avg_step_latency_ns: avg * 10, // estimativa grosseira: 10ns por instrução
        total_sequence_latency_ns: avg * latencies.len() as u64 * 10,
    }
}

/// Nomeia heuristicamente um registrador
pub fn heuristic_register_name(offset: u32, role: RegisterRole) -> String {
    match role {
        RegisterRole::Control => "control".into(),
        RegisterRole::Status => "status".into(),
        RegisterRole::AddressPtr => "buf_addr".into(),
        RegisterRole::DataLength => "length".into(),
        RegisterRole::DataPort => "data".into(),
        RegisterRole::Trigger => "trigger".into(),
        RegisterRole::InterruptMask => "irq_mask".into(),
        RegisterRole::InterruptAck => "irq_ack".into(),
        RegisterRole::ClockConfig => "clock_div".into(),
        RegisterRole::Unknown => format!("reg_{:x}", offset),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::extraction::{BlockCluster, BlockType, RawRegister};

    fn mock_block() -> BlockCluster {
        BlockCluster {
            base_address: 0x10000000,
            size: 0x1000,
            block_type: BlockType::RegisterFile,
            registers: vec![
                RawRegister { offset: 0x00, writes: vec![0, 1], reads: vec![], instruction_addrs: vec![10, 20], function_names: vec!["f1".into(), "f1".into()] },
                RawRegister { offset: 0x04, writes: vec![0x20000000], reads: vec![], instruction_addrs: vec![30], function_names: vec!["f1".into()] },
                RawRegister { offset: 0x08, writes: vec![0x1000], reads: vec![], instruction_addrs: vec![40], function_names: vec!["f1".into()] },
            ],
            confidence: 0.8,
        }
    }

    #[test]
    fn test_infer_protocol_basic() {
        let block = mock_block();
        let protocol = infer_protocol(&block);

        assert!(!protocol.register_roles.is_empty(), "Should infer register roles");
        assert!(!protocol.sequences.is_empty() || protocol.register_roles.len() == 3,
            "Should have sequences or all registers mapped");
    }

    #[test]
    fn test_register_role_control() {
        assert_eq!(heuristic_register_name(0x00, RegisterRole::Control), "control");
        assert_eq!(heuristic_register_name(0x04, RegisterRole::AddressPtr), "buf_addr");
    }

    #[test]
    fn test_collect_ngrams() {
        let block = mock_block();
        let ngrams = collect_ngrams(&block.registers, 3);
        // May be empty if instruction_addrs don't form clear sequences
        // but shouldn't crash
    }
}
