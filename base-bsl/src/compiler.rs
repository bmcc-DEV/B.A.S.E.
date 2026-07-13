use base_bir::types::*;
use pest::Parser;
use crate::parser::BslParser;

#[derive(Debug)]
pub enum BslError {
    ParseError(String),
    CompileError(String),
}

/// Compila um source BSL para um BirDevice
pub fn compile(source: &str) -> Result<BirDevice, BslError> {
    let mut pairs = BslParser::parse(parser::Rule::program, source)
        .map_err(|e| BslError::ParseError(e.to_string()))?;

    let pair = pairs.next().ok_or_else(|| BslError::ParseError("Empty source".into()))?;

    let mut device = BirDevice::new("unknown");

    // Parse top-level device
    for inner in pair.into_inner() {
        match inner.as_rule() {
            parser::Rule::name => {
                device.name = inner.as_str().to_string();
            }
            parser::Rule::address => {
                let addr_str = inner.as_str().trim_start_matches("0x");
                device.base_address = u64::from_str_radix(addr_str, 16).ok();
            }
            parser::Rule::body => {
                for section in inner.into_inner() {
                    match section.as_rule() {
                        parser::Rule::register_section => parse_registers(section, &mut device),
                        parser::Rule::event_section => parse_events(section, &mut device),
                        parser::Rule::interrupt_section => parse_interrupts(section, &mut device),
                        parser::Rule::timing_section => parse_timing(section, &mut device),
                        parser::Rule::contract_section => parse_contract(section, &mut device),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Ok(device)
}

fn parse_registers(section: pest::iterators::Pair<'_, parser::Rule>, device: &mut BirDevice) {
    for decl in section.into_inner() {
        if decl.as_rule() != parser::Rule::register_decl { continue; }
        let mut reg = BirRegister {
            name: String::new(), offset: 0,
            access: BirAccess::ReadWrite, width: 32,
            reset_value: None, bitfields: vec![],
        };

        for field in decl.into_inner() {
            match field.as_rule() {
                parser::Rule::name => reg.name = field.as_str().to_string(),
                parser::Rule::offset => {
                    let o = field.as_str().trim_start_matches("0x");
                    reg.offset = u32::from_str_radix(o, 16).unwrap_or(0);
                }
                parser::Rule::access => {
                    reg.access = match field.as_str() {
                        "ro" => BirAccess::Read,
                        "wo" => BirAccess::Write,
                        "rw" | _ => BirAccess::ReadWrite,
                    };
                }
                parser::Rule::reset_value => {
                    reg.reset_value = u64::from_str_radix(field.as_str(), 16).ok();
                }
                _ => {}
            }
        }
        device.registers.push(reg);
    }
}

fn parse_events(section: pest::iterators::Pair<'_, parser::Rule>, device: &mut BirDevice) {
    for decl in section.into_inner() {
        if decl.as_rule() != parser::Rule::event_decl { continue; }
        let mut event = BirEvent {
            name: String::new(),
            trigger: BirTrigger {
                kind: TriggerKind::Write, register: String::new(),
                bit_range: None, value: None,
            },
            timing: None,
        };

        let mut inner_rules: Vec<pest::iterators::Pair<'_, parser::Rule>> = decl.into_inner().collect();
        if let Some(first) = inner_rules.first() {
            if first.as_rule() == parser::Rule::name {
                event.name = first.as_str().to_string();
            }
        }
        if inner_rules.len() > 1 {
            if let Some(trigger_pair) = inner_rules.get(1) {
                parse_trigger(trigger_pair.clone(), &mut event.trigger);
            }
        }
        device.events.push(event);
    }
}

fn parse_trigger(pair: pest::iterators::Pair<'_, parser::Rule>, trigger: &mut BirTrigger) {
    for field in pair.into_inner() {
        match field.as_rule() {
            parser::Rule::name => trigger.register = field.as_str().to_string(),
            parser::Rule::int_literal => {
                trigger.value = field.as_str().parse::<u64>().ok();
            }
            parser::Rule::range => {
                let mut parts = field.as_str().split("..");
                let lo = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
                let hi = parts.next().and_then(|s| s.parse::<u8>().ok()).unwrap_or(1);
                trigger.bit_range = Some(lo..hi);
            }
            _ => {
                if field.as_str() == "write" { trigger.kind = TriggerKind::Write; }
                else if field.as_str() == "read" { trigger.kind = TriggerKind::Read; }
            }
        }
    }
}

fn parse_interrupts(section: pest::iterators::Pair<'_, parser::Rule>, device: &mut BirDevice) {
    for (i, decl) in section.into_inner().enumerate() {
        if decl.as_rule() != parser::Rule::interrupt_decl { continue; }
        let mut irq = BirInterrupt {
            name: String::new(), vector: (i + 1) as u8,
            irq_type: IrqType::Level, polarity: IrqPolarity::High,
        };
        for field in decl.into_inner() {
            match field.as_rule() {
                parser::Rule::name => irq.name = field.as_str().to_string(),
                parser::Rule::irq_type => {
                    irq.irq_type = if field.as_str() == "edge" { IrqType::Edge } else { IrqType::Level };
                }
                parser::Rule::polarity => {
                    irq.polarity = if field.as_str() == "low" { IrqPolarity::Low } else { IrqPolarity::High };
                }
                _ => {}
            }
        }
        device.interrupts.push(irq);
    }
}

fn parse_timing(section: pest::iterators::Pair<'_, parser::Rule>, device: &mut BirDevice) {
    for decl in section.into_inner() {
        if decl.as_rule() != parser::Rule::timing_decl { continue; }
        let mut entry = BirTimingEntry {
            name: String::new(),
            latency: BirLatencyRange::new(0, 0),
            per_unit: None,
        };

        for field in decl.into_inner() {
            match field.as_rule() {
                parser::Rule::name => entry.name = field.as_str().to_string(),
                parser::Rule::latency => {
                    let s = field.as_str();
                    let parts: Vec<&str> = s.split("ns").collect();
                    let range_str = parts[0].trim();
                    if let Some(dots) = range_str.find("..") {
                        let min = range_str[..dots].trim().parse::<u64>().unwrap_or(0);
                        let max = range_str[dots+2..].trim().parse::<u64>().unwrap_or(0);
                        entry.latency = BirLatencyRange::new(min, max);
                    }
                }
                _ => {
                    if field.as_str() == "per_word" { entry.per_unit = Some("word".into()); }
                    else if field.as_str() == "per_byte" { entry.per_unit = Some("byte".into()); }
                }
            }
        }
        device.timing.push(entry);
    }
}

fn parse_contract(section: pest::iterators::Pair<'_, parser::Rule>, device: &mut BirDevice) {
    let mut contract = BirContract {
        must_occur_before: Vec::new(),
        latency: Vec::new(),
        window_ns: None,
    };

    for field in section.into_inner() {
        match field.as_rule() {
            parser::Rule::must_occur_before_decl => {
                let s = field.as_str().trim_start_matches("must_occur_before:").trim();
                if let Some(arrow) = s.find("->") {
                    let a = s[..arrow].trim().to_string();
                    let b = s[arrow+2..].trim().trim_end_matches(';').to_string();
                    contract.must_occur_before.push((a, b));
                }
            }
            parser::Rule::latency_constraint_decl => {
                let s = field.as_str().trim_end_matches(';');
                if let Some(colon) = s.find(':') {
                    let name = s[..colon].trim().to_string();
                    let range = s[colon+1..].trim();
                    let parts: Vec<&str> = range.split("ns").collect();
                    let range_str = parts[0].trim();
                    if let Some(dots) = range_str.find("..") {
                        let min = range_str[..dots].trim().parse::<u64>().unwrap_or(0);
                        let max = range_str[dots+2..].trim().parse::<u64>().unwrap_or(0);
                        contract.latency.push(BirLatencyConstraint {
                            event: name, min_ns: min, max_ns: max,
                        });
                    }
                }
            }
            parser::Rule::window_decl => {
                let s = field.as_str().trim_start_matches("window:").trim();
                let num_str: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
                contract.window_ns = num_str.parse::<u64>().ok();
            }
            _ => {}
        }
    }

    device.contracts.push(contract);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_device() {
        let source = r#"
device GPU @ 0x10000000 {
    registers {
        CONTROL @ 0x00: rw = 0;
        STATUS  @ 0x04: ro;
    }

    events {
        DMA_START: write CONTROL[0] = 1;
    }

    interrupts {
        IRQ_GPU: level high;
    }

    timing {
        dma_setup: 100ns..400ns;
    }
}
"#;
        let device = compile(source).expect("Should compile");
        assert_eq!(device.name, "GPU");
        assert_eq!(device.base_address, Some(0x10000000));
        assert_eq!(device.registers.len(), 2);
        assert_eq!(device.events.len(), 1);
        assert_eq!(device.interrupts.len(), 1);
        assert_eq!(device.timing.len(), 1);
    }

    #[test]
    fn test_compile_with_contract() {
        let source = r#"
device DMA {
    registers {
        CTRL @ 0x00: rw;
    }
    events {
        DMA_START: write CTRL[0] = 1;
        DMA_DONE:  read CTRL[7]  = 1;
    }
    contract {
        must_occur_before: DMA_DONE -> DMA_START;
        window: 10us;
    }
}
"#;
        let device = compile(source).expect("Should compile with contract");
        assert_eq!(device.name, "DMA");
        assert_eq!(device.contracts.len(), 1);
        assert_eq!(device.contracts[0].must_occur_before.len(), 1);
    }

    #[test]
    fn test_compile_error() {
        let source = "invalid bsl content";
        let result = compile(source);
        assert!(result.is_err(), "Invalid BSL should fail");
    }

    #[test]
    fn test_bir_roundtrip() {
        let source = r#"
device TEST @ 0x1000 {
    registers {
        R1 @ 0x00: rw;
    }
}
"#;
        let device = compile(source).expect("Should compile");
        let yaml = device.to_yaml().expect("Should serialize");
        let decoded = BirDevice::from_yaml(&yaml).expect("Should deserialize");
        assert_eq!(decoded.name, "TEST");
        assert_eq!(decoded.registers[0].name, "R1");
    }
}
