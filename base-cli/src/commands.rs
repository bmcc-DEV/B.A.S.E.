use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use base_core::component_db::ComponentDb;
use base_core::inference::extraction::{MmioAccess, MmioAccessType};
use base_core::mapping::mapper::ComponentMapper;
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
use base_check::report::{ReportContext, ReportGenerator};

use base_evolve::analyzer::BottleneckAnalyzer;
use base_evolve::tradeoff::TradeoffAnalyzer;
use base_evolve::migrate::MigrationPlanner;

use crate::cli::{Command, HilCommand, PaleoCommand, PortCommand, ReasonCommand, VirtCommand};

pub fn execute(cmd: &Command, output: &Path) -> Result<()> {
    match cmd {
        Command::Analyze { firmware, mmio_traces, classify, dot, disasm } => {
            handle_analyze(firmware, mmio_traces.as_deref(), classify.as_deref(), *dot, *disasm, output)?;
        }
        Command::Synth {
            input,
            component_db,
            max_bom_cost,
            preferred_manufacturer,
        } => {
            handle_synth(
                input,
                component_db,
                *max_bom_cost,
                preferred_manufacturer.as_deref(),
                output,
            )?;
        }
        Command::Pcb { input, project, drc } => {
            handle_pcb(input, project, *drc, output)?;
        }
        Command::Fw { input, target, zephyr } => {
            handle_fw(input, target, *zephyr, output)?;
        }
        Command::Check {
            input,
            original_trace,
            new_trace,
            max_latency,
            format,
            strict,
        } => {
            handle_check(
                input,
                original_trace,
                new_trace.as_deref(),
                *max_latency,
                format,
                *strict,
                output,
            )?;
        }
        Command::Evolve { input, component_db, format } => {
            handle_evolve(input, component_db, format, output)?;
        }
        Command::Pipeline {
            firmware,
            trace,
            new_trace,
            strict,
            target,
            pcb,
            drc,
            zephyr,
            evolve,
            disasm,
        } => {
            handle_pipeline(
                firmware,
                trace.as_deref(),
                new_trace.as_deref(),
                *strict,
                target,
                *pcb,
                *drc,
                *zephyr,
                *evolve,
                *disasm,
                output,
            )?;
        }
        Command::Reconstruct { input, threshold, max_iterations, continuous, iter_output } => {
            handle_reconstruct(input, *threshold, *max_iterations, *continuous, *iter_output, output)?;
        }
        Command::Replay { trace, contracts, bir, output: rp_output } => {
            handle_replay(trace.as_path(), contracts.clone(), bir.clone(), rp_output.clone(), output)?;
        }
        Command::Prove { contracts, smt_output, deadlock } => {
            handle_prove(contracts.as_path(), smt_output.clone(), *deadlock, output)?;
        }
        Command::Design {
            input,
            pcb,
            max_bom_cost,
            preferred_manufacturer,
        } => {
            handle_design(
                input.as_path(),
                *pcb,
                *max_bom_cost,
                preferred_manufacturer.as_deref(),
                output,
            )?;
        }
        Command::EventGraph { contracts, trace, format } => {
            handle_event_graph(contracts.as_path(), trace.as_path(), &format, output)?;
        }
        Command::Bir { input, compile, validate, to_legacy, dot } => {
            handle_bir(input, *compile, *validate, *to_legacy, *dot, output)?;
        }
        Command::Hil { action } => {
            handle_hil(action, output)?;
        }
        Command::Study { input, policy, program, evidence } => {
            handle_study(input, policy.as_deref(), program.as_deref(), evidence.as_deref(), output)?;
        }
        Command::Port { action } => {
            handle_port(action, output)?;
        }
        Command::Paleo { action } => {
            handle_paleo(action, output)?;
        }
        Command::Virt { action } => {
            handle_virt(action, output)?;
        }
        Command::Reason { action } => {
            handle_reason(action, output)?;
        }
    }
    Ok(())
}

// ─── Analyze ────────────────────────────────────────────

fn handle_analyze(firmware: &Path, mmio_traces: Option<&Path>, classify: Option<&str>, dot: bool, disasm: bool, output: &Path) -> Result<()> {
    tracing::info!("Reading firmware from {}", firmware.display());
    let data = fs::read(firmware)?;

    tracing::info!("Running behavioral inference on {} bytes", data.len());
    let mut mmio_accesses = if let Some(traces) = mmio_traces {
        load_mmio_traces(traces)?
    } else if disasm {
        crate::disasm::analyze_with_disasm(&data)
    } else {
        tracing::warn!("Heuristic MMIO scan (no --disasm / --mmio-traces)");
        let accesses = mock_mmio_from_binary(&data);
        tracing::warn!("Heuristic candidates: {}", accesses.len());
        accesses
    };

    if let Some(kind) = classify {
        tracing::info!("Applying classify override: {}", kind);
        prefix_mmio_by_classify(&mut mmio_accesses, kind);
    }

    let (mut spec, evidence) = base_core::inference::generate_spec_with_evidence(
        &mmio_accesses,
        &firmware.to_string_lossy(),
    );
    if let Some(kind) = classify {
        apply_classify_override(&mut spec, kind);
        // Recompute evidence-based confidence after kind override
        for b in &mut spec.blocks {
            b.confidence = base_core::loop_::evidence_confidence(b);
        }
        if !spec.blocks.is_empty() {
            spec.confidence = spec.blocks.iter().map(|b| b.confidence).sum::<f64>()
                / spec.blocks.len() as f64;
        }
    }

    fs::create_dir_all(output)?;
    let path = output.join("hardware_spec.yaml");
    fs::write(&path, spec.to_yaml()?)?;
    tracing::info!(
        "HardwareSpec written to {} ({} blocks, confidence={:.2})",
        path.display(),
        spec.blocks.len(),
        spec.confidence
    );

    let ev_path = output.join("evidence_db.yaml");
    fs::write(&ev_path, evidence.to_yaml()?)?;
    tracing::info!(
        "Evidence DB written to {} ({} entries)",
        ev_path.display(),
        evidence.entries.len()
    );

    // Tension Ψ — auditável (Path to Real R4)
    let fn_count = {
        let mut names = std::collections::HashSet::new();
        for a in &mmio_accesses {
            names.insert(a.function_name.clone());
        }
        names.len()
    };
    let tension = base_core::tension::TensionMetric::compute(
        &evidence,
        &spec,
        fn_count,
        data.len(),
        0,
    );
    let tension_path = output.join("tension_report.json");
    fs::write(
        &tension_path,
        base_core::tension::TensionMetric::to_json(&tension)?,
    )?;
    tracing::info!(
        "Tension Ψ written to {} (ψ={:.4}, confidence={:.2}%, {:?})",
        tension_path.display(),
        tension.overall_tension,
        tension.overall_confidence * 100.0,
        tension.conclusiveness
    );

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

fn load_mmio_traces(path: &Path) -> Result<Vec<MmioAccess>> {
    let text = fs::read_to_string(path)?;
    let accesses: Vec<MmioAccess> = if path.extension().and_then(|e| e.to_str()) == Some("yaml")
        || path.extension().and_then(|e| e.to_str()) == Some("yml")
    {
        serde_yaml::from_str(&text)?
    } else {
        serde_json::from_str(&text)?
    };
    tracing::info!("Loaded {} MMIO accesses from {}", accesses.len(), path.display());
    Ok(accesses)
}

/// `--classify uart` → todos os blocos.
/// `--classify 0x40034000=uart,0x4003c000=spi` → por página 4K (T1 B2).
fn apply_classify_override(spec: &mut HardwareSpec, kind: &str) {
    if let Some(map) = parse_classify_address_map(kind) {
        for block in &mut spec.blocks {
            let page = block.base_address & !0xfff;
            if let Some(block_kind) = map.get(&page) {
                block.kind = *block_kind;
                block.confidence = (block.confidence + 0.25).min(0.95);
            }
        }
        return;
    }
    let Some(block_kind) = parse_block_kind_name(kind) else {
        return;
    };
    for block in &mut spec.blocks {
        block.kind = block_kind;
        block.confidence = (block.confidence + 0.25).min(0.95);
    }
}

fn prefix_mmio_by_classify(accesses: &mut [MmioAccess], kind: &str) {
    if let Some(map) = parse_classify_kind_labels(kind) {
        for a in accesses {
            let page = a.address & !0xfff;
            if let Some(label) = map.get(&page) {
                a.function_name = format!("{}_{}", label, a.function_name);
            }
        }
        return;
    }
    for a in accesses {
        a.function_name = format!("{}_{}", kind, a.function_name);
    }
}

fn parse_block_kind_name(kind: &str) -> Option<BlockKind> {
    Some(match kind.to_lowercase().as_str() {
        "gpu" => BlockKind::Gpu,
        "audio" => BlockKind::Audio,
        "dma" => BlockKind::Dma,
        "usb" => BlockKind::Usb,
        "uart" => BlockKind::Uart,
        "spi" => BlockKind::Spi,
        "i2c" => BlockKind::I2c,
        "timer" | "tim" => BlockKind::Timer,
        "ethernet" => BlockKind::Ethernet,
        _ => return None,
    })
}

fn parse_u64_addr(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(hex) = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).ok()
    } else {
        s.parse().ok()
    }
}

fn parse_classify_address_map(spec: &str) -> Option<std::collections::HashMap<u64, BlockKind>> {
    if !spec.contains('=') {
        return None;
    }
    let mut map = std::collections::HashMap::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (addr_s, kind_s) = part.split_once('=')?;
        let addr = parse_u64_addr(addr_s)?;
        let kind = parse_block_kind_name(kind_s.trim())?;
        map.insert(addr & !0xfff, kind);
    }
    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

fn parse_classify_kind_labels(spec: &str) -> Option<std::collections::HashMap<u64, String>> {
    if !spec.contains('=') {
        return None;
    }
    let mut map = std::collections::HashMap::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (addr_s, kind_s) = part.split_once('=')?;
        let addr = parse_u64_addr(addr_s)?;
        let label = kind_s.trim().to_lowercase();
        if parse_block_kind_name(&label).is_none() {
            return None;
        }
        map.insert(addr & !0xfff, label);
    }
    if map.is_empty() {
        None
    } else {
        Some(map)
    }
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

