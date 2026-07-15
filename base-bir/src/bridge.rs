//! Conversão BIR → contratos temporais do base-core (formato SequenceContract YAML-compat).
//!
//! Event types são mapeados para o vocabulário Saleae usado pelo replay:
//! `mmio_write` / `mmio_read` / `irq`.
use crate::types::*;
use serde::{Deserialize, Serialize};

/// Espelho serializável do SequenceContract de base-core (evita dependência circular).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalSequenceContract {
    pub name: String,
    pub steps: Vec<TemporalEventStep>,
    pub max_total_ns: u64,
    pub max_step_ns: u64,
    pub order: TemporalOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalEventStep {
    pub event_type: String,
    pub address: Option<u64>,
    pub value: Option<u64>,
    pub tolerance_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemporalOrder {
    Strict,
    Relaxed,
    Any,
}

/// Extrai SequenceContracts a partir dos contratos/eventos BIR.
pub fn bir_to_sequence_contracts(device: &BirDevice) -> Vec<TemporalSequenceContract> {
    let mut out = Vec::new();

    for (ci, contract) in device.contracts.iter().enumerate() {
        for (oi, order) in contract.must_occur_before.iter().enumerate() {
            let max_delta = order
                .max_delta_ns
                .or(contract.window_ns)
                .unwrap_or(5_000);
            out.push(TemporalSequenceContract {
                name: format!("{}_causal_{}_{}", device.name, ci, oi),
                steps: vec![
                    event_step(device, &order.event_a, 100),
                    event_step(device, &order.event_b, 100),
                ],
                max_total_ns: max_delta,
                max_step_ns: max_delta,
                order: TemporalOrder::Strict,
            });
        }

        // Pairwise must_occur_before only — não inventar cadeias a→b→c a partir de
        // arestas independentes (gera padrões impossíveis no replay).

        for (li, lat) in contract.latency.iter().enumerate() {
            let _ = (li, lat); // latências BIR tipadas ficam para HIL/traces tipados
        }

        if let Some(window) = contract.window_ns {
            if contract.must_occur_before.is_empty() && !device.events.is_empty() {
                let steps: Vec<_> = device
                    .events
                    .iter()
                    .map(|ev| event_step(device, &ev.name, 100))
                    .collect();
                if steps.len() >= 2 {
                    out.push(TemporalSequenceContract {
                        name: format!("{}_window", device.name),
                        steps,
                        max_total_ns: window,
                        max_step_ns: window,
                        order: TemporalOrder::Strict,
                    });
                }
            }
        }
    }

    if out.is_empty() && device.events.len() >= 2 {
        let steps: Vec<_> = device
            .events
            .iter()
            .map(|ev| event_step(device, &ev.name, 100))
            .collect();
        let max_ns = device
            .timing
            .iter()
            .map(|t| t.latency.max_ns)
            .max()
            .unwrap_or(10_000);
        out.push(TemporalSequenceContract {
            name: format!("{}_event_chain", device.name),
            steps,
            max_total_ns: max_ns,
            max_step_ns: max_ns,
            order: TemporalOrder::Strict,
        });
    }

    out
}

fn event_step(device: &BirDevice, name: &str, tolerance_ns: u64) -> TemporalEventStep {
    TemporalEventStep {
        event_type: saleae_event_type(device, name),
        address: resolve_event_address(device, name),
        value: resolve_event_value(device, name),
        tolerance_ns,
    }
}

/// Mapeia nome BIR → tipo Saleae do replay engine.
pub fn saleae_event_type(device: &BirDevice, event_name: &str) -> String {
    if matches!(event_name, "mmio_write" | "mmio_read" | "irq") {
        return event_name.to_string();
    }
    if device.interrupts.iter().any(|i| i.name == event_name) {
        return "irq".into();
    }
    if event_name.to_ascii_lowercase().contains("irq") {
        return "irq".into();
    }
    if let Some(ev) = device.events.iter().find(|e| e.name == event_name) {
        return match ev.trigger.kind {
            TriggerKind::Write | TriggerKind::WriteBit => "mmio_write".into(),
            TriggerKind::Read | TriggerKind::ReadBit => "mmio_read".into(),
            TriggerKind::AnyAccess => "mmio_write".into(),
        };
    }
    event_name.to_string()
}

fn resolve_event_address(device: &BirDevice, event_name: &str) -> Option<u64> {
    if let Some(irq) = device.interrupts.iter().find(|i| i.name == event_name) {
        return Some(irq.vector as u64);
    }
    if let Some(ev) = device.events.iter().find(|e| e.name == event_name) {
        if let Some(reg) = device
            .registers
            .iter()
            .find(|r| r.name == ev.trigger.register)
        {
            return Some(device.base_address.unwrap_or(0) + reg.offset as u64);
        }
    }
    if let Some(reg) = device.registers.iter().find(|r| r.name == event_name) {
        return Some(device.base_address.unwrap_or(0) + reg.offset as u64);
    }
    None
}

fn resolve_event_value(device: &BirDevice, event_name: &str) -> Option<u64> {
    let ev = device.events.iter().find(|e| e.name == event_name)?;
    // Reads/IRQ rarely carry data no Saleae; só escreve o value de write no contrato
    match ev.trigger.kind {
        TriggerKind::Write | TriggerKind::WriteBit => ev.trigger.value,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_uart() -> BirDevice {
        let mut d = BirDevice::new("UART");
        d.base_address = Some(0x40034000);
        d.registers.push(BirRegister {
            name: "DR".into(),
            offset: 0,
            access: BirAccess::ReadWrite,
            width: 32,
            reset_value: None,
            bitfields: vec![],
        });
        d.registers.push(BirRegister {
            name: "FR".into(),
            offset: 4,
            access: BirAccess::Read,
            width: 32,
            reset_value: None,
            bitfields: vec![],
        });
        d.events.push(BirEvent {
            name: "TX".into(),
            trigger: BirTrigger {
                kind: TriggerKind::Write,
                register: "DR".into(),
                bit_range: None,
                value: Some(0x41),
            },
            timing: None,
        });
        d.events.push(BirEvent {
            name: "STATUS".into(),
            trigger: BirTrigger {
                kind: TriggerKind::Read,
                register: "FR".into(),
                bit_range: None,
                value: None,
            },
            timing: None,
        });
        d.interrupts.push(BirInterrupt {
            name: "UART_IRQ".into(),
            vector: 0x10,
            irq_type: IrqType::Level,
            polarity: IrqPolarity::High,
        });
        d.contracts.push(BirContract {
            must_occur_before: vec![
                CausalOrder {
                    event_a: "TX".into(),
                    event_b: "UART_IRQ".into(),
                    max_delta_ns: Some(2000),
                },
            ],
            latency: vec![],
            window_ns: Some(5000),
            jitter_ns: None,
            repetition_rate: None,
        });
        d
    }

    #[test]
    fn test_bir_to_contracts_saleae_types() {
        let contracts = bir_to_sequence_contracts(&sample_uart());
        assert!(!contracts.is_empty());
        let c = &contracts[0];
        assert_eq!(c.steps[0].event_type, "mmio_write");
        assert_eq!(c.steps[1].event_type, "irq");
        assert_eq!(c.steps[0].address, Some(0x40034000));
        assert_eq!(c.steps[1].address, Some(0x10));
    }

    #[test]
    fn test_bir_to_contracts_from_causal() {
        let mut d = BirDevice::new("dma");
        d.base_address = Some(0x1000);
        d.events.push(BirEvent {
            name: "write".into(),
            trigger: BirTrigger {
                kind: TriggerKind::Write,
                register: "CTRL".into(),
                bit_range: None,
                value: None,
            },
            timing: None,
        });
        d.events.push(BirEvent {
            name: "done".into(),
            trigger: BirTrigger {
                kind: TriggerKind::Read,
                register: "STATUS".into(),
                bit_range: None,
                value: None,
            },
            timing: None,
        });
        d.registers.push(BirRegister {
            name: "CTRL".into(),
            offset: 0,
            access: BirAccess::Write,
            width: 32,
            reset_value: None,
            bitfields: vec![],
        });
        d.registers.push(BirRegister {
            name: "STATUS".into(),
            offset: 4,
            access: BirAccess::Read,
            width: 32,
            reset_value: None,
            bitfields: vec![],
        });
        d.contracts.push(BirContract {
            must_occur_before: vec![CausalOrder {
                event_a: "write".into(),
                event_b: "done".into(),
                max_delta_ns: Some(5000),
            }],
            latency: vec![],
            window_ns: None,
            jitter_ns: None,
            repetition_rate: None,
        });

        let contracts = bir_to_sequence_contracts(&d);
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].steps[0].event_type, "mmio_write");
        assert_eq!(contracts[0].steps[1].event_type, "mmio_read");
    }
}
