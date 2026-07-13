/// Event Graph — grafo causal de eventos com latências.
///
/// Diferente do Behavior Graph (estrutural: CPU→MMIO→GPU),
/// o Event Graph mostra causalidade temporal:
///   WRITE(0x1000) ──150ns──► DMA_START ──2.3µs──► DMA_COMPLETE ──200ns──► IRQ
use crate::temporal::{SequenceContract, TraceEvent, TemporalVerifier};

#[derive(Debug, Clone)]
pub struct EventGraph {
    pub title: String,
    pub sequences: Vec<EventSequence>,
}

#[derive(Debug, Clone)]
pub struct EventSequence {
    pub name: String,
    pub steps: Vec<EventNode>,
    pub edges: Vec<CausalEdge>,
}

#[derive(Debug, Clone)]
pub struct EventNode {
    pub label: String,
    pub kind: String,
    pub timestamp_ns: u64,
    pub address: Option<u64>,
    pub value: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct CausalEdge {
    pub from: usize,
    pub to: usize,
    pub latency_ns: u64,
}

impl EventGraph {
    pub fn new(title: &str) -> Self {
        Self { title: title.to_string(), sequences: Vec::new() }
    }

    /// Constrói grafo causal a partir de contratos + trace
    pub fn from_trace(contracts: &[SequenceContract], events: &[TraceEvent], title: &str) -> Self {
        let mut graph = Self::new(title);

        for contract in contracts {
            let occurrences = TemporalVerifier::find_patterns(events, &contract.steps);
            for (i, occ) in occurrences.iter().enumerate() {
                let name = format!("{}#{}", contract.name, i + 1);
                let mut steps = Vec::new();
                let mut edges = Vec::new();

                for (j, event) in occ.iter().enumerate() {
                    steps.push(EventNode {
                        label: Self::label_for(event),
                        kind: event.event_type.clone(),
                        timestamp_ns: event.timestamp_ns,
                        address: if event.address > 0 { Some(event.address) } else { None },
                        value: event.value,
                    });
                    if j > 0 {
                        let dt = event.timestamp_ns.saturating_sub(occ[j - 1].timestamp_ns);
                        edges.push(CausalEdge { from: j - 1, to: j, latency_ns: dt });
                    }
                }

                graph.sequences.push(EventSequence { name, steps, edges });
            }
        }

        graph
    }

    fn label_for(event: &TraceEvent) -> String {
        let addr = if event.address > 0 { format!(" 0x{:x}", event.address) } else { String::new() };
        let val = event.value.map(|v| format!(" = {}", v)).unwrap_or_default();
        match event.event_type.as_str() {
            "mmio_write" => format!("WRITE{}{}", addr, val),
            "mmio_read" => format!("READ{}", addr),
            "dma_start" => "DMA START".into(),
            "dma_complete" => "DMA COMPLETE".into(),
            "irq" => format!("IRQ{}", addr),
            _ => format!("{}{}", event.event_type, addr),
        }
    }

    fn kind_style(kind: &str) -> (&'static str, &'static str) {
        match kind {
            "mmio_write" => ("box", "#ff6b6b"),
            "mmio_read" => ("box", "#ff8787"),
            "dma_start" => ("diamond", "#da77f2"),
            "dma_complete" => ("diamond", "#9775fa"),
            "irq" => ("triangle", "#fcc419"),
            _ => ("box", "#adb5bd"),
        }
    }

    /// Exporta como Graphviz DOT
    pub fn to_dot(&self) -> String {
        let mut dot = String::new();
        dot.push_str("// B.A.S.E. Event Graph — Causal & Temporal\n");
        dot.push_str(&format!("digraph \"{}\" {{\n", self.title));
        dot.push_str("  rankdir=TB;\n  splines=curved;\n");
        dot.push_str("  node [fontname=\"JetBrains Mono\", fontsize=10];\n");
        dot.push_str("  edge [fontname=\"JetBrains Mono\", fontsize=9];\n\n");

        for seq in &self.sequences {
            dot.push_str(&format!("  subgraph cluster_{} {{\n", sanitize(&seq.name)));
            dot.push_str(&format!("    label=\"{}\";\n", seq.name));
            dot.push_str("    style=filled; fillcolor=\"#1a1a2e\"; fontcolor=\"#4a9eff\";\n");

            for (i, node) in seq.steps.iter().enumerate() {
                let nid = format!("{}_{}", sanitize(&seq.name), i);
                let (shape, color) = Self::kind_style(&node.kind);

                let addr_str = node.address.map(|a| format!("\\n0x{:x}", a)).unwrap_or_default();
                let val_str = node.value.map(|v| format!(" = {}", v)).unwrap_or_default();
                let ts_str = if node.timestamp_ns > 0 { format!("\\n{}ns", node.timestamp_ns) } else { String::new() };

                dot.push_str(&format!(
                    "    {} [label=\"{}{}{}{}\", shape={}, style=filled, fillcolor=\"{}\", fontcolor=\"#fff\"];\n",
                    nid, node.label, addr_str, val_str, ts_str, shape, color
                ));
            }

            for edge in &seq.edges {
                let from = format!("{}_{}", sanitize(&seq.name), edge.from);
                let to = format!("{}_{}", sanitize(&seq.name), edge.to);
                let lat = Self::format_latency(edge.latency_ns);
                dot.push_str(&format!("    {} -> {} [label=\"{}\", color=\"#fab005\", penwidth=2];\n", from, to, lat));
            }

            dot.push_str("  }\n\n");
        }

        dot.push_str("  // Legend\n");
        dot.push_str("  leg_write [label=\"MMIO Write\", shape=box, style=filled, fillcolor=\"#ff6b6b\", fontcolor=\"#fff\", fontsize=9];\n");
        dot.push_str("  leg_dma [label=\"DMA Event\", shape=diamond, style=filled, fillcolor=\"#da77f2\", fontcolor=\"#fff\", fontsize=9];\n");
        dot.push_str("  leg_irq [label=\"Interrupt\", shape=triangle, style=filled, fillcolor=\"#fcc419\", fontcolor=\"#fff\", fontsize=9];\n");
        dot.push_str("  leg_lat [label=\"Latency\", shape=plaintext, fontsize=9];\n");

        dot.push_str("}\n");
        dot
    }