fn handle_synth(
    input: &Path,
    component_db: &Path,
    max_bom_cost: Option<f64>,
    preferred_manufacturer: Option<&str>,
    output: &Path,
) -> Result<()> {
    tracing::info!("Loading HardwareSpec from {}", input.display());
    let yaml = fs::read_to_string(input)?;
    let spec = HardwareSpec::from_yaml(&yaml)?;

    let mut db = ComponentDb::new();
    if component_db.exists() {
        db.load_directory(component_db)?;
        tracing::info!("Loaded {} components", db.len());
    }

    if let Some(budget) = max_bom_cost {
        tracing::info!("BOM budget: ${:.2}", budget);
    }
    if let Some(mfg) = preferred_manufacturer {
        tracing::info!("Preferred manufacturer: {}", mfg);
    }

    let mapper = ComponentMapper::new(db);
    let mut synthesized =
        mapper.map_spec_with_prefs(&spec, max_bom_cost, preferred_manufacturer);

    let netlist_segments = base_core::mapping::netlist::generate_netlist(
        &synthesized,
        &ComponentDb::new(),
    );
    synthesized.netlist = Some(netlist_segments);

    fs::create_dir_all(output)?;
    let path = output.join("synthesized_spec.yaml");
    fs::write(&path, serde_yaml::to_string(&synthesized)?)?;
    tracing::info!(
        "SynthesizedSpec written to {} ({} assignments)",
        path.display(),
        synthesized.assignments.len()
    );
    Ok(())
}

// ─── PCB ────────────────────────────────────────────────

fn handle_pcb(input: &Path, project: &str, drc: bool, output: &Path) -> Result<()> {
    let yaml = fs::read_to_string(input)?;
    let spec: SynthesizedSpec = serde_yaml::from_str(&yaml)?;

    fs::create_dir_all(output)?;

    // Schematic — load DB so pin annotations (UART/USART/SPI) appear when present.
    let mut db = ComponentDb::new();
    let db_path = Path::new("base-core/component_db");
    if db_path.exists() {
        db.load_directory(db_path)?;
        tracing::info!("Loaded {} components for PCB draft", db.len());
    }
    let sch_gen = SchematicGenerator::new(Some(db));
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

    let main_c = drv_gen.generate_main(&spec);
    fs::write(output.join("main.c"), &main_c)?;

    let mk = drv_gen.generate_build_system(&spec);
    fs::write(output.join("Makefile"), &mk)?;

    let ld = drv_gen.generate_linker_script(&spec);
    fs::write(output.join("linker.ld"), &ld)?;

    tracing::info!("Firmware generated in {} (host: make -C {} host)", output.display(), output.display());

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
    strict: bool,
    output: &Path,
) -> Result<()> {
    let yaml = fs::read_to_string(input)?;
    let spec: SynthesizedSpec = serde_yaml::from_str(&yaml)?;

    let original = TraceParser::parse(original_trace)?;
    tracing::info!("Parsed original trace: {} events", original.events.len());

    fs::create_dir_all(output)?;
    let gen = ReportGenerator;

    // Optional Ψ from sibling analyze output
    let tension_json = load_optional_tension(input);

    let Some(new_path) = new_trace else {
        let msg = "NO_NEW_TRACE: dual comparison skipped (refusing self-pass). Provide new_trace or use --strict to fail.";
        tracing::warn!("{}", msg);
        if strict {
            anyhow::bail!("{}", msg);
        }
        let mut ctx = ReportContext::skipped(
            &original_trace.display().to_string(),
            max_latency,
            msg,
        );
        ctx.tension = tension_json;
        write_validation_report(&gen, &[], &spec.original.source, format, &ctx, output)?;
        tracing::info!("Validation skipped — report written (comparison_mode=skipped)");
        return Ok(());
    };

    let actual = TraceParser::parse(new_path)?;
    tracing::info!("Parsed new trace: {} events", actual.events.len());

    let thresholds = ValidationThresholds {
        max_latency_ratio: max_latency,
        ..ValidationThresholds::default()
    };

    let items = OperationComparator::compare(&original, &actual, &spec.original, &thresholds);

    let mut ctx = ReportContext::dual(
        &original_trace.display().to_string(),
        &new_path.display().to_string(),
        max_latency,
    );
    ctx.tension = tension_json;

    write_validation_report(&gen, &items, &spec.original.source, format, &ctx, output)?;

    let passed = items.iter().filter(|i| i.passed).count();
    let total = items.len();
    let rate = passed as f64 / total.max(1) as f64;
    tracing::info!(
        "Validation: {}/{} passed ({:.1}%)",
        passed,
        total,
        rate * 100.0
    );

    if strict && rate < 1.0 {
        anyhow::bail!(
            "strict validation failed: {}/{} operations passed",
            passed,
            total
        );
    }

    Ok(())
}

fn write_validation_report(
    gen: &ReportGenerator,
    items: &[base_check::compare::ComparisonItem],
    title: &str,
    format: &str,
    ctx: &ReportContext,
    output: &Path,
) -> Result<()> {
    match format {
        "json" => {
            let json = gen.generate_json_with_context(items, title, ctx);
            fs::write(output.join("validation_report.json"), &json)?;
        }
        "both" => {
            let json = gen.generate_json_with_context(items, title, ctx);
            fs::write(output.join("validation_report.json"), &json)?;
            let html = gen.generate_html_with_context(items, title, ctx);
            fs::write(output.join("validation_report.html"), &html)?;
        }
        _ => {
            let html = gen.generate_html_with_context(items, title, ctx);
            fs::write(output.join("validation_report.html"), &html)?;
            // JSON always for CI auditability
            let json = gen.generate_json_with_context(items, title, ctx);
            fs::write(output.join("validation_report.json"), &json)?;
        }
    }
    Ok(())
}

fn load_optional_tension(synth_input: &Path) -> Option<serde_json::Value> {
    // Prefer ../analyze/tension_report.json or sibling tension_report.json
    let candidates = [
        synth_input
            .parent()
            .map(|p| p.join("tension_report.json")),
        synth_input
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("analyze/tension_report.json")),
        synth_input
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("01_analyze/tension_report.json")),
    ];
    for opt in candidates.into_iter().flatten() {
        if opt.exists() {
            if let Ok(text) = fs::read_to_string(&opt) {
                if let Ok(v) = serde_json::from_str(&text) {
                    return Some(v);
                }
            }
        }
    }
    None
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
    new_trace: Option<&Path>,
    strict: bool,
    target: &str,
    pcb: bool,
    drc: bool,
    zephyr: bool,
    evolve: bool,
    disasm: bool,
    output: &Path,
) -> Result<()> {
    tracing::info!("=== B.A.S.E. Pipeline ===");
    fs::create_dir_all(output)?;

    // Step 1: Analyze
    tracing::info!("[1] Analyzing firmware...");
    handle_analyze(firmware, None, None, true, disasm, &output.join("01_analyze"))?;

    // Step 2: Synth
    tracing::info!("[2] Synthesizing hardware mapping...");
    let component_db_path = Path::new("base-core/component_db");
    handle_synth(
        &output.join("01_analyze/hardware_spec.yaml"),
        component_db_path,
        None,
        None,
        &output.join("02_synth"),
    )?;

    // Step 3: Design (reference YAML — saída principal)
    tracing::info!("[3] Reference design...");
    handle_design(
        &output.join("01_analyze/hardware_spec.yaml"),
        false,
        None,
        None,
        &output.join("03_design"),
    )?;

    // Step 4: FW draft (host-testable)
    tracing::info!("[4] Generating firmware draft...");
    handle_fw(
        &output.join("02_synth/synthesized_spec.yaml"),
        target,
        zephyr,
        &output.join("04_fw"),
    )?;

    // Step 5: Check (never self-pass)
    tracing::info!("[5] Validating...");
    if let Some(trace_path) = trace {
        handle_check(
            &output.join("02_synth/synthesized_spec.yaml"),
            trace_path,
            new_trace,
            2.0,
            "both",
            strict,
            &output.join("05_validation"),
        )?;
    } else {
        tracing::warn!("Skipping validation (no --trace provided)");
        if strict {
            anyhow::bail!("strict pipeline: --trace required for check");
        }
    }

    // Step 6: PCB — opt-in only (engineering draft)
    if pcb {
        tracing::info!("[6] Generating PCB engineering draft (--pcb)...");
        handle_pcb(
            &output.join("02_synth/synthesized_spec.yaml"),
            "project",
            drc,
            &output.join("06_pcb"),
        )?;
    } else {
        tracing::info!("[6] Skipping PCB (pass --pcb for engineering_draft)");
    }

    // Step 7: Evolve — opt-in scaffold
    if evolve {
        tracing::info!("[7] Analyzing evolution (--evolve)...");
        handle_evolve(
            &output.join("02_synth/synthesized_spec.yaml"),
            component_db_path,
            "md",
            &output.join("07_evolution"),
        )?;
    } else {
        tracing::info!("[7] Skipping evolution (pass --evolve to enable scaffold)");
    }

    write_pipeline_summary(output, pcb, evolve, trace.is_some())?;
    tracing::info!("=== Pipeline complete → {} ===", output.display());
    Ok(())
}

fn write_pipeline_summary(output: &Path, pcb: bool, evolve: bool, checked: bool) -> Result<()> {
    let mut md = String::from("# B.A.S.E. Pipeline SUMMARY\n\n");
    md.push_str("- Stages:\n");
    md.push_str("  - `01_analyze/` — HardwareSpec + Evidence + tension_report.json\n");
    md.push_str("  - `02_synth/` — SynthesizedSpec + netlist nominal\n");
    md.push_str("  - `03_design/` — Reference Design (engineering draft)\n");
    md.push_str("  - `04_fw/` — host-testable C (`make host`); **host smoke ≠ silício**\n");
    if checked {
        md.push_str("  - `05_validation/` — dual check or skipped (no self-pass)\n");
    } else {
        md.push_str("  - `05_validation/` — skipped (no `--trace`)\n");
    }
    if pcb {
        md.push_str(
            "  - `06_pcb/` — KiCad **engineering_draft — NOT FABRICABLE**\n",
        );
    } else {
        md.push_str("  - `06_pcb/` — not generated (use `--pcb`)\n");
    }
    if evolve {
        md.push_str("  - `07_evolution/` — evolve scaffold (opt-in)\n");
    } else {
        md.push_str("  - `07_evolution/` — not generated (use `--evolve`)\n");
    }
    md.push_str("\nClaims proibidos neste draft: PCB fabricável, ASIC drop-in, host = target.\n");
    fs::write(output.join("SUMMARY.md"), md)?;
    Ok(())
}

fn handle_replay(trace_path: &Path, contracts_path: Option<PathBuf>, bir_path: Option<PathBuf>, output_path: Option<PathBuf>, output_dir: &Path) -> Result<()> {
    let csv = fs::read_to_string(trace_path)?;
    let events = base_core::replay::parse_saleae_csv(&csv);
    tracing::info!("Parsed {} events from trace", events.len());
    if events.is_empty() { anyhow::bail!("No events found in trace"); }

    let contracts: Vec<base_core::temporal::SequenceContract> = if let Some(cp) = &contracts_path {
        serde_yaml::from_str(&fs::read_to_string(cp)?)?
    } else if let Some(bp) = &bir_path {
        tracing::info!("Extracting contracts from BIR {}", bp.display());
        let bir_yaml = fs::read_to_string(bp)?;
        let device = base_bir::types::BirDevice::from_yaml(&bir_yaml)
            .map_err(|e| anyhow::anyhow!("Invalid BIR: {}", e))?;
        let temporal = base_bir::bir_to_sequence_contracts(&device);
        if temporal.is_empty() {
            anyhow::bail!("BIR {} has no extractable contracts/events", bp.display());
        }
        // Serialize via YAML bridge to SequenceContract
        let yaml = serde_yaml::to_string(&temporal)?;
        serde_yaml::from_str(&yaml)?
    } else {
        anyhow::bail!("Need --contracts or --bir");
    };

    tracing::info!("Using {} contracts", contracts.len());
    let engine = base_core::replay::ReplayEngine::new(contracts);
    let result = engine.replay(&events);
    tracing::info!("Replay: {} sequences, {} passed, {} violations",
        result.summary.total_sequences_found, result.summary.passed, result.summary.failed);

    let out_file = output_path.unwrap_or_else(|| output_dir.join("violations.json"));
    if let Some(parent) = out_file.parent() { fs::create_dir_all(parent)?; }
    fs::write(&out_file, base_core::replay::violations_to_json(&result.violations))?;
    tracing::info!("Violations written to {}", out_file.display());
    Ok(())
}

