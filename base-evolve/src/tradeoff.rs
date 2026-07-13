use crate::analyzer::{Bottleneck, BottleneckType};
use base_core::component_db::ComponentDb;
use base_core::spec::types::SynthesizedSpec;

#[derive(Debug, Clone)]
pub enum TradeoffComplexity {
    Same,
    SlightlyMore,
    SignificantlyMore,
}

#[derive(Debug, Clone)]
pub enum TradeoffRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct Tradeoff {
    pub component: String,
    pub original: String,
    pub candidate: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub cost_delta: f64,
    pub complexity: TradeoffComplexity,
    pub risk: TradeoffRisk,
    pub estimated_effort_days: u32,
}

/// Analisa trade-offs entre componente atual e candidato
pub struct TradeoffAnalyzer;

impl TradeoffAnalyzer {
    /// Avalia trade-off para um bottleneck específico
    pub fn evaluate(&self, bottleneck: &Bottleneck, _spec: &SynthesizedSpec) -> Tradeoff {
        let (pros, cons, cost, complexity, risk, effort) = match bottleneck.bottleneck_type {
            BottleneckType::Bandwidth => (
                vec![format!("{:.1}x more bandwidth", bottleneck.improvement)],
                vec!["May require PCB respin".into(), "Higher power consumption".into()],
                15.0 + bottleneck.improvement * 5.0,
                TradeoffComplexity::SlightlyMore,
                TradeoffRisk::Low,
                5 + (bottleneck.improvement as u32 / 10).min(14),
            ),
            BottleneckType::Capacity => (
                vec![format!("{:.0}x more capacity", bottleneck.improvement)],
                vec!["Different package/footprint".into(), "More PCB layers needed".into()],
                25.0,
                TradeoffComplexity::SlightlyMore,
                TradeoffRisk::Low,
                7,
            ),
            BottleneckType::Power => (
                vec!["Lower power consumption".into()],
                vec!["May need different regulator".into()],
                -5.0,
                TradeoffComplexity::Same,
                TradeoffRisk::Low,
                3,
            ),
            BottleneckType::Cost => (
                vec!["Lower BOM cost".into()],
                vec!["May have worse availability".into()],
                -bottleneck.improvement * 0.5,
                TradeoffComplexity::Same,
                TradeoffRisk::Medium,
                2,
            ),
            BottleneckType::Availability => (
                vec!["Better supply chain".into()],
                vec!["Premium pricing".into()],
                2.0,
                TradeoffComplexity::Same,
                TradeoffRisk::Low,
                1,
            ),
            BottleneckType::Latency => (
                vec![format!("{:.1}x lower latency", bottleneck.improvement)],
                vec!["Complex timing revalidation needed".into()],
                10.0,
                TradeoffComplexity::SignificantlyMore,
                TradeoffRisk::High,
                14,
            ),
        };

        Tradeoff {
            component: bottleneck.component.clone(),
            original: bottleneck.description.split("→").next().unwrap_or("original").trim().to_string(),
            candidate: bottleneck.description.split("→").nth(1).unwrap_or("candidate").trim().to_string(),
            pros,
            cons,
            cost_delta: cost,
            complexity,
            risk,
            estimated_effort_days: effort,
        }
    }

    /// Avalia múltiplos bottlenecks e retorna trade-offs ordenados por impacto
    pub fn evaluate_all(&self, bottlenecks: &[Bottleneck], spec: &SynthesizedSpec) -> Vec<Tradeoff> {
        let mut tradeoffs: Vec<Tradeoff> = bottlenecks.iter()
            .map(|b| self.evaluate(b, spec))
            .collect();
        tradeoffs.sort_by(|a, b| b.cost_delta.partial_cmp(&a.cost_delta).unwrap_or(std::cmp::Ordering::Equal));
        tradeoffs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{Bottleneck, BottleneckType};
    use base_core::spec::types::*;

    fn mock_bottleneck() -> Bottleneck {
        Bottleneck {
            block_id: "gpu_0".into(),
            component: "GPU".into(),
            bottleneck_type: BottleneckType::Bandwidth,
            current_perf: 150.0,
            candidate_perf: 2000.0,
            improvement: 13.3,
            description: "CPU: RP2350A @ 150MHz → RK3566 @ 2GHz (13.3x)".into(),
        }
    }

    #[test]
    fn test_tradeoff_evaluation() {
        let analyzer = TradeoffAnalyzer;
        let bottleneck = mock_bottleneck();
        let spec = SynthesizedSpec {
            original: HardwareSpec::empty(),
            assignments: vec![],
            netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        };
        let tradeoff = analyzer.evaluate(&bottleneck, &spec);
        assert!(!tradeoff.pros.is_empty());
        assert!(!tradeoff.cons.is_empty());
        assert!(tradeoff.cost_delta > 0.0);
    }

    #[test]
    fn test_evaluate_all_sorted() {
        let analyzer = TradeoffAnalyzer;
        let bottlenecks = vec![
            Bottleneck { block_id: "a".into(), component: "A".into(), bottleneck_type: BottleneckType::Bandwidth, current_perf: 1.0, candidate_perf: 10.0, improvement: 10.0, description: "A".into() },
            Bottleneck { block_id: "b".into(), component: "B".into(), bottleneck_type: BottleneckType::Capacity, current_perf: 1.0, candidate_perf: 2.0, improvement: 2.0, description: "B".into() },
        ];
        let spec = SynthesizedSpec {
            original: HardwareSpec::empty(),
            assignments: vec![], netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        };
        let tradeoffs = analyzer.evaluate_all(&bottlenecks, &spec);
        assert_eq!(tradeoffs.len(), 2);
    }
}
