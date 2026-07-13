use anyhow::Result;
use std::fs;
use std::path::Path;

use base_core::component_db::ComponentDb;
use base_core::inference::generate_spec;
use base_core::inference::extraction::{MmioAccess, MmioAccessType};
use base_core::mapping::mapper::ComponentMapper;
use base_core::mapping::netlist;
use base_core::spec::types::*;

use base_pcb::schematic::SchematicGenerator;
use base_pcb::bom::BomGenerator;
use base_pcb::pcb_layout::generate_pcb_layout;
use base_pcb::drc::KicadDrcValidator;

use base_fw::bootloader::BootloaderGenerator;
use base_fw::hal::HalGenerator;
use base_fw::timing::TimingCompensation;
use base_fw::irq::IrqGenerator;
use base_fw::drivers::DriverGenerator;
use base_fw::zephyr::ZephyrGenerator;

use base_check::tracer::TraceParser;
use base_check::compare::OperationComparator;
use base_check::metrics::ValidationThresholds;
use base_check::report::ReportGenerator;

use base_evolve::analyzer::BottleneckAnalyzer;
use base_evolve::tradeoff::TradeoffAnalyzer;
use base_evolve::migrate::MigrationPlanner;

use std::path::PathBuf;

use crate::cli::Command;

pub fn execute(cmd: &Command, output: &Path) -> Result<()> {
    match cmd {
        Command::Analyze { firmware, mmio_traces, classify, dot, disasm } => {
            handle_analyze(firmware, mmio_traces.as_deref(), classify.as_deref(), *dot, *disasm, output)?;
        }
        Command::Synth { input, component_db, max_bom_cost } => {
            handle_synth(input, component_db, *max_bom_cost, output)?;
        }
        Command::Pcb { input, project, drc } => {
            handle_pcb(input, project, *drc, output)?;
        }
        Command::Fw { input, target, zephyr } => {
            handle_fw(input, target, *zephyr, output)?;
        }
        Command::Check { input, original_trace, new_trace, max_latency, format } => {
            handle_check(input, original_trace, new_trace.as_deref(), *max_latency, format, output)?;
        }
        Command::Evolve { input, component_db, format } => {
            handle_evolve(input, component_db, format, output)?;
        }
        Command::Pipeline { firmware, trace, target, drc, zephyr, no_evolve, disasm } => {
            handle_pipeline(firmware, trace.as_deref(), target, *drc, *zephyr, *no_evolve, *disasm, output)?;
        }
        Command::Reconstruct { input, threshold, max_iterations, continuous, iter_output } => {
            handle_reconstruct(input, *threshold, *max_iterations, *continuous, *iter_output, output)?;
        }
        Command::Bir { input, compile, validate, to_legacy, dot } => {
            handle_bir(input, *compile, *validate, *to_legacy, *dot, output)?;
        }
    }
    Ok(())
}

// ─── Analyze ────────────────────────────────────────────

fn handle_analyze(firmware: &Path, _mmio_traces: Option<&Path>, _classify: Option<&str>, dot: bool, disasm: bool, output: &Path) -> Result<()> {
    tracing::info!("Reading firmware from {}", firmware.display());
    let data = fs::read(firmware)?;

    tracing::info!("Running behavioral inference on {} bytes", data.len());
    let mmio_accesses = if disasm {
        crate::disasm::analyze_with_disasm(&data)
    } else {
        mock_mmio_from_binary(&data)
    };
    let spec = generate_spec(&mmio_accesses, &firmware.to_string_lossy());

    fs::create_dir_all(output)?;
    let path = output.join("hardware_spec.yaml");
    fs::write(&path, spec.to_yaml()?)?;
    tracing::info!("HardwareSpec written to {}", path.display());

    if dot {
        let (beh_dot, ev_dot) = base_core::graphviz::generate_all(&spec, &firmware.to_string_lossy());
        let beh_path = output.join("behavior_graph.dot");
        fs::write(&beh_path, &beh_dot)?;
        tracing::info!("Behavior graph DOT written to {}", beh_path.display());
        let ev_path = output.join("event_graph.dot");
        fs::write(&ev_path, &ev_dot)?;
        tracing::info!("Event graph DOT written to {}", ev_path.display());
        tracing::info!("Render with: dot -Tpng -O <file>.dot");
    }

    Ok(())
}

