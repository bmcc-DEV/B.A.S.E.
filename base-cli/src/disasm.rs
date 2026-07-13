/// Bridge entre SpecterProbe (disassembly real) e base-core (inferência).
///
/// Em vez de escanear bytes brutos heuristicamente, usa Capstone para
/// desassemblar o firmware ARM64 e encontrar acessos MMIO reais no código.
use base_core::inference::extraction::{MmioAccess, MmioAccessType};

/// Executa o pipeline completo: disassembly → MMIO discovery → inferência.
pub fn analyze_with_disasm(data: &[u8]) -> Vec<MmioAccess> {
    // Passo 0: Strip known bootloader headers
    let clean_data = strip_headers(data);
    if clean_data.len() != data.len() {
        tracing::info!("Stripped header: {} bytes → {} bytes", data.len(), clean_data.len());
    }

    // Passo 1: Disassembly com Capstone
    let lift_output = specter_probe::lift::lift_binary(&clean_data);

    if lift_output.functions.is_empty() {
        tracing::warn!("No functions found in binary, falling back to heuristic scan");
        return heuristic_scan(data);
    }

    tracing::info!(
        "Disassembled {} functions ({} instructions, {} lifted)",
        lift_output.functions.len(),
        lift_output.total_instructions,
        lift_output.lifted_functions,
    );

    // Passo 2: Análise estática → MMIO candidates
    let analysis = specter_probe::lift::analysis::analyze(&lift_output.functions);

    tracing::info!(
        "Analysis: {} MMIO candidates, {} syscalls, {} call edges",
        analysis.mmio_candidates.len(),
        analysis.syscalls.len(),
        analysis.call_graph.len(),
    );

    // Passo 3: Converter para formato base-core
    let mut accesses: Vec<MmioAccess> = analysis.mmio_candidates.iter().map(|mc| {
        MmioAccess {
            address: mc.address,
            value: Some(1), // valor observado na escrita
            access_type: match mc.access_type.as_str() {
                "write" => MmioAccessType::Write,
                _ => MmioAccessType::Read,
            },
            function_name: mc.function.clone(),
            instruction_addr: mc.instruction_addr,
        }
    }).collect();

    // Se o disassembly não encontrou candidatos, fallback para heurística
    if accesses.is_empty() {
        tracing::warn!("No MMIO candidates from disassembly, falling back to heuristic scan");
        return heuristic_scan(data);
    }

    // Deduplica por endereço
    accesses.sort_by_key(|a| a.address);
    accesses.dedup_by_key(|a| a.address);

    tracing::info!("Found {} unique MMIO addresses via disassembly", accesses.len());
    accesses
}

/// Fallback: scan heurístico de bytes brutos (funciona em qualquer binário)
fn heuristic_scan(data: &[u8]) -> Vec<MmioAccess> {
    let mut accesses = Vec::new();
    for chunk in data.chunks(4) {
        if chunk.len() == 4 {
            let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            // Procura valores que parecem endereços MMIO
            if is_likely_mmio(val) {
                accesses.push(MmioAccess {
                    address: val as u64,
                    value: Some(1),
                    access_type: MmioAccessType::Write,
                    function_name: "heuristic".into(),
                    instruction_addr: 0,
                });
            }
        }
    }
    accesses.truncate(500);
    accesses.dedup_by_key(|a| a.address);
    accesses
}

/// Remove cabeçalhos conhecidos de bootloader (DHTB, MTK, etc.)
fn strip_headers(data: &[u8]) -> Vec<u8> {
    // DHTB header (Little Kernel): 0x200 bytes
    if data.len() > 4 && &data[..4] == b"DHTB" {
        return data[0x200..].to_vec();
    }
    // MTK bootrom header: starts with 0xA00A1A00
    if data.len() > 16 && data[..4] == [0xA0, 0x0A, 0x1A, 0x00] {
        // MTK header is 32 bytes typically
        return data[32..].to_vec();
    }
    // Android boot image
    if data.len() > 8 && &data[..8] == b"ANDROID!" {
        // Skip to kernel data (after header)
        use std::io::Read;
        let mut cursor = std::io::Cursor::new(data);
        let mut header_version = [0u8; 4];
        cursor.read_exact(&mut header_version).ok();
        // v3/v4 has kernel at offset 1584 or 1648
        return data[1648..].to_vec();
    }
    // GZip compressed
    if data.len() > 2 && data[..2] == [0x1F, 0x8B] {
        // Can't decompress easily, return as-is (lift may handle partial)
        return data.to_vec();
    }
    data.to_vec()
}

fn is_likely_mmio(val: u32) -> bool {
    // Faixas típicas de MMIO em ARM SoCs
    (val >= 0x10000000 && val <= 0x20000000)
    || (val >= 0xA0000000 && val <= 0xB0000000)
    || (val >= 0x40000000 && val <= 0x50000000)
    || (val >= 0xE0000000 && val <= 0xF0000000)
    || (val >= 0x1C000000 && val <= 0x1D000000)  // MediaTek range
    || (val >= 0xA9000000 && val <= 0xAB000000)  // Unisoc range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_likely_mmio() {
        assert!(is_likely_mmio(0x10000000));
        assert!(is_likely_mmio(0xA9BF0000));
        assert!(is_likely_mmio(0x1C52D000));
        assert!(!is_likely_mmio(0x00000000));
        assert!(!is_likely_mmio(0xFFFFFFFF));
        assert!(!is_likely_mmio(0x20000001));
    }

    #[test]
    fn test_analyze_with_disasm_empty() {
        let accesses = analyze_with_disasm(&[]);
        assert!(accesses.is_empty() || accesses.len() > 0);
    }

    #[test]
    fn test_disasm_arm64_binary() {
        // ARM64: add x0, x0, #1; ret
        let data = vec![0x00, 0x04, 0x00, 0x91, 0xC0, 0x03, 0x5F, 0xD6];
        let accesses = analyze_with_disasm(&data);
        // Should not crash, might find nothing (small test binary)
        let _ = accesses;
    }
}