fn handle_prove(contracts_path: &Path, smt_output: Option<PathBuf>, deadlock: bool, output_dir: &Path) -> Result<()> {
    let yaml = fs::read_to_string(contracts_path)?;
    let contracts: Vec<base_core::temporal::SequenceContract> = serde_yaml::from_str(&yaml)?;
    fs::create_dir_all(output_dir)?;

    if deadlock {
        let result = base_core::smt::SmtProver::deadlock_free(&contracts);
        let out = smt_output.unwrap_or_else(|| output_dir.join("deadlock_proof.smt"));
        fs::write(&out, &result.smt_lib)?;
        let report_path = output_dir.join("deadlock_result.json");
        fs::write(&report_path, serde_json::to_string_pretty(&result)?)?;
        tracing::info!(
            "Deadlock proof: proved={} satisfiable={} → {}",
            result.proved,
            result.satisfiable,
            report_path.display()
        );
        if let Some(m) = &result.model {
            tracing::info!("  model: {}", m);
        }
    } else {
        let report = base_core::smt::SmtProver::prove_all(&contracts);
        let out = smt_output.unwrap_or_else(|| output_dir.join("proof_report.json"));
        fs::write(&out, serde_json::to_string_pretty(&report)?)?;
        // Also dump concatenated SMT for inspection
        let mut smt_all = String::new();
        for r in &report.results {
            smt_all.push_str(&r.smt_lib);
            smt_all.push_str("\n\n");
        }
        fs::write(output_dir.join("contracts_proof.smt"), &smt_all)?;
        tracing::info!(
            "Proved {}/{} contracts (backend={:?}) → {}",
            report.contracts_proved,
            contracts.len(),
            report.backend,
            out.display()
        );
        for r in &report.results {
            tracing::info!(
                "  {}: proved={} sat={} backend={:?} {:?}",
                r.contract,
                r.proved,
                r.satisfiable,
                r.backend,
                r.model
            );
        }
    }
    Ok(())
}

fn handle_design(
    input: &Path,
    pcb: bool,
    max_bom_cost: Option<f64>,
    preferred_manufacturer: Option<&str>,
    output: &Path,
) -> Result<()> {
    let yaml = fs::read_to_string(input)?;
    let spec = base_core::spec::types::HardwareSpec::from_yaml(&yaml)?;

    let mut db = ComponentDb::new();
    let db_path = Path::new("base-core/component_db");
    if db_path.exists() {
        db.load_directory(db_path)?;
        tracing::info!("Loaded {} components for design", db.len());
    }

    if let Some(mfg) = preferred_manufacturer {
        tracing::info!("Preferred manufacturer: {}", mfg);
    }

    let design = base_core::design::ReferenceDesign::from_hardware_spec_prefs(
        &spec,
        &db,
        max_bom_cost,
        preferred_manufacturer,
    );
    fs::create_dir_all(output)?;
    fs::write(output.join("reference_design.yaml"), design.to_yaml()?)?;
    tracing::info!(
        "Reference design: cpu={}, parts={}, contracts {}/{} satisfied",
        design.architecture.cpu.part,
        design.bom.total_parts,
        design.contracts.satisfied,
        design.contracts.total
    );

    if pcb {
        tracing::info!("Generating engineering-draft PCB from design mapping...");
        let mapper = ComponentMapper::new(db);
        let synthesized =
            mapper.map_spec_with_prefs(&spec, max_bom_cost, preferred_manufacturer);
        let synth_path = output.join("synthesized_spec.yaml");
        fs::write(&synth_path, serde_yaml::to_string(&synthesized)?)?;
        handle_pcb(&synth_path, "reference", false, &output.join("pcb"))?;
    }

    Ok(())
}

fn handle_event_graph(contracts_path: &Path, trace_path: &Path, format: &str, output: &Path) -> Result<()> {
    let contracts_yaml = fs::read_to_string(contracts_path)?;
    let contracts: Vec<base_core::temporal::SequenceContract> = serde_yaml::from_str(&contracts_yaml)?;
    let csv = fs::read_to_string(trace_path)?;
    let events = base_core::replay::parse_saleae_csv(&csv);
    let title = trace_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("trace");
    let graph = base_core::event_graph::EventGraph::from_trace(&contracts, &events, title);
    fs::create_dir_all(output)?;
    match format { "mermaid" => fs::write(output.join("event_graph.mmd"), graph.to_mermaid())?, _ => fs::write(output.join("event_graph.dot"), graph.to_dot())?, }
    tracing::info!("Event graph written");
    Ok(())
}

