/// Geração de Graphviz DOT do grafo comportamental inferido.
///
/// Produz um grafo visual mostrando conexões entre CPU, barramentos,
/// periféricos e registradores — a "essência" da máquina.
use crate::spec::types::{self, HardwareSpec};

pub fn generate_dot(spec: &HardwareSpec, title: &str) -> String {
    let mut dot = String::new();
    dot.push_str("// B.A.S.E. Behavioral Graph\n");
    dot.push_str(&format!("digraph \"{}\" {{\n", escape_dot(title)));
    dot.push_str("  rankdir=LR;\n");
    dot.push_str("  splines=ortho;\n");
    dot.push_str("  node [shape=box, style=rounded, fontname=\"JetBrains Mono\"];\n");
    dot.push_str("  edge [fontname=\"JetBrains Mono\", fontsize=10];\n\n");

    // CPU node
    dot.push_str(&format!(
        "  cpu [label=<CPU<br/><font point-size=\"10\">{:?} @ {}MHz</font>>, shape=box, style=filled, fillcolor=\"#4a9eff\"];\n",
        spec.cpu.architecture, spec.cpu.clock_mhz
    ));

    // Memory node
    dot.push_str("  mem [label=<Memory<br/><font point-size=\"10\">");
    for (i, region) in spec.memory.regions.iter().enumerate() {
        if i > 0 { dot.push_str(",<br/>"); }
        dot.push_str(&format!("{:?}@{:08x}", region.region_type, region.base));
    }
    dot.push_str("</font>>, shape=box, style=filled, fillcolor=\"#69db7c\"];\n");

    // Edge: CPU → Memory
    dot.push_str("  cpu -> mem [label=\"bus\", style=bold];\n");

    // Block nodes
    for block in &spec.blocks {
        let color = match block.kind {
            types::BlockKind::Gpu => "#ff6b6b",
            types::BlockKind::Audio => "#ffd43b",
            types::BlockKind::Dma => "#da77f2",
            types::BlockKind::Usb => "#20c997",
            types::BlockKind::Ethernet => "#5c7cfa",
            types::BlockKind::Spi => "#f06595",
            types::BlockKind::I2c => "#9775fa",
            types::BlockKind::Uart => "#748ffc",
            types::BlockKind::Timer => "#38d9a9",
            types::BlockKind::InterruptController => "#fcc419",
            _ => "#adb5bd",
        };

        dot.push_str(&format!(
            "  {} [label=<{}<br/><font point-size=\"10\">0x{:08x}</font>>, fillcolor=\"{}\"];\n",
            sanitize_dot_id(&block.id), block_name(&block.kind), block.base_address, color
        ));

        // Edge: CPU → Block
        dot.push_str(&format!(
            "  cpu -> {} [label=\"MMIO\", style=dashed];\n",
            sanitize_dot_id(&block.id)
        ));

        // Register sub-nodes for blocks with registers
        for reg in &block.registers {
            let reg_id = format!("{}_{:x}", sanitize_dot_id(&block.id), reg.offset);
            let reg_name = reg.name.as_deref().unwrap_or("reg");
            dot.push_str(&format!(
                "  {} [label=\"{:?}\", shape=note, style=filled, fillcolor=\"#e0e0e0\", fontsize=9];\n",
                reg_id, reg_name
            ));
            dot.push_str(&format!(
                "  {} -> {} [label=\"+0x{:x}\", arrowhead=none];\n",
                sanitize_dot_id(&block.id), reg_id, reg.offset
            ));
        }

        // Interrupt edge
        if block.kind == types::BlockKind::InterruptController {
            dot.push_str(&format!(
                "  {} -> cpu [label=\"IRQ\", style=dashed, color=\"#fcc419\"];\n",
                sanitize_dot_id(&block.id)
            ));
        }
    }

    // Global confidence
    dot.push_str(&format!(
        "\n  // Confidence: {:.1}%\n",
        spec.confidence * 100.0
    ));

    dot.push_str("}\n");
    dot
}

fn block_name(kind: &types::BlockKind) -> &'static str {
    match kind {
        types::BlockKind::Gpu => "GPU",
        types::BlockKind::Audio => "Audio",
        types::BlockKind::Dma => "DMA",
        types::BlockKind::Usb => "USB",
        types::BlockKind::Ethernet => "Ethernet",
        types::BlockKind::Spi => "SPI",
        types::BlockKind::I2c => "I2C",
        types::BlockKind::Uart => "UART",
        types::BlockKind::Timer => "Timer",
        types::BlockKind::InterruptController => "IRQ Ctrl",
        types::BlockKind::MemoryController => "Mem Ctrl",
        types::BlockKind::Crypto => "Crypto",
        types::BlockKind::VideoCodec => "Video",
        types::BlockKind::Isp => "ISP",
        types::BlockKind::Npu => "NPU",
        types::BlockKind::Unknown => "Unknown",
    }
}

fn sanitize_dot_id(s: &str) -> String {
    s.replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
}

fn escape_dot(s: &str) -> String {
    s.replace('\"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_dot_empty() {
        let spec = types::HardwareSpec::empty();
        let dot = generate_dot(&spec, "test");
        assert!(dot.contains("digraph"));
        assert!(dot.contains("CPU"));
        assert!(dot.contains("Memory"));
    }

    #[test]
    fn test_generate_dot_with_blocks() {
        let mut spec = types::HardwareSpec::empty();
        spec.blocks.push(types::FunctionalBlock {
            id: "gpu_0".into(),
            kind: types::BlockKind::Gpu,
            base_address: 0x10000000,
            size: 0x1000,
            registers: vec![
                types::Register {
                    offset: 0, name: Some("control".into()), width: 32,
                    access: types::AccessType::ReadWrite,
                    purpose: types::RegisterPurpose::Control,
                    reset_value: None, observed_values: vec![], bitfields: vec![],
                    polling: false, count: 1,
                },
            ],
            protocol: types::Protocol {
                states: vec![], transitions: vec![],
                entry_condition: None, exit_condition: None,
            },
            timing: types::TimingProfile {
                activation: None, processing: None, interrupt_response: None,
                dma_setup: None, polling_interval: None,
            },
            dma: None, dependencies: vec![], confidence: 0.8,
        });
        let dot = generate_dot(&spec, "GPU Test");
        assert!(dot.contains("gpu_0"));
        assert!(dot.contains("GPU"));
        assert!(dot.contains("control"));
        assert!(dot.contains("0x10000000"));
    }
}
