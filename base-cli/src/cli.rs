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

    /// Port package: address/driver map + fossils + atlas (≠ OS rewrite)
    Port {
        #[command(subcommand)]
        action: PortCommand,
    },

    /// Paleocomputação: StratAlign + excavate (PDF §7–§8) — ≠ PaleoCLI product / ≠ auto-fix
    Paleo {
        #[command(subcommand)]
        action: PaleoCommand,
    },
}

/// `base paleo` — algoritmos da Paleocomputação Estrutural (assist)
#[derive(Subcommand)]
pub enum PaleoCommand {
    /// StratAlign: alinhar duas sequências fósseis (Evidence YAML ou FossilSequence YAML)
    Align {
        /// Sequência A / EvidenceDb YAML (referência / estrato X)
        a: PathBuf,

        /// Sequência B / EvidenceDb YAML (artefato)
        b: PathBuf,
    },

    /// Pipeline Ω → Ψ [+ StratAlign] → atlas
    Excavate {
        /// HardwareSpec YAML
        input: PathBuf,

        /// Evidence DB YAML
        #[arg(long)]
        evidence: PathBuf,

        /// Optional reference EvidenceDb for StratAlign
        #[arg(long)]
        reference: Option<PathBuf>,

        #[arg(long, default_value_t = 0)]
        functions: usize,

        #[arg(long, default_value_t = 0)]
        instructions: usize,

        #[arg(long, default_value_t = 0)]
        calls: usize,
    },

    /// Filogenia N-a-N: G(B), d_φ, Neighbor-Joining, THC/homoplasia → Newick
    Phylo {
        /// EvidenceDb YAML files (≥2) — linhagem / ports / forks
        evidence: Vec<PathBuf>,

        /// Optional HardwareSpec YAML (same order as evidence) for phenotype Φ
        #[arg(long)]
        spec: Vec<PathBuf>,

        /// Optional stratum Δt per taxon (same order); default 1,2,3…
        #[arg(long)]
        delta_t: Vec<f64>,
    },
}

/// `base port` — HAL/driver port assist
#[derive(Subcommand)]
pub enum PortCommand {
    /// Build port package from analyze artefacts
    Package {
        /// HardwareSpec YAML
        input: PathBuf,

        /// Evidence DB YAML (from analyze)
        #[arg(long)]
        evidence: Option<PathBuf>,

        /// Tension report JSON (from analyze)
        #[arg(long)]
        tension: Option<PathBuf>,

        /// Abstract HAL target name
        #[arg(long, default_value = "hal_abstract_v1")]
        target_hal: String,

        /// Also emit host HAL C stub via base-fw (optional)
        #[arg(long, default_value_t = false)]
        hal_stub: bool,

        /// Device Tree blob or DTBO/vendor_boot containing FDT(s)
        #[arg(long)]
        dtb: Option<PathBuf>,

        /// Optional flash.cfg for product hints (Unisoc PAC)
        #[arg(long)]
        flash_cfg: Option<PathBuf>,
    },

    /// OS-port platform inventory from DTB (CPU/GIC/timer/UART/…)
    Platform {
        /// DTB, DTBO, or image with embedded FDT
        input: PathBuf,

        /// Optional flash.cfg
        #[arg(long)]
        flash_cfg: Option<PathBuf>,
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

    /// Attempt flash — **never** `mode=production`; use `--live` for USB+CMD (no mock)
    Flash {
        /// Firmware image to flash (or dry-run)
        image: PathBuf,

        #[arg(long, default_value = "0xcafe")]
        vid: String,

        #[arg(long, default_value = "0x4007")]
        pid: String,

        /// Force ProbePresence::Detected offline (rehearsal; refused with `--live`)
        #[arg(long)]
        mock_detected: bool,

        /// Dry-run receipt (`mock_dry_run`) — no silicon (refused with `--live`)
        #[arg(long)]
        mock_flash: bool,

        /// USB+programmer only — no mock; implies auto-probe; sets lab_assist receipt if SOW envs
        #[arg(long, default_value_t = false)]
        live: bool,

        /// Scan known USB probes / BASE_HIL_PROBE_IDS (implied by `--live`)
        #[arg(long, default_value_t = false)]
        auto_probe: bool,
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

        /// Force A1 Detected offline (rehearsal; refused with `--live`)
        #[arg(long, default_value_t = false)]
        mock_detected: bool,

        /// USB-only Gate A (no mock); auto-probe on
        #[arg(long, default_value_t = false)]
        live: bool,

        /// Scan known probes (implied by `--live`)
        #[arg(long, default_value_t = false)]
        auto_probe: bool,

        /// Operator asserts SOW §HIL signed (Gate A5) — do not lie
        #[arg(long, default_value_t = false)]
        sow_signed: bool,
    },
}