fn handle_bir(input: &Path, compile: bool, validate: bool, to_legacy: bool, dot: bool, output: &Path) -> Result<()> {
    let content = fs::read_to_string(input)?;
    fs::create_dir_all(output)?;

    let device = if compile {
        tracing::info!("Compiling BSL → BIR");
        let device = base_bsl::compile(&content)
            .map_err(|e| anyhow::anyhow!("BSL compile error: {:?}", e))?;
        let bir_path = output.join("compiled.bir.yaml");
        fs::write(&bir_path, device.to_yaml().map_err(|e| anyhow::anyhow!("{}", e))?)?;
        tracing::info!("BIR written to {}", bir_path.display());
        device
    } else {
        base_bir::types::BirDevice::from_yaml(&content)
            .map_err(|e| anyhow::anyhow!("Invalid BIR YAML: {}", e))?
    };

    if validate || (!compile && !to_legacy && !dot) {
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

    // Always export extractable temporal contracts for replay/prove
    let temporal = base_bir::bir_to_sequence_contracts(&device);
    if !temporal.is_empty() {
        let cpath = output.join("contracts.yaml");
        fs::write(&cpath, serde_yaml::to_string(&temporal)?)?;
        tracing::info!("Extracted {} temporal contracts → {}", temporal.len(), cpath.display());
    }

    Ok(())
}

fn handle_reconstruct(input: &Path, threshold: f64, max_iterations: usize, continuous: bool, verbose: bool, output: &Path) -> Result<()> {
    let yaml = fs::read_to_string(input)?;
    let spec = base_core::spec::types::HardwareSpec::from_yaml(&yaml)?;

    fs::create_dir_all(output)?;
    let effective_max = if continuous {
        max_iterations.max(base_core::loop_::CONTINUOUS_ITERATION_CAP)
    } else {
        max_iterations
    };
    tracing::info!("=== B.A.S.E. Reconstruction Loop ===");
    tracing::info!(
        "Threshold: {:.0}%, Max iterations: {}, Continuous: {} (cap={}, not infinite auto-fix)",
        threshold * 100.0,
        effective_max,
        continuous,
        base_core::loop_::CONTINUOUS_ITERATION_CAP
    );
    if continuous {
        tracing::warn!(
            "[reconstruct] --continuous raises the iteration cap only; loop still stops on \
             convergence or structural stagnation. Not full auto-fix."
        );
    }

    let mut loop_ = base_core::loop_::FeedbackLoop::new(threshold, effective_max);
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
    let stop = match report.stop_reason {
        base_core::loop_::StopReason::Converged => "converged",
        base_core::loop_::StopReason::Stagnated => "stagnated",
        base_core::loop_::StopReason::MaxIterations => "max_iterations",
    };
    let report_json = serde_json::json!({
        "total_iterations": report.total_iterations,
        "initial_pass_rate": report.initial_pass_rate,
        "final_pass_rate": report.final_pass_rate,
        "improvement": report.improvement,
        "total_errors_found": report.total_errors_found,
        "avg_errors_per_iteration": report.avg_errors_per_iteration,
        "converged": report.converged,
        "stagnated": report.stagnated,
        "stop_reason": stop,
        "threshold": threshold,
        "continuous": continuous,
        "max_iterations_effective": effective_max,
        "auto_fix_complete": false,
    });
    fs::write(output.join("convergence_report.json"), serde_json::to_string_pretty(&report_json)?)?;

    if let Some(last) = iterations.last() {
        fs::write(output.join("hardware_spec_refined.yaml"), last.spec.to_yaml()?)?;
    }

    match report.stop_reason {
        base_core::loop_::StopReason::Converged => {
            tracing::info!("Converged in {} iterations (stop_reason=converged)", report.total_iterations);
        }
        base_core::loop_::StopReason::Stagnated => {
            tracing::info!(
                "Stagnated after {} iteration(s) — no structural improvements left \
                 (pass {:.1}% < threshold {:.1}%); not auto-fix complete",
                report.total_iterations,
                report.final_pass_rate * 100.0,
                threshold * 100.0
            );
        }
        base_core::loop_::StopReason::MaxIterations => {
            tracing::info!(
                "Hit max iterations ({}) — {:.1}% < {:.1}% (stop_reason=max_iterations)",
                report.total_iterations,
                report.final_pass_rate * 100.0,
                threshold * 100.0
            );
        }
    }

    Ok(())
}

// ─── HIL (EXPERIMENTAL) ─────────────────────────────────

fn parse_usb_id(s: &str) -> Result<u16> {
    let t = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(t, 16).map_err(|e| anyhow::anyhow!("invalid USB id '{s}': {e}"))
}

fn handle_hil(action: &HilCommand, output: &Path) -> Result<()> {
    tracing::info!("[HIL] host REAL* — production gated; not in pipeline default");
    match action {
        HilCommand::Enumerate { vid, pid } => {
            let vid_n = parse_usb_id(vid)?;
            let pid_n = parse_usb_id(pid)?;
            let presence = base_hil::HilAgent::enumerate_presence(vid_n, pid_n);
            tracing::info!(
                "[HIL] enumerate {:04x}:{:04x} → {:?}",
                vid_n,
                pid_n,
                presence
            );
            fs::create_dir_all(output)?;
            let report = serde_json::json!({
                "host_real": true,
                "production": false,
                "vid": format!("0x{vid_n:04x}"),
                "pid": format!("0x{pid_n:04x}"),
                "presence": format!("{presence:?}"),
                "hil_usb_feature": cfg!(feature = "hil_usb"),
                "hil_programmer_feature": cfg!(feature = "hil_programmer"),
                "programmer_feature_lib": base_hil::programmer_feature_enabled(),
            });
            let path = output.join("hil_enumerate.json");
            fs::write(&path, serde_json::to_string_pretty(&report)?)?;
            tracing::info!("Enumerate report → {}", path.display());
        }
        HilCommand::Flash {
            image,
            vid,
            pid,
            mock_detected,
            mock_flash,
            live,
            auto_probe,
        } => {
            let vid_n = parse_usb_id(vid)?;
            let pid_n = parse_usb_id(pid)?;
            if *live && (*mock_detected || *mock_flash) {
                anyhow::bail!("--live refuses --mock-detected / --mock-flash (USB+CMD only)");
            }
            if *live && !cfg!(feature = "hil_usb") {
                anyhow::bail!(
                    "--live requires build with --features hil_live (or hil_usb,hil_programmer)"
                );
            }
            if *live && !base_hil::programmer_feature_enabled() {
                anyhow::bail!("--live requires --features hil_programmer (use hil_live)");
            }
            let data = fs::read(image)?;
            let auto = *auto_probe || *live;
            let agent = if *live {
                std::env::set_var(base_hil::ENV_REQUIRE_LIVE, "1");
                std::env::set_var(base_hil::ENV_LAB_ASSIST, "1");
                base_hil::HilAgent::connect_opts(vid_n, pid_n, auto, true)
                    .map_err(|e| anyhow::anyhow!("{e}"))?
            } else if *mock_detected || *mock_flash {
                if *mock_flash {
                    tracing::warn!(
                        "[HIL][EXPERIMENTAL] --mock-flash → dry-run only (NO silicon)"
                    );
                    base_hil::HilAgent::with_mock_flash(base_hil::ProbePresence::Detected)
                } else {
                    tracing::warn!(
                        "[HIL][EXPERIMENTAL] --mock-detected → Detected offline (no USB)"
                    );
                    base_hil::HilAgent::with_presence(base_hil::ProbePresence::Detected)
                }
            } else {
                base_hil::HilAgent::connect_opts(vid_n, pid_n, auto, false)
                    .map_err(|e| anyhow::anyhow!("{e}"))?
            };

            match agent.try_flash(&data) {
                Ok(receipt) => {
                    assert_ne!(
                        receipt.mode, "production",
                        "CLI must never claim production flash"
                    );
                    tracing::info!(
                        "[HIL] flash receipt mode={} bytes={} live={}",
                        receipt.mode,
                        receipt.bytes,
                        live
                    );
                    fs::create_dir_all(output)?;
                    let report = serde_json::json!({
                        "experimental": !*live,
                        "lab_assist": *live || receipt.mode == "lab_assist",
                        "production": false,
                        "mode": receipt.mode,
                        "bytes": receipt.bytes,
                        "image": image.display().to_string(),
                        "live": *live,
                    });
                    let path = output.join("hil_flash_receipt.json");
                    fs::write(&path, serde_json::to_string_pretty(&report)?)?;
                    tracing::info!("Flash receipt → {}", path.display());
                }
                Err(e) => {
                    anyhow::bail!("{e}");
                }
            }
        }
        HilCommand::LabStatus {
            vid,
            pid,
            sop,
            mock_detected,
            live,
            auto_probe,
            sow_signed,
        } => {
            let vid_n = parse_usb_id(vid)?;
            let pid_n = parse_usb_id(pid)?;
            if *live && *mock_detected {
                anyhow::bail!("--live refuses --mock-detected (USB only)");
            }
            if *live && !cfg!(feature = "hil_usb") {
                anyhow::bail!(
                    "--live requires build with --features hil_live (or hil_usb,hil_programmer)"
                );
            }
            let report = base_hil::evaluate_lab_gate_opts(
                vid_n,
                pid_n,
                base_hil::LabGateOptions {
                    sow_signed: *sow_signed,
                    sop_path: sop.as_deref(),
                    mock_detected: *mock_detected,
                    live: *live,
                    auto_probe: *auto_probe || *live,
                },
            );
            tracing::info!(
                "[HIL][Gate A] lab_assist_ready={} production={}",
                report.lab_assist_ready,
                report.production
            );
            for c in &report.checks {
                tracing::info!(
                    "[HIL][Gate A] {} {} — {}",
                    c.id,
                    if c.green { "GREEN" } else { "BLOCK" },
                    c.detail
                );
            }
            fs::create_dir_all(output)?;
            let path = output.join("hil_lab_gate.json");
            fs::write(&path, serde_json::to_string_pretty(&report)?)?;
            tracing::info!("Lab gate report → {}", path.display());
            if !report.lab_assist_ready {
                tracing::warn!(
                    "[HIL][Gate A] not lab-ready — see {}",
                    report.sow_path_hint
                );
            }
        }
    }
    Ok(())
}

fn handle_port(action: &PortCommand, output: &Path) -> Result<()> {
    match action {
        PortCommand::Package {
            input,
            evidence,
            tension,
            target_hal,
            hal_stub,
            dtb,
            flash_cfg,
        } => {
            tracing::info!(
                "[PORT] package assist — ≠ OS rewrite; target_hal={}",
                target_hal
            );
            let yaml = fs::read_to_string(input)?;
            let spec = HardwareSpec::from_yaml(&yaml)?;
            let evidence_db = match evidence {
                Some(p) => {
                    let t = fs::read_to_string(p)?;
                    Some(base_core::evidence::EvidenceDb::from_yaml(&t)?)
                }
                None => None,
            };
            let tension_report = match tension {
                Some(p) => {
                    let t = fs::read_to_string(p)?;
                    Some(serde_json::from_str::<base_core::tension::TensionReport>(&t)?)
                }
                None => None,
            };
            let mut opts = base_port::PortPackageOptions::new(target_hal.clone());
            opts.target_arch_note =
                "abstract HAL — bind concrete ISA in SOW; fossils = do-not-invent".into();
            let pkg = base_port::build_port_package(
                &spec,
                evidence_db.as_ref(),
                tension_report.as_ref(),
                opts,
            );
            fs::create_dir_all(output)?;
            fs::write(output.join("port_package.yaml"), pkg.to_yaml()?)?;
            fs::write(output.join("address_driver_map.yaml"), pkg.map_yaml()?)?;
            fs::write(output.join("fossil_inventory.yaml"), pkg.fossils_yaml()?)?;
            fs::write(output.join("PORT_PACKAGE.md"), pkg.to_markdown())?;
            tracing::info!(
                "[PORT] wrap={} rewrite={} fossils={} → {}",
                pkg.rewrite_avoidance.wrap_candidates,
                pkg.rewrite_avoidance.must_rewrite,
                pkg.fossil_inventory.fossils.len(),
                output.display()
            );
            if *hal_stub {
                let synth = SynthesizedSpec {
                    original: spec.clone(),
                    assignments: vec![],
                    netlist: None,
                    constraints: SynthesisConstraints {
                        max_bom_cost: None,
                        preferred_manufacturer: None,
                        preferred_package: None,
                    },
                };
                let gen = HalGenerator;
                let c = gen.generate(&synth, target_hal);
                fs::write(output.join("hal_mmio_stub.c"), c)?;
                tracing::info!("[PORT] wrote hal_mmio_stub.c (HOST_BUILD shadow regs)");
            }
            if let Some(dtb_path) = dtb {
                let cfg = flash_cfg
                    .as_ref()
                    .map(|p| fs::read_to_string(p))
                    .transpose()?;
                let plat = base_port::build_platform_from_path(dtb_path, cfg.as_deref())?;
                fs::write(output.join("platform_inventory.yaml"), plat.to_yaml()?)?;
                fs::write(output.join("PLATFORM_INVENTORY.md"), plat.to_markdown())?;
                tracing::info!(
                    "[PORT] platform readiness={:.0}% missing={:?}",
                    plat.os_port_readiness.score * 100.0,
                    plat.os_port_readiness.missing
                );
            }
            assert!(!pkg.generates_os);
            assert!(!pkg.auto_fix_complete);
        }
        PortCommand::Platform { input, flash_cfg } => {
            tracing::info!("[PORT] platform inventory from {}", input.display());
            let cfg = flash_cfg
                .as_ref()
                .map(|p| fs::read_to_string(p))
                .transpose()?;
            let plat = base_port::build_platform_from_path(input, cfg.as_deref())?;
            fs::create_dir_all(output)?;
            fs::write(output.join("platform_inventory.yaml"), plat.to_yaml()?)?;
            fs::write(output.join("PLATFORM_INVENTORY.md"), plat.to_markdown())?;
            tracing::info!(
                "[PORT] CPU={} readiness={:.0}% missing={:?}",
                plat.cpu.isa_hint,
                plat.os_port_readiness.score * 100.0,
                plat.os_port_readiness.missing
            );
            assert!(!plat.generates_os);
        }
        PortCommand::UsbProbe {
            serial,
            skip_adb,
            skip_fastboot,
            skip_lsusb,
        } => {
            tracing::info!(
                "[PORT] USB HW probe — read-only · ≠ flash · ≠ OS turnkey"
            );
            let opts = base_port::UsbProbeOptions {
                serial: serial.clone(),
                skip_adb: *skip_adb,
                skip_fastboot: *skip_fastboot,
                skip_lsusb: *skip_lsusb,
            };
            let inv = base_port::run_usb_hw_probe(&opts);
            fs::create_dir_all(output)?;
            fs::write(output.join("usb_hw_inventory.yaml"), inv.to_yaml()?)?;
            fs::write(
                output.join("usb_hw_inventory.json"),
                inv.to_json_pretty()?,
            )?;
            fs::write(output.join("USB_HW_PROBE.md"), inv.to_markdown())?;
            assert!(!inv.generates_os);
            assert!(!inv.auto_fix_complete);
            if inv.skipped {
                tracing::warn!(
                    "[PORT] USB probe skipped: {}",
                    inv.skip_reason.as_deref().unwrap_or("unknown")
                );
                println!(
                    "usb-probe SKIP → {} ({})",
                    output.display(),
                    inv.skip_reason.as_deref().unwrap_or("no device")
                );
            } else {
                tracing::info!(
                    "[PORT] USB probe mode={:?} ok={} props={} sysfs={} dt_compat={} platform={}",
                    inv.mode,
                    inv.ok,
                    inv.props.len(),
                    inv.sysfs_classes.len(),
                    inv.dt_compatibles.len(),
                    inv.platform_devices.len()
                );
                println!(
                    "usb-probe OK → {} (mode={:?} props={} platform={} dt_compat={})",
                    output.display(),
                    inv.mode,
                    inv.props.len(),
                    inv.platform_devices.len(),
                    inv.dt_compatibles.len()
                );
            }
        }
        PortCommand::UsbCross { usb, platform } => {
            tracing::info!(
                "[PORT] USB×DTB cross — {} ↔ {}",
                usb.display(),
                platform.display()
            );
            let usb_yaml = fs::read_to_string(usb)?;
            let plat_yaml = fs::read_to_string(platform)?;
            let report = base_port::cross_usb_dt_files(&usb_yaml, &plat_yaml)?;
            fs::create_dir_all(output)?;
            fs::write(output.join("usb_dt_cross.yaml"), report.to_yaml()?)?;
            fs::write(
                output.join("usb_dt_cross.json"),
                report.to_json_pretty()?,
            )?;
            fs::write(
                output.join("wedge_mmio_map.yaml"),
                report.wedge_map.to_yaml()?,
            )?;
            fs::write(output.join("BRINGUP_CHECKLIST.md"), report.to_markdown())?;
            assert!(!report.generates_os);
            println!(
                "usb-cross OK → {} (matches={} p0_ready={} target={})",
                output.display(),
                report.matches.len(),
                report.wedge_map.p0_ready,
                report.port_target
            );
            if report.wedge_map.p0_ready {
                for e in &report.wedge_map.entries {
                    if e.priority == "P0" {
                        if let Some(h) = &e.absolute_base_hex {
                            println!("  P0 {}: {} ({:?})", e.class, h, e.source);
                        }
                    }
                }
            } else {
                println!("  p0_missing: {:?}", report.wedge_map.p0_missing);
            }
        }
        PortCommand::WedgeP0 { map } => {
            tracing::info!("[PORT] wedge P0 board stub from {}", map.display());
            let yaml = fs::read_to_string(map)?;
            let wedge: base_port::WedgeMmioMap = serde_yaml::from_str(&yaml)?;
            let pkg = base_port::build_wedge_p0_package(&wedge);
            fs::create_dir_all(output)?;
            fs::write(output.join("wedge_p0_package.yaml"), pkg.to_yaml()?)?;
            fs::write(output.join("board-ums9620-wedge-p0.dtsi"), &pkg.dtsi)?;
            fs::write(output.join("cmdline_earlycon.txt"), {
                let mut t = String::new();
                for h in &pkg.earlycon_hints {
                    t.push_str(h);
                    t.push('\n');
                }
                t
            })?;
            fs::write(output.join("hal_wedge_p0.h"), &pkg.hal_h)?;
            fs::write(output.join("hal_wedge_p0.c"), &pkg.hal_c)?;
            fs::write(output.join("WEDGE_P0.md"), pkg.to_markdown())?;
            assert!(!pkg.generates_os);
            println!(
                "wedge-p0 OK → {} (p0_ready={} uart={:?} gicd={:?} gicr={:?} ufs={:?})",
                output.display(),
                pkg.p0_ready,
                pkg.uart_base.map(|a| format!("{a:#x}")),
                pkg.gic_base.map(|a| format!("{a:#x}")),
                pkg.gicr_base.map(|a| format!("{a:#x}")),
                pkg.ufs_base.map(|a| format!("{a:#x}")),
            );
        }
        PortCommand::ClocksPinctrl { usb, dtb } => {
            tracing::info!(
                "[PORT] clocks/pinctrl hints — {} × {}",
                usb.display(),
                dtb.display()
            );
            let raw = fs::read_to_string(usb)?;
            let usb_inv: base_port::UsbHwInventory = serde_yaml::from_str(&raw)
                .or_else(|_| serde_json::from_str(&raw))?;
            let data = fs::read(dtb)?;
            let blobs = base_port::extract_fdt_blobs(&data);
            let primary = blobs
                .iter()
                .max_by_key(|b| b.len())
                .map(|b| b.as_slice())
                .unwrap_or(data.as_slice());
            let hints = base_port::build_clocks_pinctrl_from_bytes(&usb_inv, primary)?;
            fs::create_dir_all(output)?;
            fs::write(output.join("clocks_pinctrl_hints.yaml"), hints.to_yaml()?)?;
            fs::write(
                output.join("clocks_pinctrl_hints.json"),
                hints.to_json_pretty()?,
            )?;
            fs::write(
                output.join("board-ums9620-wedge-clocks-pinctrl.dtsi"),
                &hints.dtsi_snippet,
            )?;
            fs::write(output.join("CLOCKS_PINCTRL.md"), hints.to_markdown())?;
            assert!(!hints.generates_os);
            println!(
                "clocks-pinctrl OK → {} (clk={} pinctrl={} uart_bindings={})",
                output.display(),
                hints.clock_controllers.len(),
                hints.pinctrl.len(),
                hints.uart_bindings.len()
            );
        }
    }
    Ok(())
}

fn load_fossil_sequence(path: &Path) -> Result<base_core::FossilSequence> {
    let text = fs::read_to_string(path)?;
    // Prefer EvidenceDb shape; fall back to FossilSequence YAML
    if let Ok(db) = base_core::evidence::EvidenceDb::from_yaml(&text) {
        return Ok(base_core::FossilSequence::from_evidence(&db));
    }
    Ok(serde_yaml::from_str(&text)?)
}

fn handle_paleo(action: &PaleoCommand, output: &Path) -> Result<()> {
    match action {
        PaleoCommand::Align { a, b } => {
            tracing::info!("[PALEO] StratAlign {} ↔ {}", a.display(), b.display());
            let seq_a = load_fossil_sequence(a)?;
            let seq_b = load_fossil_sequence(b)?;
            let result = base_core::StratAligner::default().align(&seq_a, &seq_b);
            fs::create_dir_all(output)?;
            fs::write(output.join("strat_align.yaml"), result.to_yaml()?)?;
            fs::write(output.join("STRAT_ALIGN.md"), result.to_markdown())?;
            tracing::info!(
                "[PALEO] similarity={:.1}% T0={:.3} matches={}",
                result.normalized_similarity * 100.0,
                result.raw_tension,
                result.match_count
            );
            assert!(!result.generates_os);
            assert!(!result.auto_fix_complete);
        }
        PaleoCommand::Excavate {
            input,
            evidence,
            reference,
            functions,
            instructions,
            calls,
        } => {
            tracing::info!("[PALEO] excavate Ω → Ψ → atlas");
            let spec = HardwareSpec::from_yaml(&fs::read_to_string(input)?)?;
            let ev = base_core::evidence::EvidenceDb::from_yaml(&fs::read_to_string(evidence)?)?;
            let ref_db = match reference {
                Some(p) => Some(base_core::evidence::EvidenceDb::from_yaml(
                    &fs::read_to_string(p)?,
                )?),
                None => None,
            };
            let result = base_core::excavate(
                &ev,
                &spec,
                ref_db.as_ref(),
                *functions,
                *instructions,
                *calls,
            );
            fs::create_dir_all(output)?;
            fs::write(output.join("paleo_excavate.yaml"), result.to_yaml()?)?;
            fs::write(output.join("PALEO_ATLAS.md"), result.to_markdown())?;
            if let Some(sa) = &result.strat_align {
                fs::write(output.join("strat_align.yaml"), sa.to_yaml()?)?;
                fs::write(output.join("STRAT_ALIGN.md"), sa.to_markdown())?;
            }
            tracing::info!(
                "[PALEO] Ψ={:.4} confidence={:.1}% {:?}",
                result.tension.overall_tension,
                result.tension.overall_confidence * 100.0,
                result.tension.conclusiveness
            );
            assert!(!result.generates_os);
            assert!(!result.auto_fix_complete);
        }
        PaleoCommand::Phylo {
            evidence,
            spec,
            delta_t,
        } => {
            tracing::info!("[PALEO] phylogeny N={} taxa", evidence.len());
            if evidence.len() < 2 {
                anyhow::bail!("phylo needs ≥2 evidence YAML files");
            }
            let mut dbs = Vec::new();
            for p in evidence {
                let db = base_core::evidence::EvidenceDb::from_yaml(&fs::read_to_string(p)?)?;
                dbs.push(db);
            }
            // Labels únicos: pasta pai + stem (evita colisão evidence_db.yaml × N)
            for (db, p) in dbs.iter_mut().zip(evidence.iter()) {
                let stem = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("taxon");
                let parent = p
                    .parent()
                    .and_then(|d| d.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("src");
                db.source = if stem == "evidence_db" || stem == "evidence" {
                    parent.to_string()
                } else {
                    format!("{parent}_{stem}")
                };
            }
            let specs_owned: Vec<HardwareSpec> = spec
                .iter()
                .map(|p| {
                    let t = fs::read_to_string(p)?;
                    HardwareSpec::from_yaml(&t).map_err(|e| anyhow::anyhow!("{e}"))
                })
                .collect::<Result<Vec<_>>>()?;
            let spec_refs: Vec<Option<&HardwareSpec>> =
                (0..dbs.len()).map(|i| specs_owned.get(i)).collect();
            let dts: Vec<f64> = if delta_t.is_empty() {
                (0..dbs.len()).map(|i| 1.0 + i as f64).collect()
            } else {
                let mut v = delta_t.clone();
                while v.len() < dbs.len() {
                    v.push(v.last().copied().unwrap_or(1.0) + 1.0);
                }
                v
            };
            let db_refs: Vec<&base_core::evidence::EvidenceDb> = dbs.iter().collect();
            let result = base_core::phylogeny_from_evidence(
                &db_refs,
                &spec_refs,
                &dts,
                &base_core::PhyloParams::default(),
            );
            fs::create_dir_all(output)?;
            fs::write(output.join("phylo.yaml"), result.to_yaml()?)?;
            fs::write(output.join("PHYLO_ATLAS.md"), result.to_markdown())?;
            fs::write(output.join("tree.nwk"), &result.newick)?;
            fs::write(output.join("cladogram.mmd"), result.to_mermaid())?;
            tracing::info!(
                "[PALEO] Newick={} THC={} homoplasy={}",
                result.newick,
                result.thc_events.len(),
                result.homoplasy_events.len()
            );
            assert!(!result.generates_os);
            assert!(!result.auto_fix_complete);
        }
    }
    Ok(())
}

fn handle_study(
    input: &Path,
    policy_path: Option<&Path>,
    program_path: Option<&Path>,
    evidence_path: Option<&Path>,
    output: &Path,
) -> Result<()> {
    tracing::info!("=== B.A.S.E. Specter study (Forth + Lua) — ≠ auto-fix ===");
    let yaml = fs::read_to_string(input)?;
    let spec = base_core::spec::types::HardwareSpec::from_yaml(&yaml)?;
    let policy = base_vm::load_policy(policy_path)?;
    let program_src = match program_path {
        Some(p) => Some(fs::read_to_string(p)?),
        None => None,
    };
    let evidence = match evidence_path {
        Some(p) => Some(base_virt::load_evidence_flexible(p)?),
        None => None,
    };
    let live = evidence.is_some();
    let (refined, report) =
        base_vm::run_study_with_evidence(&spec, evidence, &policy, program_src.as_deref())?;

    fs::create_dir_all(output)?;
    fs::write(
        output.join("hardware_spec_refined.yaml"),
        refined.to_yaml()?,
    )?;
    let report_json = serde_json::to_string_pretty(&report)?;
    fs::write(output.join("study_report.json"), &report_json)?;
    tracing::info!(
        "Study done: steps={} live={} stop_reason={:?} auto_fix_complete={}",
        report.total_steps,
        live,
        report.stop_reason,
        report.auto_fix_complete
    );
    Ok(())
}

fn handle_virt(action: &VirtCommand, output: &Path) -> Result<()> {
    fs::create_dir_all(output)?;
    tracing::info!("=== B.A.S.E. Specter Live (virt) — ≠ OS turnkey ===");
    tracing::info!("{}", base_core::HONESTY_BANNER);

    match action {
        VirtCommand::Ingest { trace, format } => {
            let fmt: base_virt::TraceFormat = format
                .parse()
                .map_err(|e: String| anyhow::anyhow!(e))?;
            let db = base_virt::ingest_path_with_format(trace, fmt)?;
            fs::write(output.join("evidence_db.yaml"), db.to_yaml()?)?;
            let summary = serde_json::json!({
                "phase": "virt_ingest",
                "format": fmt.as_str(),
                "entries": db.count(),
                "unique_mmio": db.unique_mmio_addresses().len(),
                "generates_os": false,
                "auto_fix_complete": false,
                "honesty": base_core::HONESTY_NOTE,
            });
            fs::write(
                output.join("ingest_summary.json"),
                serde_json::to_string_pretty(&summary)?,
            )?;
            tracing::info!("Ingested {} evidence entries (format={})", db.count(), fmt.as_str());
        }
        VirtCommand::Score {
            spec,
            evidence,
            window_size,
            max_windows,
        } => {
            let spec = HardwareSpec::from_yaml(&fs::read_to_string(spec)?)?;
            let db = base_core::evidence::EvidenceDb::from_yaml(&fs::read_to_string(evidence)?)?;
            let tension = base_core::tension::TensionMetric::compute(&db, &spec, 0, 0, 0);
            fs::write(
                output.join("tension_report.json"),
                base_core::tension::TensionMetric::to_json(&tension)?,
            )?;
            let cfg = base_virt::LiveConfig {
                window_size: (*window_size).max(1),
                max_windows: *max_windows,
                ..Default::default()
            };
            let session = base_virt::run_live_windows(&db, &spec, &cfg);
            fs::write(output.join("virt_session.yaml"), session.to_yaml()?)?;
            fs::write(output.join("virt_session.json"), session.to_json_pretty()?)?;
            tracing::info!(
                "Score: confidence={:.3} conclusiveness={:?} windows={}",
                session.final_confidence,
                session.final_conclusiveness,
                session.windows.len()
            );
        }
        VirtCommand::Run {
            spec,
            trace,
            kernel,
            qemu,
            timeout_sec,
            window_size,
            max_windows,
            no_qemu,
            plugin,
            plugin_outfile,
            qmp,
            probe_qmp,
            plugin_arg,
        } => {
            let spec = HardwareSpec::from_yaml(&fs::read_to_string(spec)?)?;
            let mut qemu_exit = None;
            let mut qemu_bin = None;
            let mut kernel_s = None;
            let plugin_out = plugin_outfile
                .clone()
                .unwrap_or_else(|| output.join("plugin_trace.ndjson"));
            let qmp_sock = if *qmp || *probe_qmp {
                Some(output.join("qmp.sock"))
            } else {
                None
            };
            let plugin_args = if plugin_arg.is_empty() {
                vec!["io_only=1".into()]
            } else {
                plugin_arg.clone()
            };

            if !no_qemu {
                if let Some(k) = kernel {
                    let opts = base_virt::QemuLaunchOpts {
                        bin: qemu.clone(),
                        kernel: Some(k.clone()),
                        timeout_sec: *timeout_sec,
                        log_path: output.join("qemu.log"),
                        plugin: plugin.clone(),
                        plugin_outfile: if plugin.is_some() {
                            Some(plugin_out.clone())
                        } else {
                            None
                        },
                        plugin_args: plugin_args.clone(),
                        qmp_socket: qmp_sock.clone(),
                        ..Default::default()
                    };

                    // Optional early QMP probe while guest still running: spawn + probe + wait.
                    let launch = if *probe_qmp && qmp_sock.is_some() {
                        match base_virt::spawn_qemu_live(&opts)? {
                            Ok(mut session) => {
                                let sock = session.qmp_socket.clone().unwrap();
                                match base_virt::probe_session(&sock) {
                                    Ok(probe) => {
                                        fs::write(
                                            output.join("qmp_probe.json"),
                                            serde_json::to_string_pretty(&probe)?,
                                        )?;
                                        tracing::info!("QMP probe OK");
                                    }
                                    Err(e) => {
                                        tracing::warn!("QMP probe failed: {e}");
                                        fs::write(
                                            output.join("qmp_probe.json"),
                                            serde_json::to_string_pretty(&serde_json::json!({
                                                "ok": false,
                                                "error": e.to_string(),
                                                "generates_os": false,
                                            }))?,
                                        )?;
                                    }
                                }
                                // Drain remaining time then quit/kill.
                                let timeout = std::time::Duration::from_secs(*timeout_sec);
                                let start = std::time::Instant::now();
                                let mut timed_out = false;
                                let exit_code = loop {
                                    match session.child.try_wait()? {
                                        Some(st) => break st.code(),
                                        None => {
                                            if start.elapsed() >= timeout {
                                                if let Ok(mut q) =
                                                    base_virt::QmpClient::connect_unix(&sock)
                                                {
                                                    let _ = q.quit();
                                                    let _ = session.child.wait();
                                                } else {
                                                    let _ = session.child.kill();
                                                    let _ = session.child.wait();
                                                }
                                                timed_out = true;
                                                break Some(124);
                                            }
                                            std::thread::sleep(std::time::Duration::from_millis(50));
                                        }
                                    }
                                };
                                base_virt::QemuLaunchResult {
                                    launched: true,
                                    skipped: false,
                                    skip_reason: None,
                                    exit_code,
                                    timed_out,
                                    bin: session.bin,
                                    kernel: Some(session.kernel),
                                    log_path: Some(session.log_path.display().to_string()),
                                    timeout_sec: *timeout_sec,
                                    qmp_socket: Some(sock.display().to_string()),
                                    plugin: plugin.as_ref().map(|p| p.display().to_string()),
                                    plugin_outfile: session
                                        .plugin_outfile
                                        .map(|p| p.display().to_string()),
                                }
                            }
                            Err(skipped) => skipped,
                        }
                    } else {
                        base_virt::launch_qemu(&opts)?
                    };

                    qemu_bin = Some(launch.bin.clone());
                    kernel_s = launch.kernel.clone();
                    qemu_exit = launch.exit_code;
                    fs::write(
                        output.join("qemu_launch.json"),
                        serde_json::to_string_pretty(&launch)?,
                    )?;
                    if launch.skipped {
                        tracing::warn!(
                            "QEMU skipped: {}",
                            launch.skip_reason.as_deref().unwrap_or("unknown")
                        );
                    }
                } else {
                    tracing::warn!("--kernel omitted; QEMU not launched (use --no-qemu to silence)");
                }
            }

            // Prefer explicit --trace; else plugin outfile if present.
            let db = if let Some(t) = trace {
                base_virt::ingest_ndjson_path(t, "specter_live")?
            } else if plugin.is_some() && plugin_out.exists() {
                tracing::info!("Ingesting plugin outfile {}", plugin_out.display());
                base_virt::ingest_ndjson_path(&plugin_out, "specter_live_plugin")?
            } else {
                tracing::warn!("No --trace / plugin outfile — empty evidence (smoke-only)");
                base_core::evidence::EvidenceDb::new("specter_live_empty")
            };
            fs::write(output.join("evidence_live.yaml"), db.to_yaml()?)?;

            let cfg = base_virt::LiveConfig {
                window_size: (*window_size).max(1),
                max_windows: *max_windows,
                ..Default::default()
            };
            let mut session = base_virt::run_live_windows(&db, &spec, &cfg);
            session.qemu_exit = qemu_exit;
            session.qemu_bin = qemu_bin;
            session.kernel = kernel_s;
            if db.count() == 0 && session.skip_reason.is_none() {
                session.ok = qemu_exit.is_some();
                session.note =
                    "QEMU smoke without NDJSON — set --trace or --plugin for Ψ live".into();
            }
            fs::write(output.join("virt_session.yaml"), session.to_yaml()?)?;
            fs::write(output.join("virt_session.json"), session.to_json_pretty()?)?;

            let md = format!(
                "# Specter Live session\n\n{}\n\n- evidence: {}\n- windows: {}\n- final_confidence: {:.3}\n- conclusiveness: {:?}\n- qemu_exit: {:?}\n- qmp: {}\n- plugin: {}\n- generates_os: false\n- auto_fix_complete: false\n",
                base_core::HONESTY_BANNER,
                session.total_evidence,
                session.windows.len(),
                session.final_confidence,
                session.final_conclusiveness,
                session.qemu_exit,
                qmp_sock.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "off".into()),
                plugin.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "off".into()),
            );
            fs::write(output.join("CASE_SUMMARY_VIRT.md"), md)?;
            tracing::info!(
                "Virt run: evidence={} conf={:.3} qemu_exit={:?}",
                session.total_evidence,
                session.final_confidence,
                session.qemu_exit
            );
        }
        VirtCommand::Qmp {
            socket,
            cmd,
            raw,
            tag,
        } => {
            if !socket.exists() {
                anyhow::bail!(
                    "QMP socket ausente: {} (nenhum QEMU a ouvir). \
                     `virt demo qmp` sobe, sonda e faz quit — para `qmp status` mantém o guest noutro terminal, p.ex.:\n\
                     qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 64M -nographic \
                     -kernel examples/pilot_moto_g35/kernel.bin \
                     -qmp unix:{},server,nowait -serial none -monitor none",
                    socket.display(),
                    socket.display()
                );
            }
            match cmd.as_str() {
                "probe" => {
                    let probe = base_virt::probe_session(socket)?;
                    fs::write(output.join("qmp_probe.json"), serde_json::to_string_pretty(&probe)?)?;
                    println!("{}", serde_json::to_string_pretty(&probe)?);
                }
                "probe-savevm" => {
                    let probe = base_virt::probe_savevm(socket, tag)?;
                    fs::write(
                        output.join("qmp_savevm_probe.json"),
                        serde_json::to_string_pretty(&probe)?,
                    )?;
                    println!("{}", serde_json::to_string_pretty(&probe)?);
                }
                "raw" => {
                    let body = raw.as_deref().ok_or_else(|| {
                        anyhow::anyhow!("--raw JSON required for qmp raw")
                    })?;
                    let v: serde_json::Value = serde_json::from_str(body)?;
                    let exec = v
                        .get("execute")
                        .and_then(|x| x.as_str())
                        .ok_or_else(|| anyhow::anyhow!("raw JSON needs execute"))?;
                    let args = v.get("arguments").cloned();
                    let mut c = base_virt::QmpClient::connect_unix_wait(
                        socket,
                        std::time::Duration::from_secs(10),
                    )?;
                    let resp = c.execute(exec, args)?;
                    fs::write(output.join("qmp_raw.json"), serde_json::to_string_pretty(&resp)?)?;
                    println!("{}", serde_json::to_string_pretty(&resp)?);
                }
                other => {
                    let mut c = base_virt::QmpClient::connect_unix_wait(
                        socket,
                        std::time::Duration::from_secs(10),
                    )?;
                    let resp = match other {
                        "stop" => c.stop()?,
                        "cont" | "continue" => c.cont()?,
                        "status" => c.query_status()?,
                        "inject-nmi" | "nmi" => c.inject_nmi()?,
                        "reset" => c.system_reset()?,
                        "quit" => c.quit()?,
                        "savevm" => c.savevm(tag)?,
                        "loadvm" => c.loadvm(tag)?,
                        _ => anyhow::bail!(
                            "unknown qmp cmd '{other}' (stop|cont|status|inject-nmi|reset|quit|probe|probe-savevm|savevm|loadvm|raw)"
                        ),
                    };
                    fs::write(output.join("qmp_response.json"), serde_json::to_string_pretty(&resp)?)?;
                    println!("{}", serde_json::to_string_pretty(&resp)?);
                }
            }
        }
        VirtCommand::Study {
            spec,
            evidence,
            policy,
            program,
            qmp_socket,
        } => {
            let spec = HardwareSpec::from_yaml(&fs::read_to_string(spec)?)?;
            let ev = base_virt::load_evidence_flexible(evidence)?;
            let pol = base_vm::load_policy(policy.as_deref())?;
            let program_src = match program {
                Some(p) => Some(fs::read_to_string(p)?),
                None => None,
            };
            let (refined, report) = base_virt::run_live_study(
                &spec,
                ev,
                &pol,
                program_src.as_deref(),
                qmp_socket.as_deref(),
            )?;
            fs::write(
                output.join("hardware_spec_refined.yaml"),
                refined.to_yaml()?,
            )?;
            fs::write(
                output.join("live_study_report.json"),
                serde_json::to_string_pretty(&report)?,
            )?;
            let md = format!(
                "# Specter Live Study (E4)\n\n{}\n\n- live: {}\n- evidence: {}\n- steps: {}\n- final_pass_rate: {:.3}\n- psi: {:.3}\n- stop: {:?}\n- qmp_gated: {}\n- generates_os: false\n",
                base_core::HONESTY_BANNER,
                report.study.live,
                report.study.evidence_count,
                report.study.total_steps,
                report.study.final_pass_rate,
                report.study.final_psi_confidence,
                report.study.stop_reason,
                report.qmp_gated,
            );
            fs::write(output.join("CASE_SUMMARY_LIVE_STUDY.md"), md)?;
            tracing::info!(
                "Live study: steps={} conf={:.3} qmp_gated={}",
                report.study.total_steps,
                report.study.final_pass_rate,
                report.qmp_gated
            );
        }
        VirtCommand::Twin { spec, evidence } => {
            let spec = HardwareSpec::from_yaml(&fs::read_to_string(spec)?)?;
            let ev = base_virt::load_evidence_flexible(evidence)?;
            let report = base_virt::compare_twin_guest(&spec, &ev);
            fs::write(output.join("twin_guest.yaml"), report.to_yaml()?)?;
            fs::write(output.join("twin_guest.json"), report.to_json_pretty()?)?;
            let md = format!(
                "# Twin↔guest (v1.6)\n\n{}\n\n- hit_rate: {:.3}\n- hits: {}\n- misses: {}\n- psi: {:.3}\n- twin_only: {:?}\n- generates_os: false\n",
                base_core::HONESTY_BANNER,
                report.hit_rate,
                report.hits,
                report.misses,
                report.psi_confidence,
                report.twin_only_blocks,
            );
            fs::write(output.join("CASE_SUMMARY_TWIN_GUEST.md"), md)?;
            tracing::info!(
                "Twin↔guest: hit_rate={:.3} hits={} misses={} psi={:.3}",
                report.hit_rate,
                report.hits,
                report.misses,
                report.psi_confidence
            );
        }
        VirtCommand::BirTwin {
            spec,
            evidence,
            block,
        } => {
            let spec = HardwareSpec::from_yaml(&fs::read_to_string(spec)?)?;
            let ev = base_virt::load_evidence_flexible(evidence)?;
            let (bir, report) =
                base_virt::replay_bir_twin(&spec, &ev, block.as_deref())?;
            fs::write(output.join("bir_device.yaml"), bir.to_yaml()?)?;
            fs::write(
                output.join("bir_twin_report.json"),
                serde_json::to_string_pretty(&report)?,
            )?;
            let md = format!(
                "# BIR Twin replay (v1.6 F1)\n\n{}\n\n- device: {}\n- base: 0x{:x}\n- writes_applied: {}\n- writes_skipped: {}\n- twin_steps: {}\n- registers: {:?}\n- generates_os: false\n",
                base_core::HONESTY_BANNER,
                report.device_name,
                report.base_address,
                report.writes_applied,
                report.writes_skipped,
                report.twin_steps,
                report.register_snapshot,
            );
            fs::write(output.join("CASE_SUMMARY_BIR_TWIN.md"), md)?;
            tracing::info!(
                "BIR twin: applied={} skipped={} regs={:?}",
                report.writes_applied,
                report.writes_skipped,
                report.register_snapshot
            );
        }
        VirtCommand::Watch {
            spec,
            trace,
            window_events,
            max_ticks,
            poll_ms,
            poll_timeout_sec,
        } => {
            let spec = HardwareSpec::from_yaml(&fs::read_to_string(spec)?)?;
            let cfg = base_virt::ContinuousDiffConfig {
                window_events: *window_events,
                max_ticks: *max_ticks,
                poll_ms: *poll_ms,
                poll_timeout_sec: *poll_timeout_sec,
            };
            let report = base_virt::run_continuous_diff_file(&spec, trace, &cfg)?;
            fs::write(output.join("continuous_diff.json"), report.to_json_pretty()?)?;
            fs::write(output.join("continuous_diff.yaml"), report.to_yaml()?)?;
            let md = format!(
                "# Continuous twin diff (v1.6 F3)\n\n{}\n\n- ticks: {}\n- evidence: {}\n- final_hit_rate: {:.3}\n- final_psi: {:.3}\n- live_polled: {}\n- generates_os: false\n",
                base_core::HONESTY_BANNER,
                report.ticks.len(),
                report.total_evidence,
                report.final_hit_rate,
                report.final_psi,
                report.live_polled,
            );
            fs::write(output.join("CASE_SUMMARY_CONTINUOUS.md"), md)?;
            tracing::info!(
                "Continuous diff: ticks={} hit_rate={:.3} psi={:.3}",
                report.ticks.len(),
                report.final_hit_rate,
                report.final_psi
            );
        }
        VirtCommand::Demo { target } => {
            handle_virt_demo(target, output)?;
        }
    }
    Ok(())
}

