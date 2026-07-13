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

        /// Manual block classification override
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

        /// New hardware trace file (CSV or JSON)
        new_trace: Option<PathBuf>,

        /// Max allowed latency ratio
        #[arg(long, default_value = "2.0")]
        max_latency: f64,

        /// Output report format (html, json)
        #[arg(long, default_value = "html")]
        format: String,
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

    /// Run full pipeline: analyze → synth → pcb → fw → check → evolve
    Pipeline {
        /// Firmware file or directory to analyze
        firmware: PathBuf,

        /// Original trace for validation
        #[arg(long)]
        trace: Option<PathBuf>,

        /// Target platform
        #[arg(long, default_value = "rp2350")]
        target: String,

        /// Run kicad-cli DRC validation
        #[arg(long)]
        drc: bool,

        /// Generate Zephyr module
        #[arg(long)]
        zephyr: bool,

        /// Skip evolution step
        #[arg(long)]
        no_evolve: bool,

        /// Use Capstone disassembly (real) instead of heuristic scan
        #[arg(long, default_value_t = true)]
        disasm: bool,
    },
}
