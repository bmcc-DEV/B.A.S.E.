use serde::{Deserialize, Serialize};

// ─── Core Enums ───────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpuArch {
    PowerPC,
    Arm,
    Arm64,
    X86,
    X8664,
    RiscV,
    Mips,
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Endian {
    Big,
    Little,
    Bi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockKind {
    Gpu,
    Audio,
    Dma,
    Usb,
    Ethernet,
    Spi,
    I2c,
    Uart,
    Timer,
    InterruptController,
    MemoryController,
    Crypto,
    VideoCodec,
    Isp,
    Npu,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessType {
    Read,
    Write,
    ReadWrite,
    WriteOnly,
    WriteClear,
    WriteToggle,
    ReadDestruct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegisterPurpose {
    Control,
    Status,
    InterruptMask,
    InterruptStatus,
    AddressPointer,
    DataLength,
    DataPort,
    ClockDivider,
    DmaControl,
    DebugRegister,
    UnknownPurpose,
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

// ─── Core Structs ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSpec {
    pub version: u32,
    pub source: String,
    pub cpu: CpuSpec,
    pub memory: MemoryLayout,
    pub blocks: Vec<FunctionalBlock>,
    pub interrupts: Vec<InterruptSpec>,
    pub dma_channels: Vec<DmaChannel>,
    pub constraints: SystemConstraints,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSpec {
    pub architecture: CpuArch,
    pub clock_mhz: u32,
    pub endianness: Endian,
    pub cores: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLayout {
    pub regions: Vec<MemoryRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRegion {
    pub name: String,
    pub base: u64,
    pub size: u64,
    pub region_type: MemoryRegionType,
    pub width: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRegionType {
    Ram,
    Rom,
    Mmio,
    Reserved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalBlock {
    pub id: String,
    pub kind: BlockKind,
    pub base_address: u64,
    pub size: u64,
    pub registers: Vec<Register>,
    pub protocol: Protocol,
    pub timing: TimingProfile,
    pub dma: Option<DmaRequirement>,
    pub dependencies: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Register {
    pub offset: u32,
    pub name: Option<String>,
    pub width: u8,
    pub access: AccessType,
    pub purpose: RegisterPurpose,
    pub reset_value: Option<u64>,
    pub observed_values: Vec<ObservedValue>,
    pub bitfields: Vec<Bitfield>,
    pub polling: bool,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedValue {
    pub value: u64,
    pub count: usize,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bitfield {
    pub offset: u8,
    pub width: u8,
    pub name: Option<String>,
    pub values: Vec<(u64, String)>,
    pub observed_mask: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protocol {
    pub states: Vec<String>,
    pub transitions: Vec<Transition>,
    pub entry_condition: Option<Condition>,
    pub exit_condition: Option<Condition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub trigger: Trigger,
    pub latency: Option<LatencyRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub kind: String,
    pub register_offset: Option<u32>,
    pub value: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub kind: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingProfile {
    pub activation: Option<LatencyRange>,
    pub processing: Option<LatencyRange>,
    pub interrupt_response: Option<LatencyRange>,
    pub dma_setup: Option<LatencyRange>,
    pub polling_interval: Option<DurationNs>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LatencyRange {
    pub min_ns: u64,
    pub max_ns: u64,
    pub avg_ns: u64,
    pub p99_ns: Option<u64>,
    pub samples: usize,
}

impl LatencyRange {
    pub fn new(min_ns: u64, max_ns: u64, avg_ns: u64) -> Self {
        Self { min_ns, max_ns, avg_ns, p99_ns: None, samples: 1 }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DurationNs(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptSpec {
    pub vector: u8,
    pub owner: String,
    pub irq_type: IrqType,
    pub polarity: IrqPolarity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmaChannel {
    pub id: u8,
    pub src: u64,
    pub dst: u64,
    pub size: u64,
    pub block: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmaRequirement {
    pub required: bool,
    pub min_bandwidth_mbps: f64,
    pub alignment: u32,
    pub max_channels: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConstraints {
    pub max_power_watts: f64,
    pub required_bandwidths: Vec<BandwidthRequirement>,
    pub pin_count: Option<u32>,
    pub pcb_layers: Option<u8>,
    pub temp_range: Option<TempRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthRequirement {
    pub path: String,
    pub min_mbps: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TempRange {
    pub min_c: i16,
    pub max_c: i16,
}

// ─── Component Assignment (p/ Synthesis) ──────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentAssignment {
    pub block_id: String,
    pub component: String,
    pub interface: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizedSpec {
    pub original: HardwareSpec,
    pub assignments: Vec<ComponentAssignment>,
    pub netlist: Option<Vec<NetSegment>>,
    pub constraints: SynthesisConstraints,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NetSegment {
    pub from: String,
    pub to: String,
    pub signal: String,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisConstraints {
    pub max_bom_cost: Option<f64>,
    pub preferred_manufacturer: Option<String>,
    pub preferred_package: Option<String>,
}

impl HardwareSpec {
    /// Cria um HardwareSpec vazio para ser populado incrementalmente
    pub fn empty() -> Self {
        Self {
            version: 1,
            source: String::new(),
            cpu: CpuSpec {
                architecture: CpuArch::Unknown(String::new()),
                clock_mhz: 0,
                endianness: Endian::Little,
                cores: 1,
            },
            memory: MemoryLayout { regions: Vec::new() },
            blocks: Vec::new(),
            interrupts: Vec::new(),
            dma_channels: Vec::new(),
            constraints: SystemConstraints {
                max_power_watts: 0.0,
                required_bandwidths: Vec::new(),
                pin_count: None,
                pcb_layers: None,
                temp_range: None,
            },
            confidence: 0.0,
        }
    }

    /// Serializa para YAML string
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Serializa para JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Carrega de YAML string
    pub fn from_yaml(s: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }

    /// Carrega de JSON string
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
