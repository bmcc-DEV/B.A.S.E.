/// Geração de Graphviz DOT do grafo comportamental e de eventos inferidos.
///
/// Produz dois grafos:
/// 1. Behavior Graph — "quem conversa com quem" (conexões estruturais)
/// 2. Event Graph — "quem faz o quê" (causalidade: WRITE → DMA_START → IRQ)
use crate::spec::types::{self, HardwareSpec};

/// Gera o Behavior Graph estrutural (conexões entre blocos)
pub fn generate_behavior_dot(spec: &HardwareSpec, title: &str) -> String {
    let mut dot = String::new();
    dot.push_str("// B.A.S.E. Behavioral Graph — Structural\n");
    dot.push_str(&format!("digraph \"{}\" {{\n", escape_dot(title)));
    dot.push_str("  rankdir=LR;\n");
    dot.push_str("  splines=ortho;\n");
    dot.push_str("  node [shape=box, style=rounded, fontname=\"JetBrains Mono\"];\n");
    dot.push_str("  edge [fontname=\"JetBrains Mono\", fontsize=10];\n\n");
    dot.push_str("  label=\"Behavior Graph — who talks to whom\";\n");
    dot.push_str("  labelloc=t; fontsize=20; fontcolor=\"#4a9eff\";\n\n");

    // CPU node with confidence
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
    dot.push_str("  cpu -> mem [label=\"bus\", style=bold];\n");

    // Block nodes with confidence
    for block in &spec.blocks {
        let color = block_color(&block.kind);
        let conf_pct = (block.confidence * 100.0) as i32;
        let conf_color = if conf_pct >= 70 { "#69db7c" } else if conf_pct >= 40 { "#ffd43b" } else { "#ff6b6b" };

        dot.push_str(&format!(
            "  {} [label=<{}<br/><font point-size=\"10\">0x{:08x}</font><br/><font point-size=\"9\" color=\"{}\">conf: {}%</font>>, fillcolor=\"{}\"];\n",
            sanitize_dot_id(&block.id), block_name(&block.kind), block.base_address, conf_color, conf_pct, color
        ));

        // Edge: CPU → Block
        dot.push_str(&format!(
            "  cpu -> {} [label=\"MMIO\", style=dashed, tooltip=\"MMIO range: 0x{:08x}-0x{:08x}\"];\n",
            sanitize_dot_id(&block.id), block.base_address, block.base_address + block.size
        ));

        // Register sub-nodes
        for reg in &block.registers {
            if block.registers.len() > 20 { break; } // avoid clutter
            let reg_id = format!("{}_{:x}", sanitize_dot_id(&block.id), reg.offset);
            let reg_name = reg.name.as_deref().unwrap_or("reg");
            let acc = match reg.access {
                types::AccessType::Read => "R",
                types::AccessType::Write => "W",
                types::AccessType::ReadWrite => "RW",
                _ => "?",
            };
            dot.push_str(&format!(
                "  {} [label=\"{}\\n+0x{:x} ({})\", shape=note, style=filled, fillcolor=\"#e0e0e0\", fontsize=9];\n",
                reg_id, reg_name, reg.offset, acc
            ));
            dot.push_str(&format!(
                "  {} -> {} [arrowhead=none, style=dotted];\n",
                sanitize_dot_id(&block.id), reg_id
            ));
        }

        // Interrupt edge
        if block.kind == types::BlockKind::InterruptController {
            dot.push_str(&format!(
                "  {} -> cpu [label=\"IRQ\", style=dashed, color=\"#fcc419\", penwidth=2];\n",
                sanitize_dot_id(&block.id)
            ));
        }
    }

    // Legend
    dot.push_str("\n  // Confidence legend\n");
    dot.push_str("  legend_high [label=\"≥70%\", shape=box, style=filled, fillcolor=\"#69db7c\", fontsize=9];\n");
    dot.push_str("  legend_mid [label=\"40-70%\", shape=box, style=filled, fillcolor=\"#ffd43b\", fontsize=9];\n");
    dot.push_str("  legend_low [label=\"<40%\", shape=box, style=filled, fillcolor=\"#ff6b6b\", fontsize=9];\n");

    dot.push_str(&format!("\n  // Overall confidence: {:.1}%\n", spec.confidence * 100.0));
    dot.push_str("}\n");
    dot
}

