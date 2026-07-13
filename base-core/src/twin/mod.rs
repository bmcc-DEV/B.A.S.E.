/// Digital Twin — interpretador BIR que simula hardware virtualmente.
///
/// Executa eventos, transições de estado, avança tempo simulado,
/// e valida contratos temporais — tudo antes da PCB existir.
use std::collections::HashMap;
use base_bir::types::*;
use base_bir::contract::{ContractVerifier, TraceSample};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TwinStatus {
    Idle,
    Running,
    Halted,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct DigitalTwin {
    pub device: BirDevice,
    pub registers: HashMap<String, u64>,
    pub time_ns: u64,
    pub trace: Vec<TraceSample>,
    pub status: TwinStatus,
    pub step_count: u64,
}

impl DigitalTwin {
    /// Cria um novo twin a partir de um device BIR
    pub fn new(device: BirDevice) -> Self {
        let mut registers = HashMap::new();
        for reg in &device.registers {
            registers.insert(reg.name.clone(), reg.reset_value.unwrap_or(0));
        }
        Self {
            device,
            registers,
            time_ns: 0,
            trace: Vec::new(),
            status: TwinStatus::Idle,
            step_count: 0,
        }
    }

    /// Escreve em um registrador e dispara eventos
    pub fn write_register(&mut self, name: &str, value: u64) -> Result<(), String> {
        if !self.registers.contains_key(name) {
            return Err(format!("Unknown register: {}", name));
        }
        self.registers.insert(name.to_string(), value);
        self.step_count += 1;
        let ts = self.time_ns;

        let mut events_to_fire = Vec::new();
        for event in &self.device.events {
            if event.trigger.register == name {
                let should_fire = if let Some(bit_range) = &event.trigger.bit_range {
                    let mask = ((1u64 << (bit_range.end - bit_range.start)) - 1) << bit_range.start;
                    let field_val = (value & mask) >> bit_range.start;
                    Some(field_val) == event.trigger.value || event.trigger.value.is_none()
                } else {
                    Some(value) == event.trigger.value || event.trigger.value.is_none()
                };
                if should_fire {
                    events_to_fire.push(event.name.clone());
                }
            }
        }

        self.trace.push(TraceSample {
            timestamp_ns: ts,
            event: format!("write_{}", name),
            value: Some(value),
        });

        for event_name in events_to_fire {
            self.fire_event(&event_name)?;
        }

        Ok(())
    }

    /// Lê um registrador
    pub fn read_register(&self, name: &str) -> Result<u64, String> {
        self.registers.get(name).copied()
            .ok_or_else(|| format!("Unknown register: {}", name))
    }

    /// Dispara um evento manualmente
    pub fn fire_event(&mut self, event: &str) -> Result<(), String> {
        // Avança o tempo simulado pela latência do evento
        for timing in &self.device.timing {
            if timing.name == event {
                self.time_ns += avg_latency_ns(&timing.latency);
                break;
            }
        }
        // Default latency if not specified
        if !self.device.timing.iter().any(|t| t.name == event) {
            self.time_ns += 100; // default 100ns
        }

        self.trace.push(TraceSample {
            timestamp_ns: self.time_ns,
            event: event.to_string(),
            value: None,
        });

        self.status = TwinStatus::Running;
        tracing::info!("[Twin] {} fired at {}ns", event, self.time_ns);
        Ok(())
    }

    /// Simula a sequência de boot completa
    pub fn boot_sequence(&mut self) -> Result<(), String> {
        self.status = TwinStatus::Running;
        tracing::info!("[Twin] Boot start — {}", self.device.name);

        // 1. Reset: clear all registers
        for val in self.registers.values_mut() {
            *val = 0;
        }
        self.time_ns = 0;
        self.trace.push(TraceSample { timestamp_ns: 0, event: "RESET".into(), value: None });

        // 2. Find control register and set it to 1 (wake) - clone name to avoid borrow conflict
        let control_name = self.device.registers.iter()
            .find(|r| r.name.to_lowercase().contains("control") || r.name.to_lowercase().contains("ctrl"))
            .map(|r| r.name.clone());
        if let Some(ref name) = control_name {
            self.write_register(name, 1)?;
        }

        // 3. Fire init events if defined - collect names first
        let init_events: Vec<String> = self.device.events.iter()
            .filter(|e| e.name.to_lowercase().contains("init") || e.name.to_lowercase().contains("start"))
            .map(|e| e.name.clone())
            .collect();
        for event_name in &init_events {
            self.fire_event(event_name)?;
        }

        self.status = TwinStatus::Running;
        tracing::info!("[Twin] Boot complete at {}ns, {} steps", self.time_ns, self.step_count);
        Ok(())
    }

    /// Valida contratos contra o trace gerado pela simulação
    pub fn verify_contracts(&self) -> ContractVerification {
        ContractVerifier::verify(&self.device, &self.trace)
    }

    /// Reinicia o twin
    pub fn reset(&mut self) {
        self.registers.clear();
        for reg in &self.device.registers {
            self.registers.insert(reg.name.clone(), reg.reset_value.unwrap_or(0));
        }
        self.time_ns = 0;
        self.trace.clear();
        self.status = TwinStatus::Idle;
        self.step_count = 0;
    }

    /// Estatísticas da simulação
    pub fn stats(&self) -> TwinStats {
        TwinStats {
            device: self.device.name.clone(),
            time_ns: self.time_ns,
            steps: self.step_count,
            events_fired: self.trace.len(),
            registers_touched: self.registers.len(),
            status: self.status.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TwinStats {
    pub device: String,
    pub time_ns: u64,
    pub steps: u64,
    pub events_fired: usize,
    pub registers_touched: usize,
    pub status: TwinStatus,
}

// ─── Helper ───────────────────────────────────────────

fn avg_latency_ns(range: &BirLatencyRange) -> u64 {
    (range.min_ns + range.max_ns) / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_device() -> BirDevice {
        let mut dev = BirDevice::new("GPU");
        dev.base_address = Some(0x10000000);
        dev.registers.push(BirRegister {
            name: "CONTROL".into(), offset: 0,
            access: BirAccess::ReadWrite, width: 32,
            reset_value: Some(0), bitfields: vec![],
        });
        dev.registers.push(BirRegister {
            name: "STATUS".into(), offset: 4,
            access: BirAccess::Read, width: 32,
            reset_value: None, bitfields: vec![],
        });
        dev.events.push(BirEvent {
            name: "DMA_START".into(),
            trigger: BirTrigger {
                kind: TriggerKind::Write, register: "CONTROL".into(),
                bit_range: Some(0..1), value: Some(1),
            },
            timing: Some(BirLatencyRange::new(100, 400)),
        });
        dev.interrupts.push(BirInterrupt {
            name: "IRQ_GPU".into(), vector: 16,
            irq_type: IrqType::Level, polarity: IrqPolarity::High,
        });
        dev.timing.push(BirTimingEntry {
            name: "DMA_START".into(),
            latency: BirLatencyRange::new(100, 400),
            per_unit: None,
        });
        dev
    }

    #[test]
    fn test_twin_new() {
        let dev = sample_device();
        let twin = DigitalTwin::new(dev);
        assert_eq!(twin.status, TwinStatus::Idle);
        assert!(twin.registers.contains_key("CONTROL"));
    }

    #[test]
    fn test_write_register() {
        let dev = sample_device();
        let mut twin = DigitalTwin::new(dev);
        twin.write_register("CONTROL", 1).unwrap();
        assert_eq!(*twin.registers.get("CONTROL").unwrap(), 1);
        assert_eq!(twin.step_count, 1);
    }

    #[test]
    fn test_write_unknown_register() {
        let dev = sample_device();
        let mut twin = DigitalTwin::new(dev);
        let result = twin.write_register("NONEXISTENT", 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_boot_sequence() {
        let dev = sample_device();
        let mut twin = DigitalTwin::new(dev);
        twin.boot_sequence().unwrap();
        assert_eq!(twin.status, TwinStatus::Running);
        assert!(twin.time_ns > 0);
        assert!(twin.step_count > 0);
    }

    #[test]
    fn test_event_triggers_on_write() {
        let dev = sample_device();
        let mut twin = DigitalTwin::new(dev);
        twin.write_register("CONTROL", 1).unwrap();
        // Should have fired DMA_START (trigger: CONTROL[0]=1)
        let has_dma_start = twin.trace.iter().any(|t| t.event == "DMA_START");
        assert!(has_dma_start, "DMA_START should fire when CONTROL[0]=1");
    }

    #[test]
    fn test_verify_contracts() {
        let dev = sample_device();
        let mut twin = DigitalTwin::new(dev);
        twin.boot_sequence().unwrap();
        let verification = twin.verify_contracts();
        // Should not crash, may have passes or violations
        let _ = verification;
    }

    #[test]
    fn test_twin_reset() {
        let dev = sample_device();
        let mut twin = DigitalTwin::new(dev);
        twin.boot_sequence().unwrap();
        assert!(twin.time_ns > 0);
        twin.reset();
        assert_eq!(twin.time_ns, 0);
        assert_eq!(twin.step_count, 0);
        assert_eq!(twin.status, TwinStatus::Idle);
    }

    #[test]
    fn test_stats() {
        let dev = sample_device();
        let mut twin = DigitalTwin::new(dev);
        twin.boot_sequence().unwrap();
        let stats = twin.stats();
        assert_eq!(stats.device, "GPU");
        assert!(stats.time_ns > 0);
        assert!(stats.events_fired > 0);
    }
}
