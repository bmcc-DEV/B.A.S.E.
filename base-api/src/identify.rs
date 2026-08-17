//! Identify: firmware + MMIO → blocks + prove por contrato + atlas de requirements.

use anyhow::{bail, Context, Result};
use base_core::inference::extraction::{MmioAccess, MmioAccessType};
use base_core::inference::generate_spec_with_evidence;
use base_core::smt::SmtProver;
use base_core::solver::{self, ContractRequirement};
use base_core::spec::types::{BlockKind, FunctionalBlock, HardwareSpec};
use base_core::temporal::SequenceContract;
use base_core::tension::TensionMetric;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct IdentifyRequest {
    /// Firmware bytes (base64). Pode ser placeholder se só tiveres MMIO.
    pub firmware_b64: String,
    /// Acessos MMIO (JSON array, mesmo schema do piloto).
    #[serde(default)]
    pub mmio: Vec<MmioAccessDto>,
    /// Contratos temporais YAML (opcional). Se vazio, gera requirements por bloco.
    #[serde(default)]
    pub contracts_yaml: Option<String>,
    /// Label opcional do job
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MmioAccessDto {
    pub address: u64,
    #[serde(default)]
    pub value: Option<u64>,
    pub access_type: String,
    #[serde(default = "default_fn")]
    pub function_name: String,
    #[serde(default)]
    pub instruction_addr: u64,
}

fn default_fn() -> String {
    "anon".into()
}

impl MmioAccessDto {
    fn to_access(&self) -> Result<MmioAccess> {
        let access_type = match self.access_type.to_lowercase().as_str() {
            "read" | "r" => MmioAccessType::Read,
            "write" | "w" => MmioAccessType::Write,
            other => bail!("unknown access_type: {other}"),
        };
        Ok(MmioAccess {
            address: self.address,
            value: self.value,
            access_type,
            function_name: self.function_name.clone(),
            instruction_addr: self.instruction_addr,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct IdentifyResponse {
    pub id: String,
    pub object: &'static str,
    pub label: Option<String>,
    pub hardware: HardwareSummary,
    pub by_contract: Vec<ContractIdentification>,
    pub proof: Option<ProofSummary>,
    pub usage: crate::billing::UsageBreakdown,
    pub honesty: HonestyFlags,
}

#[derive(Debug, Serialize)]
pub struct HonestyFlags {
    pub saas_production: bool,
    pub auto_fix_complete: bool,
    pub generates_os: bool,
    pub note: &'static str,
}

#[derive(Debug, Serialize)]
pub struct HardwareSummary {
    pub blocks: Vec<BlockSummary>,
    pub confidence: f64,
    pub tension_psi: f64,
    pub tension_confidence: f64,
    pub evidence_entries: usize,
    pub firmware_sha256: String,
}

#[derive(Debug, Serialize)]
pub struct BlockSummary {
    pub id: String,
    pub kind: String,
    pub base_address: u64,
    pub base_address_hex: String,
    pub size: u64,
    pub confidence: f64,
    pub register_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ContractIdentification {
    /// Nome do contrato temporal ou requirement
    pub contract: String,
    pub kind: ContractKind,
    /// O que o HW “parece” ser neste contrato
    pub identified_as: String,
    pub related_block_id: Option<String>,
    pub related_base_address: Option<u64>,
    pub requirements: Vec<ContractRequirement>,
    pub proved: Option<bool>,
    pub satisfiable: Option<bool>,
    pub detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractKind {
    TemporalSequence,
    BlockRequirement,
}

#[derive(Debug, Serialize)]
pub struct ProofSummary {
    pub contracts_proved: usize,
    pub contracts_total: usize,
    pub backend: String,
}

pub struct IdentifyOutcome {
    pub response_without_usage: IdentifyResponseDraft,
    pub firmware_bytes: u64,
    pub mmio_events: u64,
    pub contracts: u64,
}

/// Draft antes de preencher usage (billing).
#[derive(Debug, Serialize)]
pub struct IdentifyResponseDraft {
    pub id: String,
    pub object: &'static str,
    pub label: Option<String>,
    pub hardware: HardwareSummary,
    pub by_contract: Vec<ContractIdentification>,
    pub proof: Option<ProofSummary>,
    pub honesty: HonestyFlags,
}

pub fn run_identify(req: IdentifyRequest) -> Result<IdentifyOutcome> {
    use base64::Engine;
    let firmware = base64::engine::general_purpose::STANDARD
        .decode(req.firmware_b64.trim())
        .context("firmware_b64 invalid")?;
    if firmware.is_empty() {
        bail!("firmware empty");
    }
    if firmware.len() > 16 * 1024 * 1024 {
        bail!("firmware too large (max 16 MiB in lab API)");
    }

    let mmio: Vec<MmioAccess> = req
        .mmio
        .iter()
        .map(|m| m.to_access())
        .collect::<Result<Vec<_>>>()?;

    if mmio.is_empty() {
        bail!("mmio required — API identify is evidence-driven (pass MMIO traces)");
    }

    let source = req
        .label
        .clone()
        .unwrap_or_else(|| "api_identify".into());
    let (spec, evidence) = generate_spec_with_evidence(&mmio, &source);

    let fn_count = {
        let mut names = std::collections::HashSet::new();
        for a in &mmio {
            names.insert(a.function_name.clone());
        }
        names.len()
    };
    let tension = TensionMetric::compute(&evidence, &spec, fn_count, firmware.len(), 0);

    let firmware_sha256 = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&firmware);
        format!("{:x}", h.finalize())
    };

    let blocks: Vec<BlockSummary> = spec
        .blocks
        .iter()
        .map(|b| BlockSummary {
            id: b.id.clone(),
            kind: format!("{:?}", b.kind).to_lowercase(),
            base_address: b.base_address,
            base_address_hex: format!("0x{:08X}", b.base_address),
            size: b.size,
            confidence: b.confidence,
            register_count: b.registers.len(),
        })
        .collect();

    let mut by_contract: Vec<ContractIdentification> = Vec::new();
    let mut proof_summary = None;
    let mut contract_units: u64 = 0;

    // 1) Temporal contracts (YAML) — prove each
    if let Some(yaml) = &req.contracts_yaml {
        let contracts: Vec<SequenceContract> =
            serde_yaml::from_str(yaml).context("contracts_yaml parse")?;
        contract_units += contracts.len() as u64;
        let report = SmtProver::prove_all(&contracts);
        proof_summary = Some(ProofSummary {
            contracts_proved: report.contracts_proved,
            contracts_total: contracts.len(),
            backend: format!("{:?}", report.backend),
        });
        for (c, r) in contracts.iter().zip(report.results.iter()) {
            let related = match_block_for_contract(&spec, c);
            by_contract.push(ContractIdentification {
                contract: c.name.clone(),
                kind: ContractKind::TemporalSequence,
                identified_as: identify_label(related, &c.name),
                related_block_id: related.map(|b| b.id.clone()),
                related_base_address: related.map(|b| b.base_address),
                requirements: related
                    .map(|b| solver::extract_contracts(b))
                    .unwrap_or_default(),
                proved: Some(r.proved),
                satisfiable: Some(r.satisfiable),
                detail: r
                    .model
                    .clone()
                    .unwrap_or_else(|| format!("proved={} sat={}", r.proved, r.satisfiable)),
            });
        }
    }

    // 2) Per-block requirement contracts — sempre (o “identificar tudo”)
    for block in &spec.blocks {
        let reqs = solver::extract_contracts(block);
        contract_units += reqs.len() as u64;
        let identified = kind_human(block.kind);
        by_contract.push(ContractIdentification {
            contract: format!("block:{}", block.id),
            kind: ContractKind::BlockRequirement,
            identified_as: identified,
            related_block_id: Some(block.id.clone()),
            related_base_address: Some(block.base_address),
            requirements: reqs,
            proved: None,
            satisfiable: None,
            detail: format!(
                "confidence={:.2} regs={} kind={:?}",
                block.confidence,
                block.registers.len(),
                block.kind
            ),
        });
    }

    let draft = IdentifyResponseDraft {
        id: format!("id_{}", Uuid::new_v4()),
        object: "identify.result",
        label: req.label,
        hardware: HardwareSummary {
            blocks,
            confidence: spec.confidence,
            tension_psi: tension.overall_tension,
            tension_confidence: tension.overall_confidence,
            evidence_entries: evidence.entries.len(),
            firmware_sha256,
        },
        by_contract,
        proof: proof_summary,
        honesty: HonestyFlags {
            saas_production: false,
            auto_fix_complete: false,
            generates_os: false,
            note: "Lab meter API — pay-as-you-go units; Stripe not wired; ≠ magic RE",
        },
    };

    Ok(IdentifyOutcome {
        firmware_bytes: firmware.len() as u64,
        mmio_events: mmio.len() as u64,
        contracts: contract_units.max(1),
        response_without_usage: draft,
    })
}

fn kind_human(kind: BlockKind) -> String {
    match kind {
        BlockKind::Uart => "UART / serial MMIO block".into(),
        BlockKind::Spi => "SPI controller block".into(),
        BlockKind::I2c => "I2C controller block".into(),
        BlockKind::Timer => "Timer / counter block".into(),
        BlockKind::Dma => "DMA engine block".into(),
        BlockKind::Gpu => "GPU / display MMIO block".into(),
        BlockKind::Audio => "Audio / DSP MMIO block".into(),
        BlockKind::Usb => "USB controller block".into(),
        BlockKind::Ethernet => "Ethernet MAC block".into(),
        BlockKind::InterruptController => "IRQ controller block".into(),
        BlockKind::MemoryController => "Memory controller block".into(),
        BlockKind::Crypto => "Crypto accelerator block".into(),
        BlockKind::VideoCodec => "Video codec block".into(),
        BlockKind::Isp => "ISP / camera block".into(),
        BlockKind::Npu => "NPU / AI accel block".into(),
        BlockKind::Unknown => "Unknown MMIO block (needs classify / more evidence)".into(),
    }
}

fn match_block_for_contract<'a>(
    spec: &'a HardwareSpec,
    contract: &SequenceContract,
) -> Option<&'a FunctionalBlock> {
    let mut addrs = Vec::new();
    for step in &contract.steps {
        if let Some(a) = step.address {
            addrs.push(a & !0xfff);
        }
    }
    if addrs.is_empty() {
        return None;
    }
    spec.blocks.iter().find(|b| {
        let page = b.base_address & !0xfff;
        addrs.iter().any(|a| *a == page)
    })
}

fn identify_label(block: Option<&FunctionalBlock>, contract_name: &str) -> String {
    match block {
        Some(b) => format!("{} (via contract `{contract_name}`)", kind_human(b.kind)),
        None => format!("temporal contract `{contract_name}` (no matching MMIO page yet)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identify_saturn_synth() {
        let mmio = vec![
            MmioAccessDto {
                address: 0x25C00000,
                value: Some(1),
                access_type: "write".into(),
                function_name: "vdp1".into(),
                instruction_addr: 0,
            },
            MmioAccessDto {
                address: 0x25C00004,
                value: None,
                access_type: "read".into(),
                function_name: "vdp1".into(),
                instruction_addr: 4,
            },
        ];
        let req = IdentifyRequest {
            firmware_b64: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                b"SATURNHW",
            ),
            mmio,
            contracts_yaml: Some(
                r#"
- name: vdp1_cmd
  steps:
    - event_type: mmio_write
      address: 0x25C00000
      value: 1
      tolerance_ns: 100
  max_total_ns: 2000
  max_step_ns: 1500
  order: Strict
"#
                .into(),
            ),
            label: Some("saturn_test".into()),
        };
        let out = run_identify(req).unwrap();
        assert!(!out.response_without_usage.by_contract.is_empty());
        assert!(out.mmio_events >= 2);
    }
}
