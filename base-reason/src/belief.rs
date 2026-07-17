//! Sparse belief graph — concepts + confidence (no dense W×X).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeliefRelation {
    Causes,
    Contradicts,
    Supports,
    Inherits,
    SameAs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefNode {
    pub id: String,
    pub concept: String,
    /// 0..=10000 (u16 scale; 10000 = full confidence)
    pub confidence: u16,
    pub seen_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefEdge {
    pub from: String,
    pub to: String,
    pub relation: BeliefRelation,
    pub weight: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeliefGraph {
    pub nodes: HashMap<String, BeliefNode>,
    pub edges: Vec<BeliefEdge>,
}

impl BeliefGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert_node(&mut self, id: impl Into<String>, concept: impl Into<String>, confidence: u16) {
        let id = id.into();
        let concept = concept.into();
        self.nodes
            .entry(id.clone())
            .and_modify(|n| {
                n.confidence = confidence;
                n.seen_count = n.seen_count.saturating_add(1);
                n.concept = concept.clone();
            })
            .or_insert(BeliefNode {
                id,
                concept,
                confidence,
                seen_count: 1,
            });
    }

    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>, relation: BeliefRelation, weight: u16) {
        self.edges.push(BeliefEdge {
            from: from.into(),
            to: to.into(),
            relation,
            weight,
        });
    }

    pub fn strengthen(&mut self, id: &str, delta: u16) {
        if let Some(n) = self.nodes.get_mut(id) {
            n.confidence = n.confidence.saturating_add(delta).min(10_000);
            n.seen_count = n.seen_count.saturating_add(1);
        }
    }

    /// Decay nodes not reinforced; remove if confidence hits 0.
    pub fn forget_weak(&mut self, floor: u16, decay: u16) {
        let mut drop_ids = Vec::new();
        for (id, n) in self.nodes.iter_mut() {
            if n.confidence <= floor {
                n.confidence = n.confidence.saturating_sub(decay);
            }
            if n.confidence == 0 {
                drop_ids.push(id.clone());
            }
        }
        for id in &drop_ids {
            self.nodes.remove(id);
        }
        self.edges
            .retain(|e| self.nodes.contains_key(&e.from) && self.nodes.contains_key(&e.to));
    }
}
