use crate::spec::types::{AccessType, BlockKind, Register, RegisterPurpose};

/// Dados brutos de um acesso MMIO (entrada para inferência)
#[derive(Debug, Clone)]
pub struct MmioAccess {
    pub address: u64,
    pub value: Option<u64>,
    pub access_type: MmioAccessType,
    pub function_name: String,
    pub instruction_addr: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmioAccessType {
    Read,
    Write,
}

/// Cluster de blocos agrupados por página de memória
#[derive(Debug, Clone)]
pub struct BlockCluster {
    pub base_address: u64,
    pub size: u64,
    pub block_type: BlockType,
    pub registers: Vec<RawRegister>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    RegisterFile,
    Fifo,
    Doorbell,
    Status,
    DmaDescriptor,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct RawRegister {
    pub offset: u32,
    pub writes: Vec<u64>,
    pub reads: Vec<u64>,
    pub instruction_addrs: Vec<u64>,
    pub function_names: Vec<String>,
}

/// Agrupa acessos MMIO por boundary de 4K
pub fn group_by_page(accesses: &[MmioAccess]) -> Vec<Vec<&MmioAccess>> {
    let mut pages: std::collections::HashMap<u64, Vec<&MmioAccess>> = std::collections::HashMap::new();
    for access in accesses {
        let page = access.address & !0xFFF;
        pages.entry(page).or_default().push(access);
    }
    pages.into_values().collect()
}

/// Extrai clusters de blocos a partir de acessos MMIO brutos
pub fn extract_blocks(accesses: &[MmioAccess]) -> Vec<BlockCluster> {
    let groups = group_by_page(accesses);
    let mut clusters = Vec::new();

    for group in &groups {
        let base = group.first().map(|a| a.address & !0xFFF).unwrap_or(0);
        let max_addr = group.iter().map(|a| a.address).max().unwrap_or(base);
        let size = ((max_addr - base) / 4 + 1) * 4; // align to 4 bytes

        let registers = extract_registers(group, base);
        let block_type = classify_block(group, &registers);
        let confidence = calculate_block_confidence(group, &registers);

        clusters.push(BlockCluster {
            base_address: base,
            size: size.min(0x1000), // max 4K per page
            block_type,
            registers,
            confidence,
        });
    }

    clusters
}

fn extract_registers(accesses: &[&MmioAccess], base: u64) -> Vec<RawRegister> {
    let mut regs: std::collections::HashMap<u32, RawRegister> = std::collections::HashMap::new();

    for access in accesses {
        let offset = (access.address - base) as u32;
        let entry = regs.entry(offset).or_insert(RawRegister {
            offset,
            writes: Vec::new(),
            reads: Vec::new(),
            instruction_addrs: Vec::new(),
            function_names: Vec::new(),
        });

        entry.instruction_addrs.push(access.instruction_addr);
        entry.function_names.push(access.function_name.clone());

        match access.access_type {
            MmioAccessType::Read => entry.reads.push(access.value.unwrap_or(0)),
            MmioAccessType::Write => entry.writes.push(access.value.unwrap_or(0)),
        }
    }

    let mut regs: Vec<RawRegister> = regs.into_values().collect();
    regs.sort_by_key(|r| r.offset);
    regs
}

fn classify_block(accesses: &[&MmioAccess], regs: &[RawRegister]) -> BlockType {
    let total = accesses.len() as f64;
    if total == 0.0 {
        return BlockType::Unknown;
    }

    let writes = accesses.iter().filter(|a| matches!(a.access_type, MmioAccessType::Write)).count() as f64;
    let reads = accesses.iter().filter(|a| matches!(a.access_type, MmioAccessType::Read)).count() as f64;
    let write_ratio = writes / total;
    let read_ratio = reads / total;

    let unique_write_vals: std::collections::HashSet<u64> = accesses.iter()
        .filter_map(|a| if matches!(a.access_type, MmioAccessType::Write) { a.value } else { None })
        .collect();

    let is_sequential = regs.iter().all(|r| {
        let all_values: Vec<u64> = r.writes.iter().chain(r.reads.iter()).copied().collect();
        if all_values.len() < 2 { return true; }
        all_values.windows(2).any(|w| w[1] == w[0] || w[1] == w[0] + 1 || (w[1] as i64 - w[0] as i64).abs() > 0x1000)
    });

    let has_fixed_offsets = regs.len() <= 16 && regs.iter().all(|r| r.offset % 4 == 0);

    match (write_ratio, read_ratio, unique_write_vals.len(), is_sequential, has_fixed_offsets) {
        (w, _, u, _, _) if w > 0.9 && u <= 3 => BlockType::Doorbell,
        (_, r, _, _, _) if r > 0.9 => BlockType::Status,
        (w, _, _, _, true) if w > 0.3 && w < 0.8 => BlockType::RegisterFile,
        (w, r, _, seq, _) if (w > 0.7 || r > 0.7) && seq => BlockType::Fifo,
        _ => BlockType::Unknown,
    }
}

fn calculate_block_confidence(accesses: &[&MmioAccess], regs: &[RawRegister]) -> f64 {
    let mut score = 0.0f64;
    let mut factors = 0u32;

    // Fator 1: Quantidade de registradores (quanto mais, mais confiança)
    if !regs.is_empty() {
        let reg_score = (regs.len() as f64 / 12.0).min(1.0);
        score += reg_score;
        factors += 1;
    }

    // Fator 2: Múltiplas funções acessando o mesmo bloco
    let unique_funcs: std::collections::HashSet<&str> = accesses.iter().map(|a| a.function_name.as_str()).collect();
    let func_score = (unique_funcs.len() as f64 / 3.0).min(1.0);
    score += func_score * 0.25;
    factors += 1;

    // Fator 3: Volume de acessos
    let access_score = (accesses.len() as f64 / 20.0).min(1.0);
    score += access_score * 0.2;
    factors += 1;

    // Fator 4: Tipo classificado != Unknown (classificação confiante)
    let classified = regs.iter().any(|r| !r.writes.is_empty() || !r.reads.is_empty());
    if classified {
        score += 0.25;
        factors += 1;
    }

    // Fator 5: Padrão de acesso consistente (writes e reads balanceados)
    if !regs.is_empty() {
        let total_writes: usize = regs.iter().map(|r| r.writes.len()).sum();
        let total_reads: usize = regs.iter().map(|r| r.reads.len()).sum();
        let total = total_writes + total_reads;
        if total > 0 {
            let balance = (total_writes as f64 / total as f64).min(total_reads as f64 / total as f64);
            if balance > 0.2 {
                score += 0.1;
                factors += 1;
            }
        }
    }

    if factors == 0 { 0.2 } else { (score / factors as f64 * 1.2).clamp(0.05, 0.99) }
}

/// Converte BlockType para BlockKind
pub fn block_type_to_kind(bt: BlockType, _block: &BlockCluster) -> BlockKind {
    match bt {
        BlockType::Doorbell => BlockKind::Gpu,
        BlockType::Fifo => BlockKind::Dma,
        BlockType::Status => BlockKind::InterruptController,
        BlockType::RegisterFile => BlockKind::Unknown,
        BlockType::DmaDescriptor => BlockKind::Dma,
        BlockType::Unknown => BlockKind::Unknown,
    }
}

/// Converte RawRegister para Register com heurísticas
pub fn raw_to_register(raw: &RawRegister) -> Register {
    let access = if !raw.writes.is_empty() && !raw.reads.is_empty() {
        AccessType::ReadWrite
    } else if !raw.writes.is_empty() {
        AccessType::Write
    } else {
        AccessType::Read
    };

    let polling = raw.reads.len() > 3;
    let purpose = guess_purpose(raw, polling);

    Register {
        offset: raw.offset,
        name: None,
        width: 32,
        access,
        purpose,
        reset_value: raw.writes.first().copied(),
        observed_values: raw_to_observed(raw),
        bitfields: Vec::new(),
        polling,
        count: raw.writes.len() + raw.reads.len(),
    }
}

fn raw_to_observed(raw: &RawRegister) -> Vec<crate::spec::types::ObservedValue> {
    let mut counts: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for &v in raw.writes.iter().chain(raw.reads.iter()) {
        *counts.entry(v).or_default() += 1;
    }
    counts.into_iter().map(|(value, count)| crate::spec::types::ObservedValue {
        value, count, context: String::new(),
    }).collect()
}

fn guess_purpose(raw: &RawRegister, polling: bool) -> RegisterPurpose {
    if polling {
        return RegisterPurpose::Status;
    }
    if raw.writes.len() == 1 && raw.reads.is_empty() {
        return RegisterPurpose::Control;
    }
    if raw.writes.len() > 2 && raw.reads.is_empty() {
        RegisterPurpose::DataPort
    } else if !raw.writes.is_empty() && !raw.reads.is_empty() {
        RegisterPurpose::AddressPointer
    } else {
        RegisterPurpose::UnknownPurpose
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_access(addr: u64, val: Option<u64>, at: MmioAccessType) -> MmioAccess {
        MmioAccess {
            address: addr,
            value: val,
            access_type: at,
            function_name: "test_func".into(),
            instruction_addr: 0,
        }
    }

    #[test]
    fn test_group_by_page() {
        let accesses = vec![
            mock_access(0x10000000, Some(1), MmioAccessType::Write),
            mock_access(0x10001000, Some(2), MmioAccessType::Write),
            mock_access(0x10000004, Some(3), MmioAccessType::Read),
        ];
        let groups = group_by_page(&accesses);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_extract_blocks_empty() {
        let blocks = extract_blocks(&[]);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_extract_blocks_doorbell() {
        let accesses = vec![
            mock_access(0x10000000, Some(1), MmioAccessType::Write),
            mock_access(0x10000000, Some(0), MmioAccessType::Write),
            mock_access(0x10000000, Some(1), MmioAccessType::Write),
        ];
        let blocks = extract_blocks(&accesses);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Doorbell);
    }
}