/// Gera o Event Graph causal (eventos e timing entre blocos)
pub fn generate_event_dot(spec: &HardwareSpec, title: &str) -> String {
    let mut dot = String::new();
    dot.push_str("// B.A.S.E. Event Graph — Causal & Temporal\n");
    dot.push_str(&format!("digraph \"{}\" {{\n", escape_dot(title)));
    dot.push_str("  rankdir=TB;\n");
    dot.push_str("  splines=curved;\n");
    dot.push_str("  node [shape=box, style=rounded, fontname=\"JetBrains Mono\"];\n");
    dot.push_str("  edge [fontname=\"JetBrains Mono\", fontsize=10];\n\n");
    dot.push_str("  label=\"Event Graph — who does what (and when)\";\n");
    dot.push_str("  labelloc=t; fontsize=20; fontcolor=\"#da77f2\";\n\n");

    let mut event_counter = 0u64;

    for block in &spec.blocks {
        let block_id = sanitize_dot_id(&block.id);
        let bc = block_color(&block.kind);

        // Block header
        dot.push_str(&format!(
            "  {} [label=<{} @ 0x{:08x}>, shape=box, style=filled, fillcolor=\"{}\", fontcolor=\"#fff\"];\n",
            block_id, block_name(&block.kind), block.base_address, bc
        ));

        // Events inferred from protocol/transitions
        if !block.protocol.states.is_empty() {
            let prev_states = &block.protocol.states;
            for (i, state) in prev_states.iter().enumerate() {
                if i + 1 < prev_states.len() {
                    event_counter += 1;
                    let event_id = format!("ev_{}", event_counter);
                    let next_state = &prev_states[i + 1];
                    dot.push_str(&format!(
                        "  {} [label=\"{}\\n→ {}\", shape=diamond, style=filled, fillcolor=\"#da77f2\", fontcolor=\"#fff\", fontsize=9];\n",
                        event_id, state, next_state
                    ));
                    dot.push_str(&format!("  {} -> {} [color=\"#da77f2\"];\n", block_id, event_id));
                }
            }
        }

        // Timing info
        if let Some(ref act) = block.timing.activation {
            event_counter += 1;
            let tid = format!("tim_{}", event_counter);
            dot.push_str(&format!(
                "  {} [label=\"⏱ activation\\n{}ns..{}ns\", shape=note, style=filled, fillcolor=\"#fff3bf\", fontsize=9];\n",
                tid, act.min_ns, act.max_ns
            ));
            dot.push_str(&format!("  {} -> {} [style=dotted, color=\"#fab005\"];\n", block_id, tid));
        }

        // IRQ events
        for irq in &spec.interrupts {
            if irq.owner == block.id {
                event_counter += 1;
                let irq_id = format!("irq_{}", event_counter);
                dot.push_str(&format!(
                    "  {} [label=\"⚡ IRQ{}\\n{}\", shape=triangle, style=filled, fillcolor=\"#fcc419\", fontsize=9];\n",
                    irq_id, irq.vector, if irq.irq_type == types::IrqType::Edge { "edge" } else { "level" }
                ));
                dot.push_str(&format!(
                    "  {} -> {} [color=\"#fcc419\", penwidth=2];\n", block_id, irq_id
                ));
                dot.push_str("  irq_cpu [label=\"CPU\", shape=box, style=filled, fillcolor=\"#4a9eff\", fontcolor=\"#fff\", fontsize=9];\n");
                dot.push_str(&format!("  {} -> irq_cpu [color=\"#fcc419\", style=dashed];\n", irq_id));
            }
        }

        // DMA events if dma requirement exists
        if let Some(ref dma) = block.dma {
            event_counter += 1;
            let dma_id = format!("dma_{}", event_counter);
            dot.push_str(&format!(
                "  {} [label=\"📦 DMA\\n{}ch @ {}MB/s\", shape=box3d, style=filled, fillcolor=\"#da77f2\", fontcolor=\"#fff\", fontsize=9];\n",
                dma_id, dma.max_channels, dma.min_bandwidth_mbps
            ));
            dot.push_str(&format!("  {} -> {} [color=\"#da77f2\"];\n", block_id, dma_id));
        }
    }

    // Legend
    dot.push_str("\n  // Event types\n");
    dot.push_str("  leg_state [label=\"State Transition\", shape=diamond, style=filled, fillcolor=\"#da77f2\", fontsize=9];\n");
    dot.push_str("  leg_irq [label=\"Interrupt\", shape=triangle, style=filled, fillcolor=\"#fcc419\", fontsize=9];\n");
    dot.push_str("  leg_timing [label=\"Timing\", shape=note, style=filled, fillcolor=\"#fff3bf\", fontsize=9];\n");
    dot.push_str("  leg_dma [label=\"DMA\", shape=box3d, style=filled, fillcolor=\"#da77f2\", fontsize=9];\n");

    dot.push_str("}\n");
    dot
}