/// Resolve `examples/pilot_moto_g35/` walking up from CWD (and common relatives).
fn resolve_pilot_g35_root() -> Result<PathBuf> {
    let marker = Path::new("examples/pilot_moto_g35/virt/hardware_spec_mame_stub.yaml");
    let cwd = std::env::current_dir()?;
    let mut dir = cwd.clone();
    for _ in 0..8 {
        let cand = dir.join(marker);
        if cand.is_file() {
            return Ok(dir.join("examples/pilot_moto_g35"));
        }
        if !dir.pop() {
            break;
        }
    }
    // Fallback: relative to this crate when run from target/
    let from_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    if let Some(mut d) = from_exe {
        for _ in 0..6 {
            let cand = d.join(marker);
            if cand.is_file() {
                return Ok(d.join("examples/pilot_moto_g35"));
            }
            if !d.pop() {
                break;
            }
        }
    }
    anyhow::bail!(
        "piloto G35 não encontrado (procure {} a partir de {}) — corre a partir da raiz do repo",
        marker.display(),
        cwd.display()
    )
}

fn handle_virt_demo(target: &str, cli_output: &Path) -> Result<()> {
    let t = target.trim().to_ascii_lowercase();
    let allowed = ["watch", "twin", "qmp", "all"];
    if !allowed.contains(&t.as_str()) {
        anyhow::bail!("demo target inválido '{target}' (use: watch|twin|qmp|all)");
    }

    let pilot = resolve_pilot_g35_root()?;
    let virt = pilot.join("virt");
    let spec = virt.join("hardware_spec_mame_stub.yaml");
    let ndjson = virt.join("sample_mmio.ndjson");
    let mame = virt.join("sample_mame.trace");
    let kernel = pilot.join("kernel.bin");

    for (label, p) in [
        ("spec", &spec),
        ("ndjson", &ndjson),
        ("mame", &mame),
        ("kernel", &kernel),
    ] {
        if !p.is_file() {
            anyhow::bail!("demo: ficheiro em falta ({label}): {}", p.display());
        }
    }

    // Prefer explicit -o; default CLI "output" → /tmp/base_virt_demo
    let root = if cli_output.as_os_str() == "output" {
        PathBuf::from("/tmp/base_virt_demo")
    } else {
        cli_output.to_path_buf()
    };
    fs::create_dir_all(&root)?;

    let run_watch = t == "watch" || t == "all";
    let run_twin = t == "twin" || t == "all";
    let run_qmp = t == "qmp" || t == "all";

    if run_watch {
        let out = root.join("watch");
        fs::create_dir_all(&out)?;
        let hw = HardwareSpec::from_yaml(&fs::read_to_string(&spec)?)?;
        let cfg = base_virt::ContinuousDiffConfig {
            window_events: 2,
            max_ticks: 32,
            poll_ms: 0,
            poll_timeout_sec: 8,
        };
        let report = base_virt::run_continuous_diff_file(&hw, &ndjson, &cfg)?;
        fs::write(out.join("continuous_diff.json"), report.to_json_pretty()?)?;
        fs::write(out.join("continuous_diff.yaml"), report.to_yaml()?)?;
        tracing::info!(
            "demo watch → {} (ticks={} hit_rate={:.3})",
            out.display(),
            report.ticks.len(),
            report.final_hit_rate
        );
        println!(
            "demo watch OK → {} (ticks={} hit_rate={:.3} psi={:.3})",
            out.display(),
            report.ticks.len(),
            report.final_hit_rate,
            report.final_psi
        );
    }

    if run_twin {
        let out = root.join("twin");
        fs::create_dir_all(&out)?;
        let hw = HardwareSpec::from_yaml(&fs::read_to_string(&spec)?)?;
        let db = base_virt::ingest_path_with_format(&mame, base_virt::TraceFormat::Mame)?;
        fs::write(out.join("evidence_db.yaml"), db.to_yaml()?)?;
        let twin = base_virt::compare_twin_guest(&hw, &db);
        fs::write(out.join("twin_guest.json"), twin.to_json_pretty()?)?;
        let (bir, bir_report) = base_virt::replay_bir_twin(&hw, &db, None)?;
        fs::write(out.join("bir_device.yaml"), bir.to_yaml()?)?;
        fs::write(
            out.join("bir_twin_report.json"),
            serde_json::to_string_pretty(&bir_report)?,
        )?;
        tracing::info!(
            "demo twin → {} (hit_rate={:.3} bir_writes={})",
            out.display(),
            twin.hit_rate,
            bir_report.writes_applied
        );
        println!(
            "demo twin OK → {} (hit_rate={:.3} psi={:.3} bir_writes={})",
            out.display(),
            twin.hit_rate,
            twin.psi_confidence,
            bir_report.writes_applied
        );
    }

    if run_qmp {
        let out = root.join("qmp");
        fs::create_dir_all(&out)?;
        let sock = PathBuf::from("/tmp/base-qmp.sock");
        let opts = base_virt::QemuLaunchOpts {
            bin: "qemu-system-aarch64".into(),
            kernel: Some(kernel.clone()),
            timeout_sec: 8,
            log_path: out.join("qemu.log"),
            qmp_socket: Some(sock.clone()),
            memory: "64M".into(),
            extra_args: vec!["-serial".into(), "none".into(), "-monitor".into(), "none".into()],
            plugin: None,
            plugin_outfile: None,
            plugin_args: Vec::new(),
            ..Default::default()
        };

        match base_virt::spawn_qemu_live(&opts)? {
            Err(skipped) => {
                fs::write(out.join("qmp_demo.json"), serde_json::to_string_pretty(&skipped)?)?;
                tracing::warn!(
                    "demo qmp skipped: {}",
                    skipped.skip_reason.as_deref().unwrap_or("unknown")
                );
                println!(
                    "demo qmp SKIP → {} ({})",
                    out.display(),
                    skipped.skip_reason.as_deref().unwrap_or("qemu missing")
                );
            }
            Ok(mut session) => {
                let probe = base_virt::probe_savevm(&sock, "base_snap");
                let status = base_virt::QmpClient::connect_unix_wait(
                    &sock,
                    std::time::Duration::from_secs(5),
                )
                .and_then(|mut c| c.query_status());

                let status_v = status
                    .as_ref()
                    .map(|v| v.clone())
                    .unwrap_or_else(|e| serde_json::json!({"error": e.to_string()}));
                let probe_v = probe
                    .as_ref()
                    .map(|v| v.clone())
                    .unwrap_or_else(|e| serde_json::json!({"error": e.to_string()}));

                let summary = serde_json::json!({
                    "phase": "virt_demo_qmp",
                    "socket": sock.display().to_string(),
                    "status": status_v,
                    "probe_savevm": probe_v,
                    "note": "savevm may fail without a block device for vmstate — expected",
                    "generates_os": false,
                    "honesty": base_core::HONESTY_NOTE,
                });
                fs::write(
                    out.join("qmp_demo.json"),
                    serde_json::to_string_pretty(&summary)?,
                )?;

                if let Ok(mut q) = base_virt::QmpClient::connect_unix(&sock) {
                    let _ = q.quit();
                    let _ = session.child.wait();
                } else {
                    let _ = session.child.kill();
                    let _ = session.child.wait();
                }

                let ok = status.is_ok();
                tracing::info!("demo qmp → {} (status_ok={ok})", out.display());
                println!(
                    "demo qmp OK → {} (status_ok={ok}; savevm machine-dependent)",
                    out.display()
                );
            }
        }
    }

    let md = format!(
        "# Virt demo ({t})\n\n{}\n\n- pilot: {}\n- out: {}\n- generates_os: false\n",
        base_core::HONESTY_BANNER,
        pilot.display(),
        root.display(),
    );
    fs::write(root.join("CASE_SUMMARY_DEMO.md"), md)?;
    Ok(())
}