fn mock_mmio_from_binary(data: &[u8]) -> Vec<MmioAccess> {
    let mut accesses = Vec::new();
    // Heuristic: look for 32-bit aligned values that look like MMIO addresses
    for chunk in data.chunks(4) {
        if chunk.len() == 4 {
            let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            if (val >= 0x10000000 && val <= 0x20000000) || (val >= 0xA0000000 && val <= 0xB0000000) {
                accesses.push(MmioAccess {
                    address: val as u64,
                    value: Some(1),
                    access_type: MmioAccessType::Write,
                    function_name: "detected".into(),
                    instruction_addr: 0,
                });
            }
        }
    }
    accesses.truncate(100); // limit
    accesses
}

// ─── Synth ──────────────────────────────────────────────

fn handle_synth(input: &Path, component_db: &Path, _max_bom_cost: Option<f64>, output: &Path) -> Result<()> {
    tracing::info!("Loading HardwareSpec from {}", input.display());
    let yaml = fs::read_to_string(input)?;
    let spec = HardwareSpec::from_yaml(&yaml)?;

    let mut db = ComponentDb::new();
    if component_db.exists() {
        db.load_directory(component_db)?;
        tracing::info!("Loaded {} components", db.len());
    }

    let mapper = ComponentMapper::new(db);
    let synthesized = mapper.map_spec(&spec);

    let netlist_segments = base_core::mapping::netlist::generate_netlist(
        &synthesized,
        &ComponentDb::new(),
    );
    let mut synthesized = synthesized;
    synthesized.netlist = Some(netlist_segments);

    fs::create_dir_all(output)?;
    let path = output.join("synthesized_spec.yaml");
    fs::write(&path, serde_yaml::to_string(&synthesized)?)?;
    tracing::info!("SynthesizedSpec written to {}", path.display());
    Ok(())
}

// ─── PCB ────────────────────────────────────────────────

fn handle_pcb(input: &Path, project: &str, drc: bool, output: &Path) -> Result<()> {
    let yaml = fs::read_to_string(input)?;
    let spec: SynthesizedSpec = serde_yaml::from_str(&yaml)?;

    fs::create_dir_all(output)?;

    // Schematic
    let sch_gen = SchematicGenerator::new(None);
    let sch = sch_gen.generate(&spec);
    let sch_path = output.join(format!("{}.kicad_sch", project));
    fs::write(&sch_path, &sch)?;
    tracing::info!("Schematic written to {}", sch_path.display());

    // BOM
    let bom_gen = BomGenerator;
    let bom = bom_gen.generate(&spec);
    let bom_path = output.join("bom.csv");
    fs::write(&bom_path, &bom)?;
    tracing::info!("BOM written to {}", bom_path.display());

    // PCB layout
    let netlist = spec.netlist.as_deref().unwrap_or(&[]);
    let pcb = generate_pcb_layout(&spec, netlist);
    let pcb_path = output.join(format!("{}.kicad_pcb", project));
    fs::write(&pcb_path, &pcb)?;
    tracing::info!("PCB layout written to {}", pcb_path.display());

    // DRC validation
    if drc {
        if KicadDrcValidator::is_available() {
            tracing::info!("Running kicad-cli DRC...");
            match KicadDrcValidator::run_sch_drc(sch_path.to_str().unwrap()) {
                Ok(r) => tracing::info!("Schematic DRC: {}", r),
                Err(e) => tracing::warn!("Schematic DRC failed: {}", e),
            }
            match KicadDrcValidator::run_pcb_drc(pcb_path.to_str().unwrap()) {
                Ok(v) => tracing::info!("PCB DRC: {} violations", v.len()),
                Err(e) => tracing::warn!("PCB DRC failed: {}", e),
            }
        } else {
            tracing::warn!("kicad-cli not found, skipping DRC");
        }
    }

    // Template application
    let templates = base_pcb::templates::TemplateLibrary::new();
    let _tmpl_sch = templates.apply_template(&spec);
    tracing::info!("Applied {} templates", templates.len());

    // CI script
    let ci = KicadDrcValidator::generate_ci_script(project);
    fs::write(output.join("check_drc.sh"), &ci)?;

    Ok(())
}

