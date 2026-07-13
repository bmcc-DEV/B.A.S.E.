use serde::{Deserialize, Serialize};
use std::path::Path;

/// Evento único de trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub timestamp_ns: u64,
    pub channel: String,
    pub event_type: EventType,
    pub address: u64,
    pub value: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    MmioRead,
    MmioWrite,
    Interrupt,
    DmaStart,
    DmaEnd,
    GpioToggle,
}

/// Trace completo de um dispositivo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTrace {
    pub source: String,
    pub device_name: String,
    pub events: Vec<TraceEvent>,
}

/// Formatos de trace suportados
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceFormat {
    SaleaeCsv,
    CustomJson,
}

/// Parser de traces
pub struct TraceParser;

impl TraceParser {
    /// Detecta o formato do arquivo e faz o parse
    pub fn parse(path: &Path) -> anyhow::Result<DeviceTrace> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "csv" => Self::parse_saleae_csv(path),
            "json" => Self::parse_custom_json(path),
            "pcap" | "cap" => crate::pcap::PcapParser::parse(path),
            _ => {
                // Try PCAP by magic number
                if crate::pcap::PcapParser::is_pcap(path) {
                    crate::pcap::PcapParser::parse(path)
                } else {
                    Err(anyhow::anyhow!("Unknown trace format: {} (try .csv, .json, .pcap)", ext))
                }
            }
        }
    }

    /// Parse de Saleae CSV export
    /// Formato: Time[s], Channel, Type, Data
    /// Ex: 0.001234, CH0, WRITE, 0x10000000=0x01
    fn parse_saleae_csv(path: &Path) -> anyhow::Result<DeviceTrace> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(path)?;

        let mut events = Vec::new();
        let source = path.to_string_lossy().to_string();

        for result in rdr.records() {
            let record = result?;
            if record.len() < 4 {
                continue;
            }

            let time_sec: f64 = record[0].parse().unwrap_or(0.0);
            let timestamp_ns = (time_sec * 1_000_000_000.0) as u64;

            let event_str = record[2].to_uppercase();
            let data_str = &record[3];

            let (event_type, address, value) = if event_str == "WRITE" || event_str == "W" {
                Self::parse_data_field(data_str, EventType::MmioWrite)
            } else if event_str == "READ" || event_str == "R" {
                Self::parse_data_field(data_str, EventType::MmioRead)
            } else if event_str == "IRQ" || event_str == "I" {
                (EventType::Interrupt, data_str.parse::<u64>().unwrap_or(0), None)
            } else {
                continue;
            };

            events.push(TraceEvent {
                timestamp_ns,
                channel: record[1].to_string(),
                event_type,
                address,
                value,
            });
        }

        Ok(DeviceTrace {
            source,
            device_name: path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
            events,
        })
    }

    /// Parse de JSON formatado pelo B.A.S.E.
    fn parse_custom_json(path: &Path) -> anyhow::Result<DeviceTrace> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let trace: DeviceTrace = serde_json::from_reader(reader)?;
        Ok(trace)
    }

    fn parse_data_field(data: &str, default_type: EventType) -> (EventType, u64, Option<u64>) {
        // Format: 0xADDR=0xVAL or just 0xADDR
        if let Some(eq_pos) = data.find('=') {
            let addr_str = data[..eq_pos].trim();
            let val_str = data[eq_pos + 1..].trim();
            let addr = u64::from_str_radix(addr_str.trim_start_matches("0x"), 16).unwrap_or(0);
            let val = u64::from_str_radix(val_str.trim_start_matches("0x"), 16).ok();
            (default_type, addr, val)
        } else {
            let addr = u64::from_str_radix(data.trim_start_matches("0x"), 16).unwrap_or(0);
            (default_type, addr, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data_field() {
        let (t, addr, val) = TraceParser::parse_data_field("0x10000000=0x01", EventType::MmioWrite);
        assert_eq!(addr, 0x10000000);
        assert_eq!(val, Some(0x01));
        assert_eq!(t, EventType::MmioWrite);

        let (t2, addr2, val2) = TraceParser::parse_data_field("0xFF", EventType::MmioRead);
        assert_eq!(addr2, 0xFF);
        assert_eq!(val2, None);
        assert_eq!(t2, EventType::MmioRead);
    }

    #[test]
    fn test_roundtrip_json() {
        let trace = DeviceTrace {
            source: "test".into(),
            device_name: "gpu".into(),
            events: vec![
                TraceEvent { timestamp_ns: 1000, channel: "CH0".into(), event_type: EventType::MmioWrite, address: 0x10000000, value: Some(1) },
                TraceEvent { timestamp_ns: 2000, channel: "CH0".into(), event_type: EventType::MmioRead, address: 0x10000004, value: None },
            ],
        };

        let json = serde_json::to_string(&trace).unwrap();
        let decoded: DeviceTrace = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.events.len(), 2);
        assert_eq!(decoded.events[0].address, 0x10000000);
    }
}