/// Gera ambos os grafos e retorna como tupla (behavior_dot, event_dot)
pub fn generate_all(spec: &HardwareSpec, title: &str) -> (String, String) {
    (generate_behavior_dot(spec, title), generate_event_dot(spec, title))
}

// ─── Old API (backward compat) ─────────────────────────

pub fn generate_dot(spec: &HardwareSpec, title: &str) -> String {
    generate_behavior_dot(spec, title)
}

// ─── Helpers ───────────────────────────────────────────

fn block_color(kind: &types::BlockKind) -> &'static str {
    match kind {
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
        types::BlockKind::MemoryController => "#69db7c",
        types::BlockKind::Crypto => "#e599f7",
        types::BlockKind::VideoCodec => "#ff8787",
        types::BlockKind::Isp => "#66d9e8",
        types::BlockKind::Npu => "#b197fc",
        types::BlockKind::Unknown => "#adb5bd",
    }
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
    fn test_generate_behavior_dot_empty() {
        let spec = types::HardwareSpec::empty();
        let dot = generate_behavior_dot(&spec, "test");
        assert!(dot.contains("Behavior Graph"));
        assert!(dot.contains("CPU"));
        assert!(dot.contains("Memory"));
    }

    #[test]
    fn test_generate_event_dot_empty() {
        let spec = types::HardwareSpec::empty();
        let dot = generate_event_dot(&spec, "test");
        assert!(dot.contains("Event Graph"));
    }

    #[test]
    fn test_generate_all() {
        let mut spec = types::HardwareSpec::empty();
        spec.blocks.push(types::FunctionalBlock {
            id: "gpu_0".into(), kind: types::BlockKind::Gpu,
            base_address: 0x10000000, size: 0x1000,
            registers: vec![],
            protocol: types::Protocol { states: vec!["idle".into(), "active".into(), "done".into()], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: types::TimingProfile {
                activation: Some(types::LatencyRange::new(100, 500)),
                processing: None, interrupt_response: None, dma_setup: None, polling_interval: None,
            },
            dma: Some(types::DmaRequirement { required: true, min_bandwidth_mbps: 400.0, alignment: 256, max_channels: 2 }),
            dependencies: vec![], confidence: 0.8,
        });
        spec.interrupts.push(types::InterruptSpec { vector: 16, owner: "gpu_0".into(), irq_type: types::IrqType::Edge, polarity: types::IrqPolarity::High });
        let (beh, ev) = generate_all(&spec, "GPU Test");
        assert!(beh.contains("GPU"));
        assert!(beh.contains("conf: 80%"));
        assert!(ev.contains("State Transition"));
        assert!(ev.contains("IRQ"));
        assert!(ev.contains("DMA"));
        assert!(ev.contains("Timing"));
    }
}