// ─── FW ─────────────────────────────────────────────────

fn handle_fw(input: &Path, target: &str, zephyr: bool, output: &Path) -> Result<()> {
    let yaml = fs::read_to_string(input)?;
    let spec: SynthesizedSpec = serde_yaml::from_str(&yaml)?;

    fs::create_dir_all(output)?;

    // Bootloader
    let bl_gen = BootloaderGenerator;
    let bl = bl_gen.generate(&spec);
    fs::write(output.join("bootloader.c"), &bl)?;

    // HAL
    let hal_gen = HalGenerator;
    let hal = hal_gen.generate(&spec, target);
    fs::write(output.join("hal_mmio.c"), &hal)?;

    // Timing
    let tim_gen = TimingCompensation;
    let tim = tim_gen.generate(&spec);
    fs::write(output.join("timing.c"), &tim)?;

    // IRQ
    let irq_gen = IrqGenerator;
    let irq = irq_gen.generate(&spec);
    fs::write(output.join("irq.c"), &irq)?;

    // Drivers
    let drv_gen = DriverGenerator;
    let drv = drv_gen.generate_baremetal(&spec);
    fs::write(output.join("drivers.c"), &drv)?;

    let mk = drv_gen.generate_build_system(&spec);
    fs::write(output.join("Makefile"), &mk)?;

    let ld = drv_gen.generate_linker_script(&spec);
    fs::write(output.join("linker.ld"), &ld)?;

    tracing::info!("Firmware generated in {}", output.display());

    // Zephyr module
    if zephyr {
        let zeph_gen = ZephyrGenerator;
        let module = zeph_gen.generate_module(&spec);
        let zephyr_dir = output.join("zephyr");
        fs::create_dir_all(&zephyr_dir)?;
        for (name, content) in &module {
            fs::write(zephyr_dir.join(name), content)?;
        }
        tracing::info!("Zephyr module written to {}", zephyr_dir.display());
    }

    Ok(())
}

// ─── Check ──────────────────────────────────────────────

fn handle_check(
    input: &Path,
    original_trace: &Path,
    new_trace: Option<&Path>,
    max_latency: f64,
    format: &str,
    output: &Path,
) -> Result<()> {
    let yaml = fs::read_to_string(input)?;
    let spec: SynthesizedSpec = serde_yaml::from_str(&yaml)?;

    let original = TraceParser::parse(original_trace)?;
    tracing::info!("Parsed original trace: {} events", original.events.len());

    let actual = match new_trace {
        Some(p) => {
            let t = TraceParser::parse(p)?;
            tracing::info!("Parsed new trace: {} events", t.events.len());
            t
        }
        None => original.clone(), // compare with itself for baseline
    };

    let thresholds = ValidationThresholds {
        max_latency_ratio: max_latency,
        ..ValidationThresholds::default()
    };

    let items = OperationComparator::compare(
        &original, &actual, &spec.original, &thresholds,
    );

    let gen = ReportGenerator;
    fs::create_dir_all(output)?;

    match format {
        "json" => {
            let json = gen.generate_json(&items, &spec.original.source);
            fs::write(output.join("validation_report.json"), &json)?;
        }
        _ => {
            let html = gen.generate_html(&items, &spec.original.source);
            fs::write(output.join("validation_report.html"), &html)?;
        }
    }

    let passed = items.iter().filter(|i| i.passed).count();
    let total = items.len();
    tracing::info!("Validation: {}/{} passed ({:.1}%)", passed, total, passed as f64 / total.max(1) as f64 * 100.0);

    Ok(())
}

