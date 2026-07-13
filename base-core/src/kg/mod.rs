/// Knowledge Graph engine — GraphML export, CYPHER queries, relationship reasoning.
///
/// Abandona YAML isolado. Transforma tudo em um grafo navegável.
use std::collections::HashMap;

use crate::spec::types::HardwareSpec;

// ─── Node & Edge Types ────────────────────────────────

#[derive(Debug, Clone)]
pub enum KgNode {
    SoC { name: String },
    Block { id: String, kind: String, base: u64, confidence: f64 },
    Register { name: String, offset: u32, purpose: String },
    Interrupt { name: String, vector: u8 },
    Timing { name: String, min_ns: u64, max_ns: u64 },
}

#[derive(Debug, Clone)]
pub struct KgEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub weight: f64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct KnowledgeGraph {
    pub title: String,
    pub nodes: Vec<KgNode>,
    pub edges: Vec<KgEdge>,
}

impl KnowledgeGraph {
    pub fn new(title: &str) -> Self {
        Self { title: title.to_string(), nodes: Vec::new(), edges: Vec::new() }
    }

    /// Constrói grafo a partir de um HardwareSpec
    pub fn from_spec(spec: &HardwareSpec, title: &str) -> Self {
        let mut kg = Self::new(title);

        // SoC root node
        kg.nodes.push(KgNode::SoC {
            name: format!("{:?} @ {}MHz", spec.cpu.architecture, spec.cpu.clock_mhz),
        });

        for block in &spec.blocks {
            let block_id = block.id.clone();
            kg.nodes.push(KgNode::Block {
                id: block_id.clone(),
                kind: format!("{:?}", block.kind),
                base: block.base_address,
                confidence: block.confidence,
            });
            kg.edges.push(KgEdge {
                from: "SoC".into(), to: block_id.clone(),
                relation: "controls".into(),
                weight: block.confidence,
                metadata: HashMap::new(),
            });

            for reg in &block.registers {
                let reg_id = format!("{}_{:x}", block_id, reg.offset);
                kg.nodes.push(KgNode::Register {
                    name: reg.name.clone().unwrap_or_else(|| format!("REG_{:x}", reg.offset)),
                    offset: reg.offset,
                    purpose: format!("{:?}", reg.purpose),
                });
                kg.edges.push(KgEdge {
                    from: block_id.clone(), to: reg_id,
                    relation: "has_register".into(), weight: 1.0,
                    metadata: HashMap::new(),
                });
            }

            if let Some(ref t) = block.timing.activation {
                let tid = format!("timing_{}", block_id);
                kg.nodes.push(KgNode::Timing {
                    name: format!("activation_{}", block_id),
                    min_ns: t.min_ns, max_ns: t.max_ns,
                });
                kg.edges.push(KgEdge {
                    from: block_id.clone(), to: tid,
                    relation: "has_timing".into(), weight: 1.0,
                    metadata: HashMap::new(),
                });
            }
        }

        for irq in &spec.interrupts {
            let iid = format!("IRQ{}", irq.vector);
            kg.nodes.push(KgNode::Interrupt {
                name: iid.clone(),
                vector: irq.vector,
            });
            kg.edges.push(KgEdge {
                from: irq.owner.clone(), to: iid.clone(),
                relation: "triggers".into(), weight: 1.0,
                metadata: HashMap::new(),
            });
            kg.edges.push(KgEdge {
                from: iid, to: "SoC".into(),
                relation: "interrupts".into(), weight: 1.0,
                metadata: HashMap::new(),
            });
        }

        kg
    }

    // ─── GraphML Export ────────────────────────────────

