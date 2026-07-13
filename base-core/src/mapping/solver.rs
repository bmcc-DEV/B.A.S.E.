use crate::component_db::{ComponentCategory, ComponentEntry};
use crate::spec::types::{FunctionalBlock, HardwareSpec};

#[derive(Debug, Clone)]
pub struct ConstraintSet {
    pub timing: TimingConstraint,
    pub pinout: PinoutConstraint,
    pub power: PowerConstraint,
}

#[derive(Debug, Clone)]
pub struct TimingConstraint {
    pub max_latency_ns: Option<u64>,
    pub min_bandwidth_mbps: Option<f64>,
    pub dma_required: bool,
}

#[derive(Debug, Clone)]
pub struct PinoutConstraint {
    pub min_gpio: u32,
    pub required_interfaces: Vec<String>,
    pub package_preference: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PowerConstraint {
    pub max_watts: f64,
    pub voltage_rails: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SolvedAssignment {
    pub block_id: String,
    pub component: ComponentEntry,
    pub interface: String,
    pub match_score: f64,    // 0.0 — 1.0
    pub constraint_satisfied: bool,
}

/// Extrai constraints de um HardwareSpec para um bloco específico
pub fn extract_constraints(block: &FunctionalBlock, spec: &HardwareSpec) -> ConstraintSet {
    let timing = TimingConstraint {
        max_latency_ns: block.timing.processing.map(|l| l.avg_ns),
        min_bandwidth_mbps: block.dma.as_ref().map(|d| d.min_bandwidth_mbps),
        dma_required: block.dma.as_ref().map_or(false, |d| d.required),
    };

    let pinout = PinoutConstraint {
        min_gpio: match block.kind {
            crate::spec::types::BlockKind::Gpu => 16,
            crate::spec::types::BlockKind::Audio => 4,
            crate::spec::types::BlockKind::Dma => 8,
            crate::spec::types::BlockKind::Usb => 4,
            crate::spec::types::BlockKind::Ethernet => 8,
            crate::spec::types::BlockKind::Spi => 4,
            crate::spec::types::BlockKind::I2c => 2,
            crate::spec::types::BlockKind::Uart => 2,
            _ => 4,
        },
        required_interfaces: match block.kind {
            crate::spec::types::BlockKind::Gpu => vec!["spi".into(), "dma".into()],
            crate::spec::types::BlockKind::Audio => vec!["i2c".into()],
            crate::spec::types::BlockKind::Dma => vec!["dma".into()],
            crate::spec::types::BlockKind::Usb => vec!["usb".into()],
            crate::spec::types::BlockKind::Ethernet => vec!["spi".into()],
            _ => vec![],
        },
        package_preference: None,
    };

    let power = PowerConstraint {
        max_watts: spec.constraints.max_power_watts.max(1.0),
        voltage_rails: vec!["3.3V".into(), "1.8V".into()],
    };

    ConstraintSet { timing, pinout, power }
}

/// Verifica se um componente satisfaz as constraints
pub fn check_constraints(
    component: &ComponentEntry,
    constraints: &ConstraintSet,
) -> SolvedAssignment {
    let mut satisfied = true;
    let mut scores = Vec::new();

    // Timing / Bandwidth (aproximado por clock)
    if let Some(ref cpu) = component.features.cpu {
        if let Some(bw) = constraints.timing.min_bandwidth_mbps {
            let approx_bw = cpu.max_mhz as f64 * 4.0; // estimativa grosseira
            let bw_ok = approx_bw >= bw * 0.5;
            satisfied &= bw_ok;
            scores.push(if bw_ok { 1.0 } else { (approx_bw / bw).min(0.5) });
        }
    }

    // DMA requirement
    if constraints.timing.dma_required {
        let dma_count = component.features.peripherals.get("dma").copied().unwrap_or(0);
        let dma_ok = dma_count > 0;
        satisfied &= dma_ok;
        scores.push(if dma_ok { 1.0 } else { 0.0 });
    }

    // Peripherals
    for iface in &constraints.pinout.required_interfaces {
        let count = component.features.peripherals.get(iface.as_str()).copied().unwrap_or(0);
        let iface_ok = count > 0;
        satisfied &= iface_ok;
        scores.push(if iface_ok { 1.0 } else { 0.0 });
    }

    // GPIO count
    let gpio_count = component.pins.as_ref().map_or(0, |p| p.len() as u32);
    let gpio_ok = gpio_count >= constraints.pinout.min_gpio;
    satisfied &= gpio_ok;
    scores.push(if gpio_ok { 1.0 } else { (gpio_count as f64 / constraints.pinout.min_gpio as f64).min(0.5) });

    let match_score = if scores.is_empty() { 0.5 } else { scores.iter().sum::<f64>() / scores.len() as f64 };

    SolvedAssignment {
        block_id: String::new(),
        component: component.clone(),
        interface: infer_interface(component, &constraints.pinout.required_interfaces),
        match_score,
        constraint_satisfied: satisfied,
    }
}

fn infer_interface(component: &ComponentEntry, required: &[String]) -> String {
    for iface in required {
        if component.features.peripherals.contains_key(iface) {
            return iface.clone();
        }
    }
    match component.category {
        ComponentCategory::Mcu => "spi".into(),
        ComponentCategory::Cpu => "memory_bus".into(),
        ComponentCategory::Connectivity => "spi".into(),
        ComponentCategory::Audio => "i2c".into(),
        _ => "gpio".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component_db::*;
    use crate::spec::types::*;
    use std::collections::HashMap;

    fn mock_block() -> FunctionalBlock {
        FunctionalBlock {
            id: "gpu_0".into(),
            kind: BlockKind::Gpu,
            base_address: 0x10000000,
            size: 0x1000,
            registers: Vec::new(),
            protocol: Protocol { states: vec!["idle".into()], transitions: vec![], entry_condition: None, exit_condition: None },
            timing: TimingProfile { activation: None, processing: Some(LatencyRange::new(1000, 5000, 2000)), interrupt_response: None, dma_setup: None, polling_interval: None },
            dma: Some(DmaRequirement { required: true, min_bandwidth_mbps: 100.0, alignment: 4, max_channels: 2 }),
            dependencies: vec![],
            confidence: 0.8,
        }
    }

    #[test]
    fn test_extract_constraints() {
        let spec = HardwareSpec::empty();
        let block = mock_block();
        let constraints = extract_constraints(&block, &spec);
        assert!(constraints.timing.dma_required);
        assert_eq!(constraints.pinout.min_gpio, 16);
    }
}