// ─── Evolve ─────────────────────────────────────────────

fn handle_evolve(input: &Path, component_db: &Path, format: &str, output: &Path) -> Result<()> {
    let yaml = fs::read_to_string(input)?;
    let spec: SynthesizedSpec = serde_yaml::from_str(&yaml)?;

    let mut db = ComponentDb::new();
    if component_db.exists() {
        db.load_directory(component_db)?;
        tracing::info!("Loaded {} components", db.len());
    }

    let analyzer = BottleneckAnalyzer::new(db);
    let bottlenecks = analyzer.analyze(&spec);
    tracing::info!("Found {} bottlenecks", bottlenecks.len());

    let tradeoff_analyzer = TradeoffAnalyzer;
    let tradeoffs = tradeoff_analyzer.evaluate_all(&bottlenecks, &spec);

    let planner = MigrationPlanner;
    let plan = planner.generate_plan(&bottlenecks, &tradeoffs, &spec);

    fs::create_dir_all(output)?;
    match format {
        "yaml" => {
            let content = planner.to_yaml(&plan);
            fs::write(output.join("evolution_plan.yaml"), &content)?;
        }
        _ => {
            let md = planner.to_markdown(&plan);
            fs::write(output.join("evolution_plan.md"), &md)?;
        }
    }

    tracing::info!("Evolution plan written to {}", output.display());
    Ok(())
}

// ─── Pipeline ───────────────────────────────────────────

fn handle_pipeline(
    firmware: &Path,
    trace: Option<&Path>,
    target: &str,
    drc: bool,
    zephyr: bool,
    no_evolve: bool,
    disasm: bool,
    output: &Path,
) -> Result<()> {
    tracing::info!("=== B.A.S.E. Pipeline ===");

    // Step 1: Analyze
    tracing::info!("[1/6] Analyzing firmware...");
    handle_analyze(firmware, None, None, true, true, &output.join("01_analyze"))?;

    // Step 2: Synth
    tracing::info!("[2/6] Synthesizing hardware mapping...");
    let component_db_path = Path::new("base-core/component_db");
    handle_synth(
        &output.join("01_analyze/hardware_spec.yaml"),
        component_db_path,
        None,
        &output.join("02_synth"),
    )?;

    // Step 3: PCB
    tracing::info!("[3/6] Generating PCB...");
    handle_pcb(
        &output.join("02_synth/synthesized_spec.yaml"),
        "project",
        drc,
        &output.join("03_pcb"),
    )?;

    // Step 4: FW
    tracing::info!("[4/6] Generating firmware...");
    handle_fw(
        &output.join("02_synth/synthesized_spec.yaml"),
        target,
        zephyr,
        &output.join("04_fw"),
    )?;

    // Step 5: Check
    tracing::info!("[5/6] Validating...");
    if let Some(trace_path) = trace {
        handle_check(
            &output.join("02_synth/synthesized_spec.yaml"),
            trace_path,
            None,
            2.0,
            "html",
            &output.join("05_validation"),
        )?;
    } else {
        tracing::warn!("Skipping validation (no trace provided)");
    }

    // Step 6: Evolve
    if !no_evolve {
        tracing::info!("[6/6] Analyzing evolution...");
        handle_evolve(
            &output.join("02_synth/synthesized_spec.yaml"),
            component_db_path,
            "md",
            &output.join("06_evolution"),
        )?;
    } else {
        tracing::info!("[6/6] Skipping evolution (--no-evolve)");
    }

    tracing::info!("=== Pipeline complete ===");
    Ok(())
}