#[cfg(test)]
mod classify_tests {
    use super::*;

    #[test]
    fn classify_map_parses_uart_spi_pages() {
        let map = parse_classify_address_map("0x40034000=uart,0x4003c000=spi").unwrap();
        assert_eq!(map.get(&0x40034000), Some(&BlockKind::Uart));
        assert_eq!(map.get(&0x4003c000), Some(&BlockKind::Spi));
    }

    #[test]
    fn classify_map_parses_timer_alias() {
        let map = parse_classify_address_map("0x40000000=timer,0x40013000=uart").unwrap();
        assert_eq!(map.get(&0x40000000), Some(&BlockKind::Timer));
        assert_eq!(map.get(&0x40013000), Some(&BlockKind::Uart));
        let map_tim = parse_classify_address_map("0x40000000=tim").unwrap();
        assert_eq!(map_tim.get(&0x40000000), Some(&BlockKind::Timer));
    }

    #[test]
    fn classify_global_uart_still_all_blocks() {
        let mut spec = HardwareSpec::empty();
        spec.blocks.push(FunctionalBlock {
            id: "a".into(),
            kind: BlockKind::Unknown,
            base_address: 0x40034000,
            size: 0x1000,
            registers: vec![],
            protocol: Protocol {
                states: vec![],
                transitions: vec![],
                entry_condition: None,
                exit_condition: None,
            },
            timing: TimingProfile {
                activation: None,
                processing: None,
                interrupt_response: None,
                dma_setup: None,
                polling_interval: None,
            },
            dma: None,
            dependencies: vec![],
            confidence: 0.5,
        });
        spec.blocks.push(FunctionalBlock {
            id: "b".into(),
            kind: BlockKind::Unknown,
            base_address: 0x4003c000,
            size: 0x1000,
            registers: vec![],
            protocol: Protocol {
                states: vec![],
                transitions: vec![],
                entry_condition: None,
                exit_condition: None,
            },
            timing: TimingProfile {
                activation: None,
                processing: None,
                interrupt_response: None,
                dma_setup: None,
                polling_interval: None,
            },
            dma: None,
            dependencies: vec![],
            confidence: 0.5,
        });
        apply_classify_override(&mut spec, "uart");
        assert!(spec.blocks.iter().all(|b| b.kind == BlockKind::Uart));
    }

