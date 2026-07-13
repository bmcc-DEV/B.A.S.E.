/// Simple hand-written BSL parser (no pest dependency).
use base_bir::types::*;

#[derive(Debug)]
pub enum BslError {
    ParseError(String),
    CompileError(String),
}

fn tokenize(source: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut in_comment = false;
    for ch in source.chars() {
        if in_comment { if ch == '\n' { in_comment = false; } continue; }
        if ch == '/' { if !cur.is_empty() { tokens.push(cur.clone()); cur.clear(); } in_comment = true; continue; }
        if matches!(ch, '{' | '}' | '@' | ':' | ';' | '=' | '.' | ',' | '[' | ']' | '-' | '>') {
            if !cur.is_empty() { tokens.push(cur.clone()); cur.clear(); }
            tokens.push(ch.to_string());
        } else if ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' {
            if !cur.is_empty() { tokens.push(cur.clone()); cur.clear(); }
        } else { cur.push(ch); }
    }
    if !cur.is_empty() { tokens.push(cur); }
    tokens
}

fn expect(tokens: &[String], pos: &mut usize, expected: &str) -> Result<(), BslError> {
    if *pos < tokens.len() && tokens[*pos] == expected { *pos += 1; Ok(()) }
    else {
        let got = tokens.get(*pos).map(|s| s.as_str()).unwrap_or("EOF");
        Err(BslError::ParseError(format!("Expected '{}', got '{}'", expected, got)))
    }
}

fn parse_name(tokens: &[String], pos: &mut usize) -> Result<String, BslError> {
    if *pos < tokens.len() { let n = tokens[*pos].clone(); *pos += 1; Ok(n) }
    else { Err(BslError::ParseError("Expected name, got EOF".into())) }
}

fn parse_hex(tokens: &[String], pos: &mut usize) -> Result<u64, BslError> {
    if *pos < tokens.len() {
        let s = tokens[*pos].trim_start_matches("0x");
        let v = u64::from_str_radix(s, 16).map_err(|_| BslError::ParseError(format!("Invalid hex: {}", tokens[*pos])))?;
        *pos += 1; Ok(v)
    } else { Err(BslError::ParseError("Expected hex".into())) }
}

fn parse_num(tokens: &[String], pos: &mut usize) -> Result<u64, BslError> {
    if *pos < tokens.len() {
        let s = tokens[*pos].clone().trim_end_matches(|c: char| !c.is_ascii_digit()).to_string();
        let v = s.parse::<u64>().map_err(|_| BslError::ParseError(format!("Invalid number: {}", tokens[*pos])))?;
        *pos += 1; Ok(v)
    } else { Err(BslError::ParseError("Expected number".into())) }
}

fn is_hex(s: &str) -> bool { s.starts_with("0x") || s.starts_with("0X") }

