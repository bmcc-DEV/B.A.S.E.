use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::tracer::{DeviceTrace, TraceEvent, EventType};

/// Parser de Wireshark PCAP (formato libpcap)
pub struct PcapParser;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PcapGlobalHeader {
    magic_number: u32,
    version_major: u16,
    version_minor: u16,
    thiszone: i32,
    sigfigs: u32,
    snaplen: u32,
    network: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PcapPacketHeader {
    ts_sec: u32,
    ts_usec: u32,
    incl_len: u32,
    orig_len: u32,
}

impl PcapParser {
    /// Parse de arquivo .pcap (libpcap format)
    pub fn parse(path: &Path) -> anyhow::Result<DeviceTrace> {
        let data = std::fs::read(path)?;
        let source = path.to_string_lossy().to_string();

        if data.len() < 24 {
            anyhow::bail!("File too small for PCAP header");
        }

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let is_swapped = magic == 0x4D3C2B1A; // swapped magic
        let is_nano = magic == 0xA1B2C3D4 || magic == 0x4D3C2B1A; // nanosecond timestamps

        if magic != 0xA1B2C3D4 && magic != 0xA1B23C4D && magic != 0x4D3C2B1A {
            anyhow::bail!("Not a valid PCAP file (magic: 0x{:08x})", magic);
        }

        let _header = PcapGlobalHeader {
            magic_number: magic,
            version_major: u16::from_le_bytes([data[4], data[5]]),
            version_minor: u16::from_le_bytes([data[6], data[7]]),
            thiszone: i32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            sigfigs: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            snaplen: u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            network: u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
        };

        let mut offset = 24usize;
        let mut events = Vec::new();

        while offset + 16 <= data.len() {
            let pkt = PcapPacketHeader {
                ts_sec: u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]),
                ts_usec: u32::from_le_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]),
                incl_len: u32::from_le_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]),
                orig_len: u32::from_le_bytes([data[offset+12], data[offset+13], data[offset+14], data[offset+15]]),
            };

            offset += 16;
            let incl = pkt.incl_len as usize;

            if offset + incl > data.len() {
                break;
            }

            let packet_data = &data[offset..offset + incl];
            let timestamp_ns = if is_nano {
                pkt.ts_sec as u64 * 1_000_000_000 + pkt.ts_usec as u64
            } else {
                pkt.ts_sec as u64 * 1_000_000_000 + pkt.ts_usec as u64 * 1000
            };

            // Parse USB packets (common for HW traces)
            if packet_data.len() >= 4 {
                let usb_type = packet_data[1];
                if usb_type == 0x43 || usb_type == 0x03 { // USB URB
                    let _urb_id = u64::from_le_bytes([
                        packet_data[8], packet_data[9], packet_data[10], packet_data[11],
                        packet_data[12], packet_data[13], packet_data[14], packet_data[15],
                    ]);
                    let endpoint = packet_data[2] & 0x7F;

                    let (event_type, address) = match packet_data[0] & 0x03 {
                        0x02 => (EventType::MmioWrite, endpoint as u64),
                        _ => (EventType::MmioRead, endpoint as u64),
                    };

                    let value = if packet_data.len() > 16 {
                        Some(u64::from_le_bytes([
                            packet_data[16], packet_data[17], packet_data[18], packet_data[19],
                            packet_data[20], packet_data[21], packet_data[22], packet_data[23],
                        ]))
                    } else {
                        None
                    };

                    events.push(TraceEvent {
                        timestamp_ns,
                        channel: format!("USB_EP{}", endpoint),
                        event_type,
                        address,
                        value,
                    });
                }
            }

            offset += incl;
        }

        Ok(DeviceTrace {
            source,
            device_name: path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
            events,
        })
    }

    /// Detecta se um arquivo é PCAP pela magic number
    pub fn is_pcap(path: &Path) -> bool {
        if let Ok(data) = std::fs::read(path) {
            if data.len() >= 4 {
                let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                return magic == 0xA1B2C3D4 || magic == 0xA1B23C4D || magic == 0x4D3C2B1A;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_pcap(path: &std::path::Path, packets: &[Vec<u8>]) {
        let mut data = Vec::new();
        // Global header
        data.extend_from_slice(&0xA1B2C3D4u32.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes()); // version_major
        data.extend_from_slice(&4u16.to_le_bytes()); // version_minor
        data.extend_from_slice(&0i32.to_le_bytes()); // thiszone
        data.extend_from_slice(&0u32.to_le_bytes()); // sigfigs
        data.extend_from_slice(&65535u32.to_le_bytes()); // snaplen
        data.extend_from_slice(&0u32.to_le_bytes()); // network (LINKTYPE_NULL)

        for pkt in packets {
            let ts_sec = 1000u32;
            let ts_usec = 0u32;
            let incl_len = pkt.len() as u32;
            data.extend_from_slice(&ts_sec.to_le_bytes());
            data.extend_from_slice(&ts_usec.to_le_bytes());
            data.extend_from_slice(&incl_len.to_le_bytes());
            data.extend_from_slice(&incl_len.to_le_bytes());
            data.extend_from_slice(pkt);
        }

        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(&data).unwrap();
    }

    #[test]
    fn test_is_pcap() {
        let dir = tempfile::tempdir().unwrap();
        let pcap = dir.path().join("test.pcap");
        write_pcap(&pcap, &[]);
        assert!(PcapParser::is_pcap(&pcap));
        assert!(!PcapParser::is_pcap(std::path::Path::new("Cargo.toml")));
    }

    #[test]
    fn test_parse_empty_pcap() {
        let dir = tempfile::tempdir().unwrap();
        let pcap = dir.path().join("empty.pcap");
        write_pcap(&pcap, &[]);
        let trace = PcapParser::parse(&pcap).unwrap();
        assert!(trace.events.is_empty());
    }

    #[test]
    fn test_parse_usb_pcap() {
        let dir = tempfile::tempdir().unwrap();
        let pcap = dir.path().join("usb.pcap");

        // Fake USB URB packet
        let mut pkt = vec![0u8; 32];
        pkt[0] = 0x43; // URB
        pkt[1] = 0x03; // submit
        pkt[2] = 0x81; // endpoint 1 IN
        pkt[8..16].copy_from_slice(&42u64.to_le_bytes()); // URB id
        pkt[16..24].copy_from_slice(&0x10000000u64.to_le_bytes()); // data

        write_pcap(&pcap, &[pkt]);
        let trace = PcapParser::parse(&pcap).unwrap();
        assert!(!trace.events.is_empty(), "Should parse USB packet");
    }

    #[test]
    fn test_invalid_magic() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.pcap");
        std::fs::write(&bad, b"not a pcap file at all").unwrap();
        assert!(PcapParser::parse(&bad).is_err());
    }
}
