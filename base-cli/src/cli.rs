use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "base", version, about = "B.A.S.E. — Behavioral ASIC Synthesis Engine")]
#[command(long_about = "Transform hardware behavior into new PCB + firmware.
  Pipeline: analyze → synth → pcb → fw → check → evolve")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Verbose output (-v, -vv)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Output directory
    #[arg(short = 'o', long = "output", default_value = "output", global = true)]
    pub output: PathBuf,
}

#[derive(Subcommand)]
pub enum Command {
    /// Analyze firmware → produce HardwareSpec
    Analyze {
        /// Firmware file (.zip, .bin) or directory
        firmware: PathBuf,

        /// MMIO discovery input (optional JSON)
        #[arg(long)]
        mmio_traces: Option<PathBuf>,

        /// Manual block classification: `uart` (all blocks) or
        /// `0x40034000=uart,0x4003c000=spi` (per 4K page)
        #[arg(long)]
        classify: Option<String>,

        /// Export Graphviz DOT of behavioral graph
        #[arg(long)]
        dot: bool,

        /// Use Capstone disassembly (real) instead of heuristic binary scan
        #[arg(long)]
        disasm: bool,
    },

    /// Synthesize HardwareSpec → component mapping
    Synth {
        /// Input HardwareSpec YAML
        input: PathBuf,

        /// Component DB directory
        #[arg(long, default_value = "base-core/component_db")]
        component_db: PathBuf,

        /// Max BOM cost (USD)
        #[arg(long)]
        max_bom_cost: Option<f64>,

        /// Prefer manufacturer substring (e.g. STMicroelectronics)
        #[arg(long)]
        preferred_manufacturer: Option<String>,
    },

    /// Generate PCB (KiCad) from SynthesizedSpec
    Pcb {
        /// Input SynthesizedSpec YAML
        input: PathBuf,

        /// Project name
        #[arg(long, default_value = "project")]
        project: String,

        /// Run kicad-cli DRC
        #[arg(long)]
        drc: bool,
    },

    /// Generate firmware (bootloader, HAL, drivers, Zephyr)
    Fw {
        /// Input SynthesizedSpec YAML
        input: PathBuf,

        /// Target platform (rp2350, cortex-a)
        #[arg(long, default_value = "rp2350")]
        target: String,

        /// Generate Zephyr module
        #[arg(long)]
        zephyr: bool,
    },

    /// Validate new hardware against original traces
    Check {
        /// SynthesizedSpec YAML
        input: PathBuf,

        /// Original trace file (CSV or JSON)
        original_trace: PathBuf,

        /// New hardware trace file (CSV or JSON). Required for dual compare.
        /// Sem isto o check NÃO self-compara — gera relatório skipped + WARN.
        new_trace: Option<PathBuf>,

        /// Max allowed latency ratio
        #[arg(long, default_value = "2.0")]
        max_latency: f64,

        /// Output report format (html, json)
        #[arg(long, default_value = "html")]
        format: String,

        /// Fail if `new_trace` is missing (no silent skip / no self-pass)
        #[arg(long, default_value_t = false)]
        strict: bool,
    },

    /// Analyze bottlenecks and suggest upgrades
    Evolve {
        /// SynthesizedSpec YAML
        input: PathBuf,

        /// Component DB directory
        #[arg(long, default_value = "base-core/component_db")]
        component_db: PathBuf,

        /// Output format (yaml, md)
        #[arg(long, default_value = "md")]
        format: String,
    },

    /// Run full pipeline: analyze → synth → design → fw → [check] → [pcb] → [evolve]
    Pipeline {
        /// Firmware file or directory to analyze
        firmware: PathBuf,

        /// Original trace for validation
        #[arg(long)]
        trace: Option<PathBuf>,

        /// New hardware trace (dual compare). Sem isto, check é skipped (WARN).
        #[arg(long)]
        new_trace: Option<PathBuf>,

        /// Fail pipeline check if `--trace` exists but `--new-trace` missing
        #[arg(long, default_value_t = false)]
        strict: bool,

        /// Target platform
        #[arg(long, default_value = "rp2350")]
        target: String,

        /// Opt-in: generate KiCad PCB draft (engineering_draft — NOT FABRICABLE)
        #[arg(long, default_value_t = false)]
        pcb: bool,

        /// Run kicad-cli DRC validation (requires --pcb)
        #[arg(long)]
        drc: bool,

        /// Generate Zephyr module
        #[arg(long)]
        zephyr: bool,

        /// Opt-in: run evolution engine (REAL* metrics — off by default)
        #[arg(long, default_value_t = false)]
        evolve: bool,

        /// Use Capstone disassembly (real) instead of heuristic scan
        #[arg(long, default_value_t = true)]
        disasm: bool,
    },