pub fn compile(source: &str) -> Result<BirDevice, BslError> {
    let tokens = tokenize(source);
    let mut pos = 0;

    expect(&tokens, &mut pos, "device")?;
    let name = parse_name(&tokens, &mut pos)?;
    let mut device = BirDevice::new(&name);

    if pos < tokens.len() && tokens[pos] == "@" { pos += 1; device.base_address = Some(parse_hex(&tokens, &mut pos)?); }

    expect(&tokens, &mut pos, "{")?;

    while pos < tokens.len() && tokens[pos] != "}" {
        match tokens[pos].as_str() {
            "registers" => {
                pos += 1; expect(&tokens, &mut pos, "{")?;
                while pos < tokens.len() && tokens[pos] != "}" {
                    let rn = parse_name(&tokens, &mut pos)?;
                    expect(&tokens, &mut pos, "@")?;
                    let off = parse_hex(&tokens, &mut pos)? as u32;
                    expect(&tokens, &mut pos, ":")?;
                    let acc = match tokens.get(pos).map(|s| s.as_str()) {
                        Some("rw") => { pos += 1; BirAccess::ReadWrite }
                        Some("ro") => { pos += 1; BirAccess::Read }
                        Some("wo") => { pos += 1; BirAccess::Write }
                        _ => return Err(BslError::ParseError("Expected rw/ro/wo".into())),
                    };
                    let rv = if pos < tokens.len() && tokens[pos] == "=" { pos += 1; Some(parse_hex(&tokens, &mut pos)?) } else { None };
                    expect(&tokens, &mut pos, ";")?;
                    device.registers.push(BirRegister { name: rn, offset: off, access: acc, width: 32, reset_value: rv, bitfields: vec![] });
                }
                if pos < tokens.len() { pos += 1; }
            }
            "events" => {
                pos += 1; expect(&tokens, &mut pos, "{")?;
                while pos < tokens.len() && tokens[pos] != "}" {
                    let en = parse_name(&tokens, &mut pos)?;
                    expect(&tokens, &mut pos, ":")?;
                    let kind = tokens[pos].clone(); pos += 1;
                    let reg = parse_name(&tokens, &mut pos)?;
                    expect(&tokens, &mut pos, "[")?;
                    let bs = parse_num(&tokens, &mut pos)? as u8;
                    if pos < tokens.len() && tokens[pos] == "." { pos += 1; let _ = parse_num(&tokens, &mut pos)?; } // skip range
                    expect(&tokens, &mut pos, "]")?;
                    expect(&tokens, &mut pos, "=")?;
                    let val = parse_num(&tokens, &mut pos)?;
                    expect(&tokens, &mut pos, ";")?;
                    device.events.push(BirEvent {
                        name: en,
                        trigger: BirTrigger {
                            kind: if kind == "write" { TriggerKind::Write } else { TriggerKind::Read },
                            register: reg, bit_range: Some(bs..bs + 1), value: Some(val),
                        },
                        timing: None,
                    });
                }
                if pos < tokens.len() { pos += 1; }
            }
            "interrupts" => {
                pos += 1; expect(&tokens, &mut pos, "{")?;
                let mut vec = 1u8;
                while pos < tokens.len() && tokens[pos] != "}" {
                    let irqn = parse_name(&tokens, &mut pos)?;
                    expect(&tokens, &mut pos, ":")?;
                    let _typ = tokens[pos].clone(); pos += 1;
                    let pol = tokens[pos].clone(); pos += 1;
                    expect(&tokens, &mut pos, ";")?;
                    device.interrupts.push(BirInterrupt {
                        name: irqn, vector: vec, irq_type: IrqType::Level,
                        polarity: if pol == "high" { IrqPolarity::High } else { IrqPolarity::Low },
                    });
                    vec += 1;
                }
                if pos < tokens.len() { pos += 1; }
            }
            "timing" => {
                pos += 1; expect(&tokens, &mut pos, "{")?;
                while pos < tokens.len() && tokens[pos] != "}" {
                    let tn = parse_name(&tokens, &mut pos)?;
                    expect(&tokens, &mut pos, ":")?;
                    let min = parse_num(&tokens, &mut pos)?;
                    if pos < tokens.len() && tokens[pos] == "ns" { pos += 1; }
                    if pos < tokens.len() && tokens[pos] == "." { pos += 1; pos += 1; } // skip ..
                    let max = parse_num(&tokens, &mut pos)?;
                    if pos < tokens.len() && tokens[pos] == "ns" { pos += 1; }
                    expect(&tokens, &mut pos, ";")?;
                    device.timing.push(BirTimingEntry { name: tn, latency: BirLatencyRange::new(min, max), per_unit: None });
                }
                if pos < tokens.len() { pos += 1; }
            }
            "contract" => {
                pos += 1; expect(&tokens, &mut pos, "{")?;
                let mut contract = BirContract { must_occur_before: Vec::new(), latency: Vec::new(), window_ns: None, jitter_ns: None, repetition_rate: None };
                while pos < tokens.len() && tokens[pos] != "}" {
                    if tokens[pos] == "must_occur_before" {
                        pos += 1; expect(&tokens, &mut pos, ":")?;
                        let a = parse_name(&tokens, &mut pos)?;
                        expect(&tokens, &mut pos, "-")?; expect(&tokens, &mut pos, ">")?;
                        let b = parse_name(&tokens, &mut pos)?;
                        expect(&tokens, &mut pos, ";")?;
                        contract.must_occur_before.push(CausalOrder { event_a: a, event_b: b, max_delta_ns: None });
                    } else if tokens[pos] == "window" {
                        pos += 1; expect(&tokens, &mut pos, ":")?;
                        let val = parse_num(&tokens, &mut pos)?;
                        let unit = if pos < tokens.len() { tokens[pos].clone() } else { "ns".into() };
                        if pos < tokens.len() && (tokens[pos] == "ns" || tokens[pos] == "us" || tokens[pos] == "ms") { pos += 1; }
                        contract.window_ns = Some(match unit.as_str() { "us" => val * 1000, "ms" => val * 1_000_000, _ => val });
                        expect(&tokens, &mut pos, ";")?;
                    } else {
                        let ev = parse_name(&tokens, &mut pos)?;
                        expect(&tokens, &mut pos, ":")?;
                        let min = parse_num(&tokens, &mut pos)?;
                        if pos < tokens.len() && tokens[pos] == "ns" { pos += 1; }
                        if pos < tokens.len() && tokens[pos] == "." { pos += 1; }
                        let max = parse_num(&tokens, &mut pos)?;
                        if pos < tokens.len() && tokens[pos] == "ns" { pos += 1; }
                        expect(&tokens, &mut pos, ";")?;
                        contract.latency.push(BirLatencyConstraint { event: ev, min_ns: min, max_ns: max, unit: None });
                    }
                }
                if pos < tokens.len() { pos += 1; }
                device.contracts.push(contract);
            }
            _ => return Err(BslError::ParseError(format!("Unexpected: {}", tokens[pos]))),
        }
    }

    Ok(device)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_device() {
        let src = "device GPU @ 0x10000000 { registers { CONTROL @ 0x00: rw = 0x0; STATUS @ 0x04: ro; } events { DMA_START: write CONTROL[0] = 1; } interrupts { IRQ_GPU: level high; } timing { dma_setup: 100ns..400ns; } }";
        let dev = compile(src).expect("Should compile");
        assert_eq!(dev.name, "GPU");
        assert_eq!(dev.base_address, Some(0x10000000));
        assert_eq!(dev.registers.len(), 2);
    }

    #[test]
    fn test_compile_with_contract() {
        let src = "device DMA { registers { CTRL @ 0x00: rw; } events { DMA_START: write CTRL[0] = 1; DMA_DONE: read CTRL[7] = 1; } contract { must_occur_before: DMA_DONE -> DMA_START; window: 10us; } }";
        let dev = compile(src).expect("Should compile with contract");
        assert_eq!(dev.contracts.len(), 1);
        assert_eq!(dev.contracts[0].must_occur_before.len(), 1);
    }

    #[test]
    fn test_tokenize() {
        let t = tokenize("device GPU @ 0x1000 { }");
        assert_eq!(t[0], "device");
    }

    #[test]
    fn test_bir_roundtrip() {
        let src = "device TEST @ 0x1000 { registers { R1 @ 0x00: rw; } }";
        let dev = compile(src).expect("compile");
        let y = dev.to_yaml().expect("yaml");
        let dec = BirDevice::from_yaml(&y).expect("parse");
        assert_eq!(dec.name, "TEST");
    }

    #[test]
    fn test_compile_error() {
        assert!(compile("invalid").is_err());
    }

    #[test]
    fn test_tokenize_arrow() {
        let t = tokenize("a -> b");
        assert!(t.contains(&">".to_string()) || t.contains(&"->".to_string()));
    }
}
