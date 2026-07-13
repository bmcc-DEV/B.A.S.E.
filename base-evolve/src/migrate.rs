use crate::analyzer::Bottleneck;
use crate::tradeoff::Tradeoff;
use base_core::spec::types::SynthesizedSpec;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MigrationPlan {
    pub title: String,
    pub summary: String,
    pub changes: Vec<MigrationChange>,
    pub estimated_bom_delta: f64,
    pub estimated_effort_days: u32,
    pub migration_steps: Vec<MigrationStep>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationChange {
    pub step: u32,
    pub component: String,
    pub original: String,
    pub replacement: String,
    pub speedup: f64,
    pub bom_delta: f64,
    pub requires_pcb_change: bool,
    pub requires_fw_change: bool,
    pub risk: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationStep {
    pub step: u32,
    pub action: String,
    pub effort: String,
}

pub struct MigrationPlanner;

impl MigrationPlanner {
    /// Gera plano de migração completo a partir de bottlenecks + trade-offs
    pub fn generate_plan(
        &self,
        bottlenecks: &[Bottleneck],
        tradeoffs: &[Tradeoff],
        spec: &SynthesizedSpec,
    ) -> MigrationPlan {
        let total_cost: f64 = tradeoffs.iter().map(|t| t.cost_delta).sum();
        let total_effort: u32 = tradeoffs.iter().map(|t| t.estimated_effort_days).sum();

        let changes: Vec<MigrationChange> = bottlenecks.iter().enumerate().map(|(i, b)| {
            MigrationChange {
                step: (i + 1) as u32,
                component: b.block_id.clone(),
                original: b.description.split("→").next().unwrap_or("").trim().to_string(),
                replacement: b.description.split("→").nth(1).unwrap_or("").trim().to_string(),
                speedup: b.improvement,
                bom_delta: tradeoffs.get(i).map(|t| t.cost_delta).unwrap_or(0.0),
                requires_pcb_change: b.improvement > 5.0,
                requires_fw_change: b.improvement > 10.0,
                risk: if b.improvement > 20.0 { "high" } else if b.improvement > 5.0 { "medium" } else { "low" }.into(),
            }
        }).collect();

        let steps: Vec<MigrationStep> = vec![
            MigrationStep { step: 1, action: "PCB respin — new footprints and routing".into(), effort: format!("{} days", total_effort / 3) },
            MigrationStep { step: 2, action: "HAL update — new register mappings".into(), effort: format!("{} days", total_effort / 4) },
            MigrationStep { step: 3, action: "Firmware update — new drivers".into(), effort: format!("{} days", total_effort / 4) },
            MigrationStep { step: 4, action: "Validation — full trace replay".into(), effort: format!("{} days", total_effort / 6) },
        ];

        let title = format!("Evolution: {} upgrades", changes.len());
        let summary = format!(
            "Replace {} components at estimated BOM cost ${:.2} over ~{} days",
            changes.len(), total_cost, total_effort
        );

        MigrationPlan {
            title,
            summary,
            changes,
            estimated_bom_delta: total_cost,
            estimated_effort_days: total_effort,
            migration_steps: steps,
        }
    }

    /// Serializa o plano como YAML
    pub fn to_yaml(&self, plan: &MigrationPlan) -> String {
        serde_yaml::to_string(plan).unwrap_or_default()
    }

    /// Serializa o plano como Markdown
    pub fn to_markdown(&self, plan: &MigrationPlan) -> String {
        let mut md = String::new();
        md.push_str(&format!("# {}\n\n", plan.title));
        md.push_str(&format!("{}\n\n", plan.summary));
        md.push_str(&format!("**Estimated BOM delta:** ${:.2}\n", plan.estimated_bom_delta));
        md.push_str(&format!("**Estimated effort:** {} days\n\n", plan.estimated_effort_days));
        md.push_str("## Changes\n\n");
        md.push_str("| # | Component | Original → Replacement | Speedup | BOM Δ | Risk |\n");
        md.push_str("|---|-----------|----------------------|---------|-------|------|\n");
        for c in &plan.changes {
            md.push_str(&format!("| {} | {} | {} → {} | {:.1}x | ${:.2} | {} |\n",
                c.step, c.component, c.original, c.replacement, c.speedup, c.bom_delta, c.risk));
        }
        md.push_str("\n## Migration Steps\n\n");
        for s in &plan.migration_steps {
            md.push_str(&format!("### Step {}: {}\n", s.step, s.action));
            md.push_str(&format!("Effort: {}\n\n", s.effort));
        }
        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{Bottleneck, BottleneckType};
    use crate::tradeoff::{Tradeoff, TradeoffComplexity, TradeoffRisk};

    fn mock_bottleneck() -> Bottleneck {
        Bottleneck {
            block_id: "gpu_0".into(), component: "GPU".into(),
            bottleneck_type: BottleneckType::Bandwidth,
            current_perf: 150.0, candidate_perf: 2000.0, improvement: 13.3,
            description: "RP2350A @ 150MHz → RK3566 @ 2GHz (13.3x)".into(),
        }
    }

    #[test]
    fn test_migration_plan() {
        let planner = MigrationPlanner;
        let bottlenecks = vec![mock_bottleneck()];
        let tradeoffs = vec![Tradeoff {
            component: "GPU".into(), original: "RP2350A".into(),
            candidate: "RK3566".into(),
            pros: vec!["13.3x bandwidth".into()],
            cons: vec!["PCB respin needed".into()],
            cost_delta: 45.0, complexity: TradeoffComplexity::SlightlyMore,
            risk: TradeoffRisk::Medium, estimated_effort_days: 10,
        }];
        let spec = base_core::spec::types::SynthesizedSpec {
            original: base_core::spec::types::HardwareSpec::empty(),
            assignments: vec![], netlist: None,
            constraints: base_core::spec::types::SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        };
        let plan = planner.generate_plan(&bottlenecks, &tradeoffs, &spec);
        assert!(plan.title.contains("Evolution"));
        assert!(plan.estimated_bom_delta > 0.0);
    }

    #[test]
    fn test_to_yaml() {
        let planner = MigrationPlanner;
        let plan = MigrationPlan {
            title: "Test".into(), summary: "Upgrade test".into(),
            changes: vec![], estimated_bom_delta: 50.0, estimated_effort_days: 10,
            migration_steps: vec![MigrationStep { step: 1, action: "Test".into(), effort: "5 days".into() }],
        };
        let yaml = planner.to_yaml(&plan);
        assert!(yaml.contains("Test"));
    }

    #[test]
    fn test_to_markdown() {
        let planner = MigrationPlanner;
        let plan = MigrationPlan {
            title: "Upgrade Plan".into(), summary: "Summary here".into(),
            changes: vec![MigrationChange { step: 1, component: "CPU".into(), original: "A".into(), replacement: "B".into(), speedup: 2.0, bom_delta: 10.0, requires_pcb_change: true, requires_fw_change: false, risk: "low".into() }],
            estimated_bom_delta: 10.0, estimated_effort_days: 5,
            migration_steps: vec![MigrationStep { step: 1, action: "Test".into(), effort: "2 days".into() }],
        };
        let md = planner.to_markdown(&plan);
        assert!(md.contains("Upgrade Plan"));
        assert!(md.contains("CPU"));
    }
}
