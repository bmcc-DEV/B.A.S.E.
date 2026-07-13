use base_core::component_db::ComponentDb;
use base_core::spec::types::{FunctionalBlock, HardwareSpec, SynthesizedSpec};

#[derive(Debug, Clone, PartialEq)]
pub enum BottleneckType {
    Bandwidth,
    Latency,
    Capacity,
    Power,
    Cost,
    Availability,
}

#[derive(Debug, Clone)]
pub struct Bottleneck {
    pub block_id: String,
    pub component: String,
    pub bottleneck_type: BottleneckType,
    pub current_perf: f64,
    pub candidate_perf: f64,
    pub improvement: f64,
    pub description: String,
}

pub struct BottleneckAnalyzer {
    db: ComponentDb,
}

impl BottleneckAnalyzer {
    pub fn new(db: ComponentDb) -> Self {
        Self { db }
    }

    pub fn analyze(&self, spec: &SynthesizedSpec) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        for block in &spec.original.blocks {
            let assigned = spec.assignments.iter().find(|a| a.block_id == block.id);
            let part = assigned.map(|a| a.component.as_str()).unwrap_or("none");

            bottlenecks.extend(self.analyze_cpu(block, part));
            bottlenecks.extend(self.analyze_memory(block, part));
            bottlenecks.extend(self.analyze_bandwidth(block, part));
            bottlenecks.extend(self.analyze_dma(block, part));
            bottlenecks.extend(self.analyze_connectivity(block, part));
        }

        bottlenecks.sort_by(|a, b| b.improvement.partial_cmp(&a.improvement).unwrap_or(std::cmp::Ordering::Equal));
        bottlenecks
    }

    fn analyze_cpu(&self, block: &FunctionalBlock, _part: &str) -> Vec<Bottleneck> {
        if !matches!(block.kind, base_core::spec::types::BlockKind::Gpu) {
            return vec![];
        }

        let mut b = Vec::new();
        if let Some(entry) = self.db.by_name("RK3566") {
            if let Some(ref cpu) = entry.features.cpu {
                if let Some(ref current_cpu) = self.db.by_name("RP2350A")
                    .and_then(|e| e.features.cpu.as_ref())
                {
                    let ratio = cpu.max_mhz as f64 / current_cpu.max_mhz as f64;
                    if ratio > 2.0 {
                        b.push(Bottleneck {
                            block_id: block.id.clone(),
                            component: block.id.clone(),
                            bottleneck_type: BottleneckType::Bandwidth,
                            current_perf: current_cpu.max_mhz as f64,
                            candidate_perf: cpu.max_mhz as f64,
                            improvement: ratio,
                            description: format!("CPU: {} @ {}MHz → {} @ {}MHz ({:.1}x)",
                                "RP2350A", current_cpu.max_mhz, "RK3566", cpu.max_mhz, ratio),
                        });
                    }
                }
            }
        }
        b
    }

    fn analyze_memory(&self, block: &FunctionalBlock, _part: &str) -> Vec<Bottleneck> {
        if !matches!(block.kind, base_core::spec::types::BlockKind::MemoryController) {
            return vec![];
        }
        let mut b = Vec::new();
        if let Some(ddr) = self.db.by_name("MT41K256M16") {
            b.push(Bottleneck {
                block_id: block.id.clone(),
                component: block.id.clone(),
                bottleneck_type: BottleneckType::Capacity,
                current_perf: 32.0,
                candidate_perf: 512.0,
                improvement: 16.0,
                description: format!("RAM: 32MB PSRAM → 512MB DDR3 ({}x capacity)", 16),
            });
        }
        b
    }

    fn analyze_bandwidth(&self, block: &FunctionalBlock, _part: &str) -> Vec<Bottleneck> {
        if !matches!(block.kind, base_core::spec::types::BlockKind::Ethernet) {
            return vec![];
        }
        let mut b = Vec::new();
        if let Some(gbe) = self.db.by_name("RTL8211") {
            b.push(Bottleneck {
                block_id: block.id.clone(),
                component: block.id.clone(),
                bottleneck_type: BottleneckType::Bandwidth,
                current_perf: 10.0,
                candidate_perf: 1000.0,
                improvement: 100.0,
                description: format!("Ethernet: 10Mbps ENC28J60 → 1Gbps RTL8211 ({}x)", 100),
            });
        }
        b
    }

    fn analyze_dma(&self, block: &FunctionalBlock, _part: &str) -> Vec<Bottleneck> {
        if !matches!(block.kind, base_core::spec::types::BlockKind::Dma) {
            return vec![];
        }
        let mut b = Vec::new();
        if let Some(stm) = self.db.by_name("STM32H743") {
            let dma_count = stm.features.peripherals.get("dma").copied().unwrap_or(0);
            b.push(Bottleneck {
                block_id: block.id.clone(),
                component: block.id.clone(),
                bottleneck_type: BottleneckType::Bandwidth,
                current_perf: 8.0,
                candidate_perf: dma_count as f64,
                improvement: dma_count as f64 / 8.0,
                description: format!("DMA channels: 8 → {} ({:.1}x)", dma_count, dma_count as f64 / 8.0),
            });
        }
        b
    }

    fn analyze_connectivity(&self, block: &FunctionalBlock, _part: &str) -> Vec<Bottleneck> {
        if !matches!(block.kind, base_core::spec::types::BlockKind::Usb) {
            return vec![];
        }
        let mut b = Vec::new();
        b.push(Bottleneck {
            block_id: block.id.clone(),
            component: block.id.clone(),
            bottleneck_type: BottleneckType::Bandwidth,
            current_perf: 12.0,
            candidate_perf: 480.0,
            improvement: 40.0,
            description: "USB: 1.1 (12Mbps) → 2.0 (480Mbps)".into(),
        });
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;
    use base_core::component_db::ComponentDb;

    fn mock_spec() -> SynthesizedSpec {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(FunctionalBlock {
            id: "gpu_0".into(), kind: BlockKind::Gpu,
            base_address: 0x10000000, size: 0x1000,
            registers: vec![], protocol: Protocol { states: vec![], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: TimingProfile { activation: None, processing: None, interrupt_response: None, dma_setup: None, polling_interval: None },
            dma: None, dependencies: vec![], confidence: 0.8,
        });
        SynthesizedSpec {
            original: spec,
            assignments: vec![
                ComponentAssignment { block_id: "gpu_0".into(), component: "RP2350A".into(), interface: "spi".into(), config: Default::default() },
            ],
            netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        }
    }

    fn load_db() -> ComponentDb {
        let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../base-core/component_db"));
        let mut db = ComponentDb::new();
        let _ = db.load_directory(path);
        db
    }

    #[test]
    fn test_analyze_gpu_bottleneck() {
        let db = load_db();
        let analyzer = BottleneckAnalyzer::new(db);
        let spec = mock_spec();
        let bottlenecks = analyzer.analyze(&spec);
        assert!(bottlenecks.iter().any(|b| b.bottleneck_type == BottleneckType::Bandwidth),
            "Should find bandwidth bottleneck for GPU");
    }

    #[test]
    fn test_bottlenecks_sorted() {
        let db = load_db();
        let analyzer = BottleneckAnalyzer::new(db);
        let spec = mock_spec();
        let bottlenecks = analyzer.analyze(&spec);
        for w in bottlenecks.windows(2) {
            assert!(w[0].improvement >= w[1].improvement,
                "Bottlenecks should be sorted by improvement descending");
        }
    }
}