    /// Exporta como Mermaid (para docs/markdown)
    pub fn to_mermaid(&self) -> String {
        let mut s = String::new();
        s.push_str("flowchart LR\n");

        for seq in &self.sequences {
            for (i, node) in seq.steps.iter().enumerate() {
                let nid = format!("ev{}_{}", sanitize(&seq.name), i);
                let kind = node.kind.as_str();
                let (open, close) = match kind {
                    "irq" => ("[/", "/]"),
                    "dma_start" | "dma_complete" => ("{", "}"),
                    _ => ("[", "]"),
                };
                let label = format!("{}, {}ns", node.label, node.timestamp_ns);
                s.push_str(&format!("    {}{}{}{}\n", nid, open, label, close));
            }

            for edge in &seq.edges {
                let from = format!("ev{}_{}", sanitize(&seq.name), edge.from);
                let to = format!("ev{}_{}", sanitize(&seq.name), edge.to);
                let lat = Self::format_latency(edge.latency_ns);
                s.push_str(&format!("    {} -->|{}| {}\n", from, lat, to));
            }
        }

        s
    }

    fn format_latency(ns: u64) -> String {
        if ns >= 1_000_000 {
            format!("{:.1}ms", ns as f64 / 1_000_000.0)
        } else if ns >= 1_000 {
            format!("{:.1}µs", ns as f64 / 1_000.0)
        } else {
            format!("{}ns", ns)
        }
    }
}

fn sanitize(s: &str) -> String {
    s.replace(|c: char| !c.is_alphanumeric(), "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::*;

    fn sample_events() -> Vec<TraceEvent> {
        vec![
            TraceEvent { timestamp_ns: 0, event_type: "mmio_write".into(), address: 0xa9bf0000, value: Some(1) },
            TraceEvent { timestamp_ns: 150, event_type: "dma_start".into(), address: 0, value: None },
            TraceEvent { timestamp_ns: 2450, event_type: "dma_complete".into(), address: 0, value: None },
            TraceEvent { timestamp_ns: 2650, event_type: "irq".into(), address: 16, value: None },
        ]
    }

    fn sample_contract() -> SequenceContract {
        SequenceContract {
            name: "dma_xfer".into(),
            steps: vec![
                EventStep { event_type: "mmio_write".into(), address: Some(0xa9bf0000), value: Some(1), tolerance_ns: 50 },
                EventStep { event_type: "dma_start".into(), address: None, value: None, tolerance_ns: 200 },
                EventStep { event_type: "dma_complete".into(), address: None, value: None, tolerance_ns: 200 },
                EventStep { event_type: "irq".into(), address: Some(16), value: None, tolerance_ns: 100 },
            ],
            max_total_ns: 5000, max_step_ns: 3000, order: OrderConstraint::Strict,
        }
    }

    #[test]
    fn test_event_graph_from_trace() {
        let events = sample_events();
        let contract = sample_contract();
        let graph = EventGraph::from_trace(&[contract], &events, "Test");
        assert!(!graph.sequences.is_empty());
        assert_eq!(graph.sequences[0].steps.len(), 4);
        assert_eq!(graph.sequences[0].edges.len(), 3);
    }

    #[test]
    fn test_dot_export() {
        let events = sample_events();
        let contract = sample_contract();
        let graph = EventGraph::from_trace(&[contract], &events, "Test");
        let dot = graph.to_dot();
        assert!(dot.contains("WRITE"));
        assert!(dot.contains("DMA START"));
        assert!(dot.contains("DMA COMPLETE"));
        assert!(dot.contains("IRQ"));
        assert!(dot.contains("150ns"));
        assert!(dot.contains("2.3") || dot.contains("2300"));
    }

    #[test]
    fn test_mermaid_export() {
        let events = sample_events();
        let contract = sample_contract();
        let graph = EventGraph::from_trace(&[contract], &events, "Test");
        let mermaid = graph.to_mermaid();
        assert!(mermaid.contains("flowchart"));
        assert!(mermaid.contains("WRITE"));
        assert!(mermaid.contains("IRQ"));
    }

    #[test]
    fn test_format_latency() {
        assert_eq!(EventGraph::format_latency(150), "150ns");
        assert_eq!(EventGraph::format_latency(2300), "2.3µs");
        assert_eq!(EventGraph::format_latency(1500000), "1.5ms");
    }

    #[test]
    fn test_label_for() {
        let e = TraceEvent { timestamp_ns: 0, event_type: "mmio_write".into(), address: 0x1000, value: Some(1) };
        assert_eq!(EventGraph::label_for(&e), "WRITE 0x1000 = 1");

        let irq = TraceEvent { timestamp_ns: 0, event_type: "irq".into(), address: 16, value: None };
        assert_eq!(EventGraph::label_for(&irq), "IRQ 0x10");
    }

    #[test]
    fn test_empty_graph() {
        let graph = EventGraph::new("empty");
        let dot = graph.to_dot();
        assert!(dot.contains("empty"));
    }
}