    /// Reconstruct: structural refinement loop (evidence-local — not full auto-fix)
    Reconstruct {
        /// Input HardwareSpec YAML
        input: PathBuf,

        /// Convergence threshold (0.0 — 1.0)
        #[arg(long, default_value = "0.9")]
        threshold: f64,

        /// Maximum iterations (ignored floor when --continuous raises the cap)
        #[arg(long, default_value_t = 10)]
        max_iterations: usize,

        /// Raise iteration cap to 1000; still stops on converge/stagnation — NOT infinite auto-fix
        #[arg(long)]
        continuous: bool,

        /// Output every iteration (detailed logs)
        #[arg(long)]
        iter_output: bool,
    },

    /// Replay trace against contracts
    Replay {
        /// Trace CSV file
        trace: PathBuf,

        /// Contracts YAML file (optional, extracted from BIR if not provided)
        #[arg(long)]
        contracts: Option<PathBuf>,

        /// BIR YAML file (to extract contracts)
        #[arg(long)]
        bir: Option<PathBuf>,

        /// Output violations JSON
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Prove contracts via SMT (Z3)
    Prove {
        /// Contracts YAML file
        contracts: PathBuf,

        /// Output SMT-LIB file
        #[arg(long)]
        smt_output: Option<PathBuf>,

        /// Prove deadlock freedom
        #[arg(long)]
        deadlock: bool,
    },

    /// Generate reference design from HardwareSpec
    Design {
        /// Input HardwareSpec YAML
        input: PathBuf,

        /// Generate PCB (engineering draft)
        #[arg(long)]
        pcb: bool,

        /// Max BOM cost (USD) — passed to mapper
        #[arg(long)]
        max_bom_cost: Option<f64>,

        /// Prefer manufacturer substring (e.g. STMicroelectronics)
        #[arg(long)]
        preferred_manufacturer: Option<String>,
    },

    /// Export event graph (causal) from contracts + trace
    EventGraph {
        /// Contracts YAML file
        contracts: PathBuf,

        /// Trace CSV file
        trace: PathBuf,

        /// Output format (dot, mermaid)
        #[arg(long, default_value = "dot")]
        format: String,
    },

    /// BIR: Behavioral IR manipulation
    Bir {
        /// Input file (BSL source, BIR YAML, or firmware)
        input: PathBuf,

        /// Compile BSL → BIR
        #[arg(long)]
        compile: bool,

        /// Validate BIR
        #[arg(long)]
        validate: bool,

        /// Convert BIR → HardwareSpec (legacy)
        #[arg(long)]
        to_legacy: bool,

        /// Export Graphviz DOT
        #[arg(long)]
        dot: bool,
    },

    /// HIL probe host agent — host REAL*; production gated (not in pipeline default)
    Hil {
        #[command(subcommand)]
        action: HilCommand,
    },

    /// Specter VM study: Forth-like loop + Lua policy (autonomous structural refine — ≠ auto-fix)
    Study {
        /// Input HardwareSpec YAML
        input: PathBuf,

        /// Lua policy file (default embedded)
        #[arg(long)]
        policy: Option<PathBuf>,

        /// Forth program for each step (default OBSERVE SCORE REFINE …)
        #[arg(long)]
        program: Option<PathBuf>,
    },
}

/// `base hil` subcommands — thin wrapper over `base-hil`.
#[derive(Subcommand)]
pub enum HilCommand {
    /// Enumerate probe presence for VID:PID (default Simulated without hil_usb / mock env)
    Enumerate {
        /// USB vendor id (hex), default 0xCAFE
        #[arg(long, default_value = "0xcafe")]
        vid: String,

        /// USB product id (hex), default 0x4007
        #[arg(long, default_value = "0x4007")]
        pid: String,
    },

    /// Attempt flash / dry-run — **never** production; gates match `base-hil`
    Flash {
        /// Firmware image to flash (or dry-run)
        image: PathBuf,

        #[arg(long, default_value = "0xcafe")]
        vid: String,

        #[arg(long, default_value = "0x4007")]
        pid: String,

        /// Force ProbePresence::Detected offline (like BASE_HIL_MOCK_DETECTED)
        #[arg(long)]
        mock_detected: bool,

        /// Dry-run receipt (`mock_dry_run`) — no silicon
        #[arg(long)]
        mock_flash: bool,
    },

    /// Industrial Gate A — report HIL lab pré-condições (never production)
    LabStatus {
        #[arg(long, default_value = "0xcafe")]
        vid: String,

        #[arg(long, default_value = "0x4007")]
        pid: String,

        /// Path to lab SOP.md (Gate A3)
        #[arg(long)]
        sop: Option<PathBuf>,

        /// Force A1 Detected offline (lab rehearsal; ≠ USB real)
        #[arg(long, default_value_t = false)]
        mock_detected: bool,

        /// Operator asserts SOW §HIL signed (Gate A5) — do not lie
        #[arg(long, default_value_t = false)]
        sow_signed: bool,
    },
}
