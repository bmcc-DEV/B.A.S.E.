use crate::types::*;

impl BirDevice {
    /// Valida o dispositivo BIR contra regras de consistência
    pub fn validate(&self) -> BirValidation {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 1. Verificar registradores duplicados
        let mut seen_offsets = std::collections::HashSet::new();
        for reg in &self.registers {
            if !seen_offsets.insert(reg.offset) {
                errors.push(BirError {
                    kind: BirErrorKind::DuplicateRegister,
                    message: format!("Duplicate register at offset 0x{:x}", reg.offset),
                    location: Some(reg.name.clone()),
                });
            }
        }

        // 2. Verificar eventos duplicados
        let mut seen_events = std::collections::HashSet::new();
        for ev in &self.events {
            if !seen_events.insert(&ev.name) {
                errors.push(BirError {
                    kind: BirErrorKind::DuplicateEvent,
                    message: format!("Duplicate event: {}", ev.name),
                    location: Some(ev.name.clone()),
                });
            }
        }

        // 3. Verificar referências a registradores existentes
        let reg_names: std::collections::HashSet<&str> =
            self.registers.iter().map(|r| r.name.as_str()).collect();

        for ev in &self.events {
            if !reg_names.contains(ev.trigger.register.as_str()) {
                errors.push(BirError {
                    kind: BirErrorKind::InvalidReference,
                    message: format!("Event '{}' references unknown register '{}'", ev.name, ev.trigger.register),
                    location: Some(ev.name.clone()),
                });
            }
        }

        // 4. Verificar contratos referenciam eventos OU interrupções existentes
        let mut event_names: std::collections::HashSet<&str> =
            self.events.iter().map(|e| e.name.as_str()).collect();
        for irq in &self.interrupts {
            event_names.insert(irq.name.as_str());
        }

        for contract in &self.contracts {
            for order in &contract.must_occur_before {
                if !event_names.contains(order.event_a.as_str()) {
                    errors.push(BirError {
                        kind: BirErrorKind::MissingEvent,
                        message: format!("Contract references unknown event '{}'", order.event_a),
                        location: Some(order.event_a.clone()),
                    });
                }
                if !event_names.contains(order.event_b.as_str()) {
                    errors.push(BirError {
                        kind: BirErrorKind::MissingEvent,
                        message: format!("Contract references unknown event '{}'", order.event_b),
                        location: Some(order.event_b.clone()),
                    });
                }
            }
        }

        // 5. Verificar timing: min <= max
        for t in &self.timing {
            if t.latency.min_ns > t.latency.max_ns {
                errors.push(BirError {
                    kind: BirErrorKind::TimingViolation,
                    message: format!("Timing '{}': min {}ns > max {}ns", t.name, t.latency.min_ns, t.latency.max_ns),
                    location: Some(t.name.clone()),
                });
            }
        }

        // 6. Warnings
        if self.registers.is_empty() {
            warnings.push("Device has no registers defined".into());
        }
        if self.base_address.is_none() {
            warnings.push("Device has no base address".into());
        }

        let is_valid = errors.is_empty();

        BirValidation {
            device_name: self.name.clone(),
            errors,
            warnings,
            is_valid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_clean_device() {
        let mut dev = BirDevice::new("test");
        dev.registers.push(BirRegister {
            name: "CTRL".into(), offset: 0, access: BirAccess::ReadWrite,
            width: 32, reset_value: None, bitfields: vec![],
        });
        dev.events.push(BirEvent {
            name: "START".into(),
            trigger: BirTrigger {
                kind: TriggerKind::Write, register: "CTRL".into(),
                bit_range: None, value: Some(1),
            },
            timing: None,
        });
        let result = dev.validate();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_duplicate_register() {
        let mut dev = BirDevice::new("test");
        dev.registers.push(BirRegister {
            name: "CTRL".into(), offset: 0, access: BirAccess::ReadWrite,
            width: 32, reset_value: None, bitfields: vec![],
        });
        dev.registers.push(BirRegister {
            name: "CTRL2".into(), offset: 0, access: BirAccess::Read,
            width: 32, reset_value: None, bitfields: vec![],
        });
        let result = dev.validate();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.kind == BirErrorKind::DuplicateRegister));
    }

    #[test]
    fn test_validate_invalid_reference() {
        let mut dev = BirDevice::new("test");
        dev.registers.push(BirRegister {
            name: "CTRL".into(), offset: 0, access: BirAccess::ReadWrite,
            width: 32, reset_value: None, bitfields: vec![],
        });
        dev.events.push(BirEvent {
            name: "START".into(),
            trigger: BirTrigger {
                kind: TriggerKind::Write, register: "NONEXISTENT".into(),
                bit_range: None, value: Some(1),
            },
            timing: None,
        });
        let result = dev.validate();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.kind == BirErrorKind::InvalidReference));
    }

    #[test]
    fn test_validate_timing_violation() {
        let mut dev = BirDevice::new("test");
        dev.timing.push(BirTimingEntry {
            name: "bad".into(),
            latency: BirLatencyRange::new(500, 100), // min > max
            per_unit: None,
        });
        let result = dev.validate();
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.kind == BirErrorKind::TimingViolation));
    }
}