fn handle_bir(input: &Path, compile: bool, validate: bool, to_legacy: bool, dot: bool, output: &Path) -> Result<()> {
    let content = fs::read_to_string(input)?;
    fs::create_dir_all(output)?;

    if compile {
        // BSL → BIR compilation requires base-bsl feature
        anyhow::bail!("BSL compilation: add base-bsl dependency or use `base bir input.bir.yaml --validate`");
    }

    // Load BIR YAML for operations
    let device = base_bir::types::BirDevice::from_yaml(&content)
        .map_err(|e| anyhow::anyhow!("Invalid BIR YAML: {}", e))?;

    if validate {
        let result = device.validate();
        tracing::info!("BIR validation: {} errors, {} warnings",
            result.errors.len(), result.warnings.len());
        for err in &result.errors {
            tracing::warn!("  BIR error: {} ({})", err.message, err.location.as_deref().unwrap_or("?"));
        }
        for w in &result.warnings {
            tracing::warn!("  BIR warning: {}", w);
        }
        let vpath = output.join("bir_validation.json");
        fs::write(&vpath, serde_json::to_string_pretty(&result)?)?;
        tracing::info!("Validation report: {}", vpath.display());
    }

    if to_legacy {
        let spec = crate::disasm::bir_to_legacy(&device);
        let path = output.join("hardware_spec.yaml");
        fs::write(&path, spec.to_yaml()?)?;
        tracing::info!("HardwareSpec written to {}", path.display());
    }

    if dot {
        let dot_content = crate::disasm::bir_to_dot(&device, &input.to_string_lossy());
        let path = output.join("bir_graph.dot");
        fs::write(&path, dot_content)?;
        tracing::info!("BIR DOT graph written to {}", path.display());
    }

    Ok(())
}

fn handle_reconstruct(input: &Path, threshold: f64, max_iterations: usize, continuous: bool, verbose: bool, output: &Path) -> Result<()> {
    let yaml = fs::read_to_string(input)?;
    let spec = base_core::spec::types::HardwareSpec::from_yaml(&yaml)?;

    fs::create_dir_all(output)?;
    tracing::info!("=== B.A.S.E. Reconstruction Loop ===");
    tracing::info!("Threshold: {:.0}%, Max iterations: {}, Continuous: {}",
        threshold * 100.0, max_iterations, continuous);

    let mut loop_ = base_core::loop_::FeedbackLoop::new(threshold, max_iterations);
    let iterations = loop_.run(&spec);

    for iter in &iterations {
        if verbose {
            let iter_dir = output.join(format!("iter_{:03}", iter.number));
            fs::create_dir_all(&iter_dir)?;
            let spec_path = iter_dir.join("hardware_spec.yaml");
            fs::write(&spec_path, iter.spec.to_yaml()?)?;
        }
    }

    let report = loop_.convergence_report();
    let report_json = serde_json::json!({
        "total_iterations": report.total_iterations,
        "initial_pass_rate": report.initial_pass_rate,
        "final_pass_rate": report.final_pass_rate,
        "improvement": report.improvement,
        "total_errors_found": report.total_errors_found,
        "avg_errors_per_iteration": report.avg_errors_per_iteration,
        "converged": report.converged,
        "threshold": threshold,
    });
    fs::write(output.join("convergence_report.json"), serde_json::to_string_pretty(&report_json)?)?;

    if let Some(last) = iterations.last() {
        fs::write(output.join("hardware_spec_refined.yaml"), last.spec.to_yaml()?)?;
    }

    if report.converged {
        tracing::info!("✅ Converged in {} iterations", report.total_iterations);
    } else {
        tracing::info!("⚠️ Did not converge — {:.1}% < {:.1}%",
            report.final_pass_rate * 100.0, threshold * 100.0);
    }

    Ok(())
}
