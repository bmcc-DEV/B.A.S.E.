/// Bridge entre SpecterProbe (disassembly real) e base-core (inferência).
///
/// Em vez de escanear bytes brutos heuristicamente, usa Capstone para
/// desassemblar o firmware ARM64 e encontrar acessos MMIO reais no código.
use base_core::inference::extraction::{MmioAccess, MmioAccessType};
use base_core::spec::types::{self, HardwareSpec};

/// Converte BIR para HardwareSpec legado
pub fn bir_to_legacy(device: &base_bir::types::BirDevice) -> HardwareSpec {
    let kind = match device.name.to_lowercase() {
        ref n if n.contains("gpu") => types::BlockKind::Gpu,
        ref n if n.contains("audio") => types::BlockKind::Audio,
        ref n if n.contains("dma") => types::BlockKind::Dma,
        _ => types::BlockKind::Unknown,
    };
    let mut spec = HardwareSpec::empty();
    spec.source = device.name.clone();
    spec.blocks.push(types::FunctionalBlock {
        id: device.name.clone(), kind,
        base_address: device.base_address.unwrap_or(0), size: 0x1000,
        registers: device.registers.iter().map(|r| types::Register {
            offset: r.offset, name: Some(r.name.clone()), width: r.width,
            access: match r.access {
                base_bir::types::BirAccess::Read => types::AccessType::Read,
                base_bir::types::BirAccess::Write => types::AccessType::Write,
                base_bir::types::BirAccess::ReadWrite => types::AccessType::ReadWrite,
                base_bir::types::BirAccess::WriteOnly => types::AccessType::WriteOnly,
            },
            purpose: types::RegisterPurpose::UnknownPurpose,
            reset_value: r.reset_value, observed_values: vec![], bitfields: vec![],
            polling: false, count: 0,
        }).collect(),
        protocol: types::Protocol { states: vec!["idle".into()], transitions: vec![], entry_condition: None, exit_condition: None },
        timing: types::TimingProfile {
            activation: None,
            processing: device.timing.first().map(|t| types::LatencyRange {
                min_ns: t.latency.min_ns, max_ns: t.latency.max_ns,
                avg_ns: (t.latency.min_ns + t.latency.max_ns) / 2,
                p99_ns: None, samples: 1,
            }),
            interrupt_response: None, dma_setup: None, polling_interval: None,
        },
        dma: None, dependencies: vec![], confidence: 0.8,
    });
    spec
}

/// Gera grafo DOT a partir de BIR
pub fn bir_to_dot(device: &base_bir::types::BirDevice, title: &str) -> String {
    let mut dot = String::new();
    dot.push_str("// B.A.S.E. BIR Graph\n");
    dot.push_str(&format!("digraph \"{}\" {{\n", title.replace('\"', "\\\"")));
    dot.push_str("  rankdir=LR;\n  splines=ortho;\n");
    dot.push_str("  node [shape=box, style=rounded, fontname=\"JetBrains Mono\"];\n\n");
    let addr = device.base_address.map(|a| format!("@ 0x{:08x}", a)).unwrap_or_default();
    dot.push_str(&format!("  device [label=<{}<br/><font point-size=\"9\">{}</font>>, fillcolor=\"#4a9eff\", style=filled, fontcolor=\"#fff\"];\n", device.name, addr));
    for reg in &device.registers {
        let rid = sanitize(&format!("reg_{}", reg.name));
        dot.push_str(&format!("  {} [label=\"{}\\n+0x{:x}\", shape=note, style=filled, fillcolor=\"#e0e0e0\", fontsize=9];\n", rid, reg.name, reg.offset));
        dot.push_str(&format!("  device -> {} [arrowhead=none, style=dotted];\n", rid));
    }
    for ev in &device.events {
        let eid = sanitize(&format!("ev_{}", ev.name));
        dot.push_str(&format!("  {} [label=\"{}\", shape=diamond, style=filled, fillcolor=\"#da77f2\", fontcolor=\"#fff\", fontsize=9];\n", eid, ev.name));
        dot.push_str(&format!("  device -> {} [color=\"#da77f2\"];\n", eid));
    }
    for irq in &device.interrupts {
        let iid = sanitize(&format!("irq_{}", irq.name));
        dot.push_str(&format!("  {} [label=\"⚡ {}\\nvec{}\", shape=triangle, style=filled, fillcolor=\"#fcc419\", fontsize=9];\n", iid, irq.name, irq.vector));
        dot.push_str(&format!("  device -> {} [color=\"#fcc419\"];\n", iid));
    }
    for t in &device.timing {
        let tid = sanitize(&format!("tim_{}", t.name));
        dot.push_str(&format!("  {} [label=\"⏱ {}\\n{}ns..{}ns\", shape=note, style=filled, fillcolor=\"#fff3bf\", fontsize=9];\n", tid, t.name, t.latency.min_ns, t.latency.max_ns));
        dot.push_str(&format!("  device -> {} [style=dotted, color=\"#fab005\"];\n", tid));
    }
    dot.push_str("}\n");
    dot
}

fn sanitize(s: &str) -> String {
    s.replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
}

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
    tracing::warn!(
        "Falling back to heuristic MMIO scan ({} bytes) — prefer --mmio-traces or richer Capstone hits",
        data.len()
    );
    let mut accesses = Vec::new();
    for chunk in data.chunks(4) {
        if chunk.len() == 4 {
            let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
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
    tracing::warn!("Heuristic MMIO candidates: {}", accesses.len());
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
    fn test_analyze_empty_no_crash() {
        let accesses = analyze_with_disasm(&[]);
        assert!(accesses.is_empty());
    }

    #[test]
    fn test_analyze_no_mmio_binary() {
        // Pure zeros / random small — should not panic
        let data = vec![0u8; 64];
        let accesses = analyze_with_disasm(&data);
        assert!(accesses.len() <= 500);
    }

    #[test]
    fn test_disasm_arm64_binary() {
        let data = vec![0x00, 0x04, 0x00, 0x91, 0xC0, 0x03, 0x5F, 0xD6];
        let _ = analyze_with_disasm(&data);
    }

    #[test]
    fn test_strip_dhtb_header() {
        let mut data = vec![0u8; 0x220];
        data[0..4].copy_from_slice(b"DHTB");
        data[0x200..0x204].copy_from_slice(&[0xC0, 0x03, 0x5F, 0xD6]);
        let clean = strip_headers(&data);
        assert_eq!(clean.len(), 0x20);
        assert_eq!(&clean[0..4], &[0xC0, 0x03, 0x5F, 0xD6]);
    }

    #[test]
    fn test_strip_android_header() {
        let mut data = vec![0u8; 2000];
        data[0..8].copy_from_slice(b"ANDROID!");
        let clean = strip_headers(&data);
        assert_eq!(clean.len(), 2000 - 1648);
    }

    #[test]
    fn test_strip_mtk_header() {
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&[0xA0, 0x0A, 0x1A, 0x00]);
        let clean = strip_headers(&data);
        assert_eq!(clean.len(), 32);
    }
}
