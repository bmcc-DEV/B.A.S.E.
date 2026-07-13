use serde::{Deserialize, Serialize};

// ─── Core Enums ───────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BirAccess {
    Read,
    Write,
    ReadWrite,
    WriteOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrqType {
    Level,
    Edge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrqPolarity {
    High,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockKind {
    Gpu, Audio, Dma, Usb, Ethernet, Spi, I2c, Uart,
    Timer, InterruptController, MemoryController,
    Crypto, VideoCodec, Isp, Npu, Unknown,
}

// ─── Core Structs ─────────────────────────────────────

/// Dispositivo completo — equivalente ao `device { }` em BSL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirDevice {
    pub name: String,
    pub base_address: Option<u64>,
    pub registers: Vec<BirRegister>,
    pub events: Vec<BirEvent>,
    pub interrupts: Vec<BirInterrupt>,
    pub timing: Vec<BirTimingEntry>,
    pub contracts: Vec<BirContract>,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirRegister {
    pub name: String,
    pub offset: u32,
    pub access: BirAccess,
    pub width: u8,
    pub reset_value: Option<u64>,
    pub bitfields: Vec<BirBitfield>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirBitfield {
    pub offset: u8,
    pub width: u8,
    pub name: Option<String>,
    pub values: Vec<(u64, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirEvent {
    pub name: String,
    pub trigger: BirTrigger,
    pub timing: Option<BirLatencyRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirTrigger {
    pub kind: TriggerKind,
    pub register: String,
    pub bit_range: Option<std::ops::Range<u8>>,
    pub value: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerKind {
    Write,
    Read,
    WriteBit,
    ReadBit,
    AnyAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirInterrupt {
    pub name: String,
    pub vector: u8,
    pub irq_type: IrqType,
    pub polarity: IrqPolarity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirTimingEntry {
    pub name: String,
    pub latency: BirLatencyRange,
    pub per_unit: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BirLatencyRange {
    pub min_ns: u64,
    pub max_ns: u64,
}

impl BirLatencyRange {
    pub fn new(min_ns: u64, max_ns: u64) -> Self {
        Self { min_ns, max_ns }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirContract {
    pub must_occur_before: Vec<CausalOrder>,
    pub latency: Vec<BirLatencyConstraint>,
    pub window_ns: Option<u64>,
    pub jitter_ns: Option<u64>,
    pub repetition_rate: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalOrder {
    pub event_a: String,
    pub event_b: String,
    pub max_delta_ns: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirLatencyConstraint {
    pub event: String,
    pub min_ns: u64,
    pub max_ns: u64,
    pub unit: Option<String>,
}

// ─── Temporal Contract Result ─────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractVerification {
    pub device: String,
    pub contracts_checked: usize,
    pub passes: Vec<ContractCheck>,
    pub violations: Vec<ContractViolation>,
    pub all_pass: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCheck {
    pub name: String,
    pub kind: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractViolation {
    pub contract: String,
    pub kind: ViolationKind,
    pub expected: String,
    pub actual: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationKind {
    CausalOrder,
    LatencyExceeded,
    LatencyBelow,
    WindowExceeded,
    JitterExceeded,
    MissingEvent,
}

// ─── Validation Result ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirValidation {
    pub device_name: String,
    pub errors: Vec<BirError>,
    pub warnings: Vec<String>,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirError {
    pub kind: BirErrorKind,
    pub message: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BirErrorKind {
    DuplicateRegister,
    DuplicateEvent,
    InvalidReference,
    TimingViolation,
    MissingRegister,
    MissingEvent,
    ContractViolation,
}

// ─── Serialization ────────────────────────────────────

impl BirDevice {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            base_address: None,
            registers: Vec::new(),
            events: Vec::new(),
            interrupts: Vec::new(),
            timing: Vec::new(),
            contracts: Vec::new(),
            version: 1,
        }
    }

    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_yaml(s: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_device() -> BirDevice {
        let mut dev = BirDevice::new("GPU");
        dev.base_address = Some(0x10000000);
        dev.registers.push(BirRegister {
            name: "CONTROL".into(), offset: 0x00,
            access: BirAccess::ReadWrite, width: 32,
            reset_value: Some(0), bitfields: vec![],
        });
        dev.registers.push(BirRegister {
            name: "STATUS".into(), offset: 0x04,
            access: BirAccess::Read, width: 32,
            reset_value: None, bitfields: vec![],
        });
        dev.events.push(BirEvent {
            name: "DMA_START".into(),
            trigger: BirTrigger {
                kind: TriggerKind::Write,
                register: "CONTROL".into(),
                bit_range: None,
                value: Some(1),
            },
            timing: None,
        });
        dev.interrupts.push(BirInterrupt {
            name: "IRQ_GPU".into(), vector: 16,
            irq_type: IrqType::Level, polarity: IrqPolarity::High,
        });
        dev.timing.push(BirTimingEntry {
            name: "dma_setup".into(),
            latency: BirLatencyRange::new(100, 400),
            per_unit: None,
        });
        dev
    }

    #[test]
    fn test_bir_device_new() {
        let dev = BirDevice::new("test");
        assert_eq!(dev.name, "test");
        assert!(dev.registers.is_empty());
    }

    #[test]
    fn test_bir_yaml_roundtrip() {
        let dev = sample_device();
        let yaml = dev.to_yaml().unwrap();
        let decoded = BirDevice::from_yaml(&yaml).unwrap();
        assert_eq!(decoded.name, "GPU");
        assert_eq!(decoded.registers.len(), 2);
        assert_eq!(decoded.interrupts.len(), 1);
        assert_eq!(decoded.timing.len(), 1);
    }

    #[test]
    fn test_bir_json_roundtrip() {
        let dev = sample_device();
        let json = dev.to_json().unwrap();
        let decoded = BirDevice::from_json(&json).unwrap();
        assert_eq!(decoded.name, "GPU");
        assert_eq!(decoded.base_address, Some(0x10000000));
    }
}