    #[test]
    fn classify_map_assigns_per_page() {
        let mut spec = HardwareSpec::empty();
        for (id, addr) in [("u", 0x40034000u64), ("s", 0x4003c000u64)] {
            spec.blocks.push(FunctionalBlock {
                id: id.into(),
                kind: BlockKind::Unknown,
                base_address: addr,
                size: 0x1000,
                registers: vec![],
                protocol: Protocol {
                    states: vec![],
                    transitions: vec![],
                    entry_condition: None,
                    exit_condition: None,
                },
                timing: TimingProfile {
                    activation: None,
                    processing: None,
                    interrupt_response: None,
                    dma_setup: None,
                    polling_interval: None,
                },
                dma: None,
                dependencies: vec![],
                confidence: 0.5,
            });
        }
        apply_classify_override(&mut spec, "0x40034000=uart,0x4003c000=spi");
        assert_eq!(spec.blocks[0].kind, BlockKind::Uart);
        assert_eq!(spec.blocks[1].kind, BlockKind::Spi);
    }
}

fn default_g35_wedge_path() -> PathBuf {
    PathBuf::from("examples/pilot_moto_g35/out_real/handoff_external/atlas/wedge_mmio_map.yaml")
}

fn handle_reason(action: &ReasonCommand, output: &Path) -> Result<()> {
    use base_reason::{questions_from_wedge_yaml, ReasonSignals, ReasoningSession};
    use base_port::{AddrSource, WedgeMmioMap};

    match action {
        ReasonCommand::Report {
            wedge,
            twin_miss,
            evidence_id,
            incoherent,
            receipt_draft,
            format,
        } => {
            let wedge_path = wedge.clone().unwrap_or_else(default_g35_wedge_path);
            let yaml = fs::read_to_string(&wedge_path).map_err(|e| {
                anyhow::anyhow!("read wedge {}: {e}", wedge_path.display())
            })?;

            let mut sig = ReasonSignals::new();
            // Prefer typed parse when possible
            if let Ok(map) = serde_yaml::from_str::<WedgeMmioMap>(&yaml) {
                sig.p0_missing = map.p0_missing.clone();
                sig.unresolved_classes = map
                    .entries
                    .iter()
                    .filter(|e| e.source == AddrSource::Unresolved)
                    .map(|e| e.class.clone())
                    .collect();
                if !map.p0_ready {
                    sig.hypothesis_scores
                        .push(("atlas_incomplete".into(), 60));
                    sig.hypothesis_scores.push(("need_usb_or_dt".into(), 40));
                } else {
                    sig.hypothesis_scores.push(("atlas_p0_ready".into(), 80));
                    sig.hypothesis_scores
                        .push(("still_needs_lab_boot".into(), 20));
                }
            } else {
                // Fallback: question adapter only
                let qs = questions_from_wedge_yaml(&yaml)?;
                for q in qs {
                    match q.kind {
                        base_reason::QuestionKind::MissingP0 => {
                            sig.p0_missing.push(q.subject);
                        }
                        base_reason::QuestionKind::UnresolvedAddr => {
                            sig.unresolved_classes.push(q.subject);
                        }
                        _ => {}
                    }
                }
            }

            sig.twin_misses = twin_miss.clone();
            sig.evidence_ids = evidence_id.clone();
            sig.coherent = !*incoherent;
            sig.causal_ok = true; // assist claims do not require CausalEdge by default

            let mut session = ReasoningSession::new();
            let report = session.ingest_signals(&sig);

            fs::create_dir_all(output)?;
            let stem = output.join("reason_report");
            if format == "json" {
                let path = stem.with_extension("json");
                fs::write(&path, report.to_json_pretty()?)?;
                tracing::info!("wrote {}", path.display());
            } else {
                let path = stem.with_extension("md");
                fs::write(&path, report.to_markdown())?;
                tracing::info!("wrote {}", path.display());
                print!("{}", report.to_markdown());
            }

            if *receipt_draft {
                let draft = serde_json::json!({
                    "kind": "hw_boot_receipt_draft",
                    "result": "pending_lab",
                    "flashed": false,
                    "mode": "lab_assist",
                    "questions_open": report.questions.len(),
                    "triad_verdict": format!("{:?}", report.triad.verdict),
                    "generates_os": false,
                    "auto_fix_complete": false,
                    "honesty": report.honesty,
                    "note": "Draft only — ≠ production flash · fill after manual lab",
                });
                let path = output.join("reason_receipt_draft.json");
                fs::write(&path, serde_json::to_string_pretty(&draft)?)?;
                tracing::info!("wrote receipt draft {}", path.display());
            }
        }
        ReasonCommand::G35 { wedge, format } => {
            handle_reason(
                &ReasonCommand::Report {
                    wedge: wedge.clone().or_else(|| Some(default_g35_wedge_path())),
                    twin_miss: vec![],
                    evidence_id: vec![],
                    incoherent: false,
                    receipt_draft: true,
                    format: format.clone(),
                },
                output,
            )?;
        }
    }
    Ok(())
}
