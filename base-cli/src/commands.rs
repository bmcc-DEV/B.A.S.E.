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

use crate::cli::{Command, HilCommand};

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
    tracing::warn!("[HIL][EXPERIMENTAL] base hil — not production; not in pipeline default");
    match action {
        HilCommand::Enumerate { vid, pid } => {
            let vid_n = parse_usb_id(vid)?;
            let pid_n = parse_usb_id(pid)?;
            let presence = base_hil::HilAgent::enumerate_presence(vid_n, pid_n);
            tracing::info!(
                "[HIL][EXPERIMENTAL] enumerate {:04x}:{:04x} → {:?}",
                vid_n,
                pid_n,
                presence
            );
            fs::create_dir_all(output)?;
            let report = serde_json::json!({
                "experimental": true,
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
        } => {
            let vid_n = parse_usb_id(vid)?;
            let pid_n = parse_usb_id(pid)?;
            let data = fs::read(image)?;
            let agent = if *mock_detected || *mock_flash {
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
                base_hil::HilAgent::connect(vid_n, pid_n)
                    .map_err(|e| anyhow::anyhow!("{e}"))?
            };

            match agent.try_flash(&data) {
                Ok(receipt) => {
                    assert_ne!(
                        receipt.mode, "production",
                        "CLI must never claim production flash"
                    );
                    tracing::info!(
                        "[HIL][EXPERIMENTAL] flash receipt mode={} bytes={}",
                        receipt.mode,
                        receipt.bytes
                    );
                    fs::create_dir_all(output)?;
                    let report = serde_json::json!({
                        "experimental": true,
                        "production": false,
                        "mode": receipt.mode,
                        "bytes": receipt.bytes,
                        "image": image.display().to_string(),
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
    }
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