    pub fn to_graphml(&self) -> String {
        let mut xml = String::new();
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push_str(r#"<graphml xmlns="http://graphml.graphdrawing.org/xmlns" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">"#);
        xml.push_str(r#"<key id="kind" for="node" attr.name="kind" attr.type="string"/>"#);
        xml.push_str(r#"<key id="confidence" for="node" attr.name="confidence" attr.type="double"/>"#);
        xml.push_str(r#"<key id="relation" for="edge" attr.name="relation" attr.type="string"/>"#);
        xml.push_str(&format!("<graph id=\"{}\" edgedefault=\"directed\">", esc_xml(&self.title)));

        for (i, node) in self.nodes.iter().enumerate() {
            let nid = format!("n{}", i);
            match node {
                KgNode::SoC { name: _ } => {
                    xml.push_str(&format!("<node id=\"{}\"><data key=\"kind\">SoC</data><data key=\"confidence\">1.0</data></node>", nid));
                }
                KgNode::Block { id: _, kind, base: _, confidence } => {
                    xml.push_str(&format!("<node id=\"{}\"><data key=\"kind\">{}</data><data key=\"confidence\">{}</data></node>", nid, kind, confidence));
                }
                KgNode::Register { name: _, offset: _, purpose: _ } => {
                    xml.push_str(&format!("<node id=\"{}\"><data key=\"kind\">Register</data><data key=\"confidence\">1.0</data></node>", nid));
                }
                KgNode::Interrupt { name: _, vector: _ } => {
                    xml.push_str(&format!("<node id=\"{}\"><data key=\"kind\">Interrupt</data><data key=\"confidence\">1.0</data></node>", nid));
                }
                KgNode::Timing { name: _, min_ns: _, max_ns: _ } => {
                    xml.push_str(&format!("<node id=\"{}\"><data key=\"kind\">Timing</data><data key=\"confidence\">1.0</data></node>", nid));
                }
            }
        }

        for (i, edge) in self.edges.iter().enumerate() {
            let (src_id, dst_id) = self.resolve_ids(edge);
            if let (Some(s), Some(d)) = (src_id, dst_id) {
                xml.push_str(&format!(
                    "<edge id=\"e{}\" source=\"{}\" target=\"{}\"><data key=\"relation\">{}</data></edge>",
                    i, s, d, edge.relation
                ));
            }
        }

        xml.push_str("</graph></graphml>");
        xml
    }

    // ─── CYPHER Export (Neo4j) ─────────────────────────

    pub fn to_cypher(&self) -> String {
        let mut cql = String::new();
        cql.push_str("// B.A.S.E. Knowledge Graph — Neo4j import\n\n");

        for (i, node) in self.nodes.iter().enumerate() {
            let nid = format!("n{}", i);
            match node {
                KgNode::SoC { name } => {
                    cql.push_str(&format!("CREATE ({}:SoC {{name: '{}'}})\n", nid, esc_cypher(name)));
                }
                KgNode::Block { id, kind, base, confidence } => {
                    cql.push_str(&format!("CREATE ({}:Block {{id: '{}', kind: '{}', base: {}, confidence: {}}})\n",
                        nid, id, kind, base, confidence));
                }
                KgNode::Register { name, offset, purpose } => {
                    cql.push_str(&format!("CREATE ({}:Register {{name: '{}', offset: {}, purpose: '{}'}})\n",
                        nid, name, offset, purpose));
                }
                KgNode::Interrupt { name, vector } => {
                    cql.push_str(&format!("CREATE ({}:Interrupt {{name: '{}', vector: {}}})\n", nid, name, vector));
                }
                KgNode::Timing { name, min_ns, max_ns } => {
                    cql.push_str(&format!("CREATE ({}:Timing {{name: '{}', min_ns: {}, max_ns: {}}})\n", nid, name, min_ns, max_ns));
                }
            }
        }

        cql.push('\n');

        for (_, edge) in self.edges.iter().enumerate() {
            let (src_id, dst_id) = self.resolve_ids(edge);
            if let (Some(s), Some(d)) = (src_id, dst_id) {
                cql.push_str(&format!("MATCH (a) WHERE id(a) = {} MATCH (b) WHERE id(b) = {} CREATE (a)-[:{}]->(b)\n",
                    s, d, edge.relation.to_uppercase()));
            }
        }

        cql
    }

    // ─── Query Engine ──────────────────────────────────

    /// Encontra blocos que disparam um IRQ específico
    pub fn blocks_triggering_irq(&self, vector: u8) -> Vec<String> {
        let irq_nodes: Vec<usize> = self.nodes.iter().enumerate()
            .filter_map(|(i, n)| if let KgNode::Interrupt { vector: v, .. } = n { if *v == vector { Some(i) } else { None } } else { None })
            .collect();

        let mut result = Vec::new();
        for &irq_i in &irq_nodes {
            for edge in &self.edges {
                if edge.relation == "triggers" {
                    if let Some(dst) = self.node_id_to_index(&edge.to) {
                        if dst == irq_i {
                            result.push(edge.from.clone());
                        }
                    }
                }
            }
        }
        result
    }

    /// Encontra o caminho causal entre dois nós
    pub fn causal_path(&self, from_kind: &str, to_kind: &str) -> Vec<String> {
        let mut path = Vec::new();
        for edge in &self.edges {
            let from_matches = self.nodes.iter().any(|n| matches!(n, KgNode::Block { kind: k, .. } if k == from_kind));
            let to_matches = self.nodes.iter().any(|n| matches!(n, KgNode::Block { kind: k, .. } if k == to_kind));
            if from_matches && to_matches {
                path.push(format!("{} --[{}]--> {}", edge.from, edge.relation, edge.to));
            }
        }
        path
    }

    // ─── Helpers ───────────────────────────────────────

    fn resolve_ids(&self, edge: &KgEdge) -> (Option<String>, Option<String>) {
        let src = self.nodes.iter().enumerate()
            .find(|(_, n)| match n {
                KgNode::SoC { .. } => edge.from == "SoC",
                KgNode::Block { id, .. } => *id == edge.from,
                _ => false,
            })
            .map(|(i, _)| format!("n{}", i));

        let dst = self.nodes.iter().enumerate()
            .find(|(_, n)| match n {
                KgNode::SoC { .. } => edge.to == "SoC",
                KgNode::Block { id, .. } => *id == edge.to,
                _ => false,
            })
            .map(|(i, _)| format!("n{}", i));

        (src, dst)
    }

    fn node_id_to_index(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| match n {
            KgNode::Block { id, .. } => id == name,
            KgNode::Interrupt { name: n, .. } => n == name,
            _ => false,
        })
    }
}

fn esc_xml(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn esc_cypher(s: &str) -> String {
    s.replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> HardwareSpec {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(types::FunctionalBlock {
            id: "gpu_0".into(), kind: types::BlockKind::Gpu,
            base_address: 0x10000000, size: 0x1000,
            registers: vec![types::Register {
                offset: 0, name: Some("control".into()), width: 32,
                access: types::AccessType::ReadWrite, purpose: types::RegisterPurpose::Control,
                reset_value: None, observed_values: vec![], bitfields: vec![], polling: false, count: 0,
            }],
            protocol: types::Protocol { states: vec![], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: types::TimingProfile {
                activation: Some(types::LatencyRange::new(100, 500, 300)),
                processing: None, interrupt_response: None, dma_setup: None, polling_interval: None,
            },
            dma: None, dependencies: vec![], confidence: 0.85,
        });
        spec.interrupts.push(types::InterruptSpec {
            vector: 16, owner: "gpu_0".into(), irq_type: types::IrqType::Edge, polarity: types::IrqPolarity::High,
        });
        spec
    }

    #[test]
    fn test_build_graph() {
        let spec = sample_spec();
        let kg = KnowledgeGraph::from_spec(&spec, "test");
        assert!(kg.nodes.len() >= 4); // SoC + Block + Register + Interrupt + Timing
        assert!(!kg.edges.is_empty());
    }

    #[test]
    fn test_graphml_export() {
        let spec = sample_spec();
        let kg = KnowledgeGraph::from_spec(&spec, "test");
        let xml = kg.to_graphml();
        assert!(xml.contains("graphml"));
        assert!(xml.contains("node"));
        assert!(xml.contains("edge"));
    }

    #[test]
    fn test_cypher_export() {
        let spec = sample_spec();
        let kg = KnowledgeGraph::from_spec(&spec, "test");
        let cql = kg.to_cypher();
        assert!(cql.contains("CREATE"));
        assert!(cql.contains("SoC"));
        assert!(cql.contains("Block"));
        assert!(cql.contains("Register"));
    }

    #[test]
    fn test_blocks_triggering_irq() {
        let spec = sample_spec();
        let kg = KnowledgeGraph::from_spec(&spec, "test");
        let blocks = kg.blocks_triggering_irq(16);
        assert!(blocks.iter().any(|b| b == "gpu_0"));
    }

    #[test]
    fn test_causal_path() {
        let spec = sample_spec();
        let kg = KnowledgeGraph::from_spec(&spec, "test");
        let _path = kg.causal_path("Gpu", "Gpu");
    }
}
