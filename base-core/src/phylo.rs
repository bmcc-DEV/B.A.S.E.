//! Filogenia Computacional — evolução orgânica da Paleocomputação (jul/2026).
//!
//! G(B) = {(f, ω(f), λ(f))} · d_φ = Ψ · exp(−λ̄·Δt) · Neighbor-Joining · THC / homoplasia.
//!
//! Fonte: *PaleoComputação — Evolução Filogenética*.  
//! Honestidade: assist de linhagem — ≠ árvore genealógica prova judicial · ≠ auto-fix.

use crate::evidence::EvidenceDb;
use crate::paleo::ObservablesOmega;
use crate::spec::types::HardwareSpec;
use crate::strat_align::{FossilAtom, FossilPersistence, FossilSequence, StratAligner};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Carga de linhagem λ(f) ∈ (0, 1] — decai quando o fóssil se espalha pelo corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenotypeLocus {
    pub fossil: FossilAtom,
    /// Observáveis locais proxy (região, persistência).
    pub omega_tag: String,
    pub lineage_load: f64,
}

/// Genótipo G(B) — nuvem fóssil com λ (≠ fenótipo Φ).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genotype {
    pub label: String,
    pub loci: Vec<GenotypeLocus>,
    pub phenotype: Option<ObservablesOmega>,
    pub stratum_delta_t: f64,
}

impl Genotype {
    /// Constrói G(B) a partir de evidência; λ via frequência no corpus (se dado).
    pub fn from_evidence(
        db: &EvidenceDb,
        corpus_freq: Option<&HashMap<String, usize>>,
        corpus_size: usize,
        phenotype: Option<ObservablesOmega>,
        delta_t: f64,
    ) -> Self {
        let seq = FossilSequence::from_evidence(db);
        let mut loci = Vec::with_capacity(seq.atoms.len() * 2);
        let mut pages: HashSet<u64> = HashSet::new();
        for atom in seq.atoms {
            if let Some(r) = atom.region {
                pages.insert(r);
            }
            let freq = corpus_freq
                .and_then(|m| m.get(&atom.id).copied())
                .unwrap_or(1)
                .max(1);
            // λ decai com presença em variantes distantes / frequentes no corpus
            let spread = freq as f64 / corpus_size.max(1) as f64;
            let base = 1.0 - atom.persistence.erosion_rate() * 0.5;
            let lambda = (base * (1.0 - 0.7 * spread)).clamp(0.05, 1.0);
            let omega_tag = format!(
                "{:?}:{}",
                atom.persistence,
                atom.region.map(|r| format!("{r:#x}")).unwrap_or_else(|| "-".into())
            );
            loci.push(GenotypeLocus {
                fossil: atom,
                omega_tag,
                lineage_load: lambda,
            });
        }
        // Fósseis de página (ancestrais): permitem linhagem SoC sem match exacto de reg
        for page in pages {
            let id = format!("page:{page:#x}");
            let freq = corpus_freq
                .and_then(|m| m.get(&id).copied())
                .unwrap_or(1)
                .max(1);
            let spread = freq as f64 / corpus_size.max(1) as f64;
            let lambda = (0.85 * (1.0 - 0.5 * spread)).clamp(0.1, 1.0);
            loci.push(GenotypeLocus {
                fossil: FossilAtom::new(id, FossilPersistence::Ancestral).with_region(page),
                omega_tag: format!("page:{page:#x}"),
                lineage_load: lambda,
            });
        }
        Self {
            label: db.source.clone(),
            loci,
            phenotype,
            stratum_delta_t: delta_t,
        }
    }

    pub fn fossil_ids(&self) -> HashSet<String> {
        self.loci.iter().map(|l| l.fossil.id.clone()).collect()
    }

    pub fn lambda_map(&self) -> HashMap<String, f64> {
        self.loci
            .iter()
            .map(|l| (l.fossil.id.clone(), l.lineage_load))
            .collect()
    }
}

/// Constrói mapa de frequência de fósseis num corpus.
pub fn corpus_fossil_frequency(dbs: &[&EvidenceDb]) -> HashMap<String, usize> {
    let mut freq: HashMap<String, usize> = HashMap::new();
    for db in dbs {
        let seq = FossilSequence::from_evidence(db);
        let mut ids: HashSet<String> = seq.atoms.iter().map(|a| a.id.clone()).collect();
        for a in &seq.atoms {
            if let Some(r) = a.region {
                ids.insert(format!("page:{r:#x}"));
            }
        }
        for id in ids {
            *freq.entry(id).or_insert(0) += 1;
        }
    }
    freq
}

/// Distância filogenética d_φ(Bᵢ, Bⱼ) = Ψ · exp(−λ̄ · Δt).
pub fn phylo_distance(gi: &Genotype, gj: &Genotype) -> PhyloPairStats {
    let seq_i = FossilSequence::new(
        gi.label.clone(),
        gi.loci.iter().map(|l| l.fossil.clone()).collect(),
    );
    let seq_j = FossilSequence::new(
        gj.label.clone(),
        gj.loci.iter().map(|l| l.fossil.clone()).collect(),
    );
    let align = StratAligner::default().align(&seq_i, &seq_j);
    let psi = align.raw_tension;

    let ids_i = gi.fossil_ids();
    let ids_j = gj.fossil_ids();
    let shared: Vec<&str> = ids_i.intersection(&ids_j).map(|s| s.as_str()).collect();
    let li = gi.lambda_map();
    let lj = gj.lambda_map();
    let lambda_bar = if shared.is_empty() {
        0.0
    } else {
        shared
            .iter()
            .map(|id| {
                let a = li.get(*id).copied().unwrap_or(0.0);
                let b = lj.get(*id).copied().unwrap_or(0.0);
                (a + b) * 0.5
            })
            .sum::<f64>()
            / shared.len() as f64
    };

    let delta_t = (gi.stratum_delta_t - gj.stratum_delta_t)
        .abs()
        .max(1.0);
    let d_phi = psi * (-lambda_bar * delta_t).exp();

    PhyloPairStats {
        a: gi.label.clone(),
        b: gj.label.clone(),
        psi,
        lambda_bar,
        delta_t,
        d_phi,
        shared_fossils: shared.len(),
        strat_similarity: align.normalized_similarity,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhyloPairStats {
    pub a: String,
    pub b: String,
    pub psi: f64,
    pub lambda_bar: f64,
    pub delta_t: f64,
    pub d_phi: f64,
    pub shared_fossils: usize,
    pub strat_similarity: f64,
}

/// Evento de transferência horizontal de código (THC).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThcEvent {
    pub from: String,
    pub to: String,
    pub d_phi: f64,
    pub local_block_similarity: f64,
    pub block_size: usize,
    pub sample_fossils: Vec<String>,
    pub note: String,
}

/// Homoplasia — similaridade sem ancestralidade nem THC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomoplasyEvent {
    pub a: String,
    pub b: String,
    pub d_phi: f64,
    pub identical_fossils: usize,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhyloTreeNode {
    pub name: String,
    pub children: Vec<PhyloTreeNode>,
    pub branch_length: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhyloResult {
    pub claim: &'static str,
    pub generates_os: bool,
    pub auto_fix_complete: bool,
    pub genotypes: Vec<Genotype>,
    pub distance_matrix: Vec<Vec<f64>>,
    pub labels: Vec<String>,
    pub pairs: Vec<PhyloPairStats>,
    pub tree: PhyloTreeNode,
    pub newick: String,
    pub thc_events: Vec<ThcEvent>,
    pub homoplasy_events: Vec<HomoplasyEvent>,
    pub honesty: &'static str,
}

/// Parâmetros de detecção THC / homoplasia.
#[derive(Debug, Clone)]
pub struct PhyloParams {
    /// d_φ acima disto = ramos distantes.
    pub distant_d_phi: f64,
    /// Similaridade local de bloco para THC.
    pub thc_local_sim: f64,
    pub thc_min_block: usize,
    /// Fração de fósseis idênticos sugerindo homoplasia se distantes.
    pub homoplasy_identical_frac: f64,
}

impl Default for PhyloParams {
    fn default() -> Self {
        Self {
            distant_d_phi: 0.55,
            thc_local_sim: 0.85,
            thc_min_block: 3,
            homoplasy_identical_frac: 0.15,
        }
    }
}

/// Reconstrói filogenia N-a-N a partir de genótipos.
pub fn reconstruct_phylogeny(genotypes: &[Genotype], params: &PhyloParams) -> PhyloResult {
    let n = genotypes.len();
    let labels: Vec<String> = genotypes.iter().map(|g| g.label.clone()).collect();
    let mut matrix = vec![vec![0.0; n]; n];
    let mut pairs = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            let stats = phylo_distance(&genotypes[i], &genotypes[j]);
            matrix[i][j] = stats.d_phi;
            matrix[j][i] = stats.d_phi;
            pairs.push(stats);
        }
    }

    let (tree, newick) = neighbor_joining(&matrix, &labels);
    let thc_events = detect_thc(genotypes, &pairs, params);
    let homoplasy_events = detect_homoplasy(genotypes, &pairs, params);

    PhyloResult {
        claim: "computational_phylogeny_assist",
        generates_os: false,
        auto_fix_complete: false,
        genotypes: genotypes.to_vec(),
        distance_matrix: matrix,
        labels,
        pairs,
        tree,
        newick,
        thc_events,
        homoplasy_events,
        honesty: "Filogenia assist — d_φ/THC heurísticos; ≠ prova de plágio · ≠ auto-fix",
    }
}

/// Atalho: evidence DBs (+ specs opcionais para Φ).
pub fn phylogeny_from_evidence(
    dbs: &[&EvidenceDb],
    specs: &[Option<&HardwareSpec>],
    delta_ts: &[f64],
    params: &PhyloParams,
) -> PhyloResult {
    let freq = corpus_fossil_frequency(dbs);
    let n = dbs.len();
    let mut genotypes = Vec::with_capacity(n);
    for (i, db) in dbs.iter().enumerate() {
        let pheno = specs
            .get(i)
            .and_then(|s| s.map(|spec| ObservablesOmega::extract(spec, db)));
        let dt = delta_ts.get(i).copied().unwrap_or(1.0 + i as f64);
        genotypes.push(Genotype::from_evidence(
            db,
            Some(&freq),
            n,
            pheno,
            dt,
        ));
    }
    reconstruct_phylogeny(&genotypes, params)
}

fn detect_thc(
    genotypes: &[Genotype],
    pairs: &[PhyloPairStats],
    params: &PhyloParams,
) -> Vec<ThcEvent> {
    let by_label: HashMap<&str, &Genotype> =
        genotypes.iter().map(|g| (g.label.as_str(), g)).collect();
    let mut events = Vec::new();

    for p in pairs {
        if p.d_phi < params.distant_d_phi {
            continue;
        }
        let Some(ga) = by_label.get(p.a.as_str()) else {
            continue;
        };
        let Some(gb) = by_label.get(p.b.as_str()) else {
            continue;
        };
        // bloco coerente: fósseis partilhados com λ média alta e ids iguais
        let set_b: HashSet<_> = gb.fossil_ids();
        let shared: Vec<&GenotypeLocus> = ga
            .loci
            .iter()
            .filter(|l| set_b.contains(&l.fossil.id))
            .collect();
        if shared.len() < params.thc_min_block {
            continue;
        }
        // THC: ramos filogeneticamente distantes + bloco partilhado coerente
        // dens = |shared| / min(|A|,|B|); exigir dens mínima real (não auto-match)
        let dens =
            shared.len() as f64 / ga.loci.len().min(gb.loci.len()).max(1) as f64;
        if dens >= 0.12 && shared.len() >= params.thc_min_block {
            let sample: Vec<String> = shared
                .iter()
                .take(8)
                .map(|l| l.fossil.id.clone())
                .collect();
            events.push(ThcEvent {
                from: p.a.clone(),
                to: p.b.clone(),
                d_phi: p.d_phi,
                local_block_similarity: dens,
                block_size: shared.len(),
                sample_fossils: sample,
                note: "THC candidate: high d_φ + shared coherent fossil block".into(),
            });
        }
    }
    events
}

fn detect_homoplasy(
    genotypes: &[Genotype],
    pairs: &[PhyloPairStats],
    params: &PhyloParams,
) -> Vec<HomoplasyEvent> {
    let by_label: HashMap<&str, &Genotype> =
        genotypes.iter().map(|g| (g.label.as_str(), g)).collect();
    let mut out = Vec::new();
    for p in pairs {
        if p.d_phi < params.distant_d_phi {
            continue;
        }
        let Some(ga) = by_label.get(p.a.as_str()) else {
            continue;
        };
        let Some(gb) = by_label.get(p.b.as_str()) else {
            continue;
        };
        let sa = ga.fossil_ids();
        let sb = gb.fossil_ids();
        let identical = sa.intersection(&sb).count();
        let frac = identical as f64 / sa.len().max(sb.len()).max(1) as f64;
        // Homoplasia: idênticos sob pressão equivalente mas fração moderada
        // e sem densidade THC alta — atratores de hardware
        if identical > 0
            && frac >= params.homoplasy_identical_frac
            && frac < params.thc_local_sim
        {
            out.push(HomoplasyEvent {
                a: p.a.clone(),
                b: p.b.clone(),
                d_phi: p.d_phi,
                identical_fossils: identical,
                note: "Homoplasy candidate: identical fossils + high d_φ (convergence / attractor)"
                    .into(),
            });
        }
    }
    out
}

/// Neighbor-Joining clássico → árvore + Newick.
fn neighbor_joining(dist: &[Vec<f64>], labels: &[String]) -> (PhyloTreeNode, String) {
    let n0 = labels.len();
    if n0 == 0 {
        return (
            PhyloTreeNode {
                name: "empty".into(),
                children: vec![],
                branch_length: 0.0,
            },
            ";".into(),
        );
    }
    if n0 == 1 {
        let leaf = PhyloTreeNode {
            name: labels[0].clone(),
            children: vec![],
            branch_length: 0.0,
        };
        return (leaf.clone(), format!("{};", sanitize_newick(&labels[0])));
    }
    if n0 == 2 {
        let d = dist[0][1] / 2.0;
        let tree = PhyloTreeNode {
            name: "root".into(),
            branch_length: 0.0,
            children: vec![
                PhyloTreeNode {
                    name: labels[0].clone(),
                    children: vec![],
                    branch_length: d,
                },
                PhyloTreeNode {
                    name: labels[1].clone(),
                    children: vec![],
                    branch_length: d,
                },
            ],
        };
        let nw = format!(
            "({}:{:.6},{}:{:.6});",
            sanitize_newick(&labels[0]),
            d,
            sanitize_newick(&labels[1]),
            d
        );
        return (tree, nw);
    }

    // Working clusters: each is a tree node
    let mut nodes: Vec<PhyloTreeNode> = labels
        .iter()
        .map(|l| PhyloTreeNode {
            name: l.clone(),
            children: vec![],
            branch_length: 0.0,
        })
        .collect();
    let mut d = dist.to_vec();
    let mut active: Vec<usize> = (0..n0).collect();
    let mut next_id = n0;

    while active.len() > 2 {
        let m = active.len();
        // Q matrix
        let mut q = vec![vec![0.0; m]; m];
        let mut best = (0usize, 1usize, f64::INFINITY);
        for i in 0..m {
            for j in (i + 1)..m {
                let mut sum_i = 0.0;
                let mut sum_j = 0.0;
                for k in 0..m {
                    sum_i += d[active[i]][active[k]];
                    sum_j += d[active[j]][active[k]];
                }
                let qij = (m as f64 - 2.0) * d[active[i]][active[j]] - sum_i - sum_j;
                q[i][j] = qij;
                if qij < best.2 {
                    best = (i, j, qij);
                }
            }
        }
        let (ii, jj, _) = best;
        let i = active[ii];
        let j = active[jj];

        // branch lengths
        let mut sum_i = 0.0;
        let mut sum_j = 0.0;
        for &k in &active {
            sum_i += d[i][k];
            sum_j += d[j][k];
        }
        let dij = d[i][j];
        let li = 0.5 * dij + (sum_i - sum_j) / (2.0 * (m as f64 - 2.0).max(1.0));
        let lj = dij - li;
        let li = li.max(0.0);
        let lj = lj.max(0.0);

        let mut child_i = nodes[i].clone();
        child_i.branch_length = li;
        let mut child_j = nodes[j].clone();
        child_j.branch_length = lj;

        let u_name = format!("n{next_id}");
        next_id += 1;
        let u_node = PhyloTreeNode {
            name: u_name.clone(),
            children: vec![child_i, child_j],
            branch_length: 0.0,
        };

        // Expand distance matrix with u
        let u_idx = nodes.len();
        nodes.push(u_node);
        for row in d.iter_mut() {
            row.push(0.0);
        }
        d.push(vec![0.0; nodes.len()]);
        for &k in &active {
            if k == i || k == j {
                continue;
            }
            let du = 0.5 * (d[i][k] + d[j][k] - dij);
            d[u_idx][k] = du.max(0.0);
            d[k][u_idx] = du.max(0.0);
        }
        d[u_idx][u_idx] = 0.0;

        // remove i,j add u
        active.retain(|&x| x != i && x != j);
        active.push(u_idx);
    }

    // join last two
    let a = active[0];
    let b = active[1];
    let dab = d[a][b];
    let mut ca = nodes[a].clone();
    ca.branch_length = dab / 2.0;
    let mut cb = nodes[b].clone();
    cb.branch_length = dab / 2.0;
    let root = PhyloTreeNode {
        name: "root".into(),
        children: vec![ca, cb],
        branch_length: 0.0,
    };
    let newick = format!("{};", node_to_newick(&root));
    (root, newick)
}

fn node_to_newick(node: &PhyloTreeNode) -> String {
    if node.children.is_empty() {
        return format!("{}:{:.6}", sanitize_newick(&node.name), node.branch_length);
    }
    let inner: Vec<String> = node.children.iter().map(node_to_newick).collect();
    if node.name == "root" {
        format!("({})", inner.join(","))
    } else {
        format!(
            "({}){}:{:.6}",
            inner.join(","),
            sanitize_newick(&node.name),
            node.branch_length
        )
    }
}

fn sanitize_newick(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

impl PhyloResult {
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Filogenia Computacional — cladograma\n\n");
        md.push_str("> A Paleocomputação mapeia o que restou. A Filogenia mapeia o que se transmite.\n\n");
        md.push_str(&format!("## Newick\n\n```\n{}\n```\n\n", self.newick));
        md.push_str("## Distâncias d_φ\n\n");
        md.push_str("| A | B | Ψ | λ̄ | Δt | d_φ | shared |\n|---|---|---|---|---|---|--------|\n");
        for p in &self.pairs {
            md.push_str(&format!(
                "| `{}` | `{}` | {:.3} | {:.3} | {:.2} | **{:.3}** | {} |\n",
                p.a, p.b, p.psi, p.lambda_bar, p.delta_t, p.d_phi, p.shared_fossils
            ));
        }
        md.push_str("\n## THC (transferência horizontal)\n\n");
        if self.thc_events.is_empty() {
            md.push_str("- (nenhum candidato)\n");
        } else {
            for e in &self.thc_events {
                md.push_str(&format!(
                    "- `{}` ↔ `{}` · d_φ={:.3} · block={} · dens={:.2} — {}\n",
                    e.from, e.to, e.d_phi, e.block_size, e.local_block_similarity, e.note
                ));
            }
        }
        md.push_str("\n## Homoplasia\n\n");
        if self.homoplasy_events.is_empty() {
            md.push_str("- (nenhum candidato)\n");
        } else {
            for e in &self.homoplasy_events {
                md.push_str(&format!(
                    "- `{}` ↔ `{}` · d_φ={:.3} · identical={} — {}\n",
                    e.a, e.b, e.d_phi, e.identical_fossils, e.note
                ));
            }
        }
        md.push_str("\n## Genótipos (λ médio)\n\n");
        for g in &self.genotypes {
            let mean_l = if g.loci.is_empty() {
                0.0
            } else {
                g.loci.iter().map(|l| l.lineage_load).sum::<f64>() / g.loci.len() as f64
            };
            md.push_str(&format!(
                "- `{}`: loci={} · λ̄={:.3} · Δt={:.2}\n",
                g.label,
                g.loci.len(),
                mean_l,
                g.stratum_delta_t
            ));
        }
        md.push_str("\n## Honesty\n\n");
        md.push_str("- `generates_os: false` · `auto_fix_complete: false`\n");
        md.push_str(&format!("- {}\n", self.honesty));
        md
    }

    /// Mermaid flowchart aproximado do cladograma (árvore binária).
    pub fn to_mermaid(&self) -> String {
        let mut lines = vec![
            "flowchart TD".into(),
            "  %% Computational phylogeny cladogram".into(),
        ];
        fn walk(node: &PhyloTreeNode, lines: &mut Vec<String>, id: &mut usize) -> String {
            let my = format!("N{id}");
            *id += 1;
            let label = node.name.replace('"', "'");
            lines.push(format!("  {my}[\"{label}\"]"));
            for ch in &node.children {
                let cid = walk(ch, lines, id);
                lines.push(format!(
                    "  {my} -->|{:.3}| {cid}",
                    ch.branch_length
                ));
            }
            my
        }
        let mut id = 0usize;
        walk(&self.tree, &mut lines, &mut id);
        for e in &self.thc_events {
            lines.push(format!(
                "  %% THC: {} <-> {}",
                sanitize_newick(&e.from),
                sanitize_newick(&e.to)
            ));
        }
        lines.join("\n")
    }
}

// silence unused import warning if FossilPersistence only used via atom
#[allow(dead_code)]
fn _persist_ref(p: FossilPersistence) -> f64 {
    p.erosion_rate()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{EvidenceEntry, EvidenceType};

    fn db_with(source: &str, addrs: &[u64]) -> EvidenceDb {
        let mut db = EvidenceDb::new(source);
        for (i, &a) in addrs.iter().enumerate() {
            db.add(EvidenceEntry {
                id: format!("{source}_{i}"),
                evidence_type: EvidenceType::MmioWrite {
                    address: a,
                    value: Some(1),
                },
                context: Default::default(),
            });
        }
        db
    }

    #[test]
    fn related_lineages_closer_than_unrelated() {
        let a = db_with("v1", &[0x1000, 0x1004, 0x1008, 0x2000]);
        let b = db_with("v2", &[0x1000, 0x1004, 0x1008, 0x2004]); // patch
        let c = db_with("other", &[0x9000, 0x9004, 0x9008, 0x9010]);
        let dbs = [&a, &b, &c];
        let r = phylogeny_from_evidence(&dbs, &[None, None, None], &[1.0, 2.0, 10.0], &PhyloParams::default());
        let d_ab = r.pairs.iter().find(|p| {
            (p.a == "v1" && p.b == "v2") || (p.a == "v2" && p.b == "v1")
        }).unwrap().d_phi;
        let d_ac = r.pairs.iter().find(|p| {
            (p.a.contains("v1") && p.b.contains("other"))
                || (p.a.contains("other") && p.b.contains("v1"))
        }).unwrap().d_phi;
        assert!(
            d_ab < d_ac,
            "related d_φ={d_ab} should be < unrelated d_φ={d_ac}"
        );
        assert!(r.newick.ends_with(';'));
        assert!(!r.generates_os);
    }

    #[test]
    fn newick_three_taxa() {
        let a = db_with("A", &[1, 2, 3]);
        let b = db_with("B", &[1, 2, 4]);
        let c = db_with("C", &[1, 5, 6]);
        let r = phylogeny_from_evidence(
            &[&a, &b, &c],
            &[None, None, None],
            &[1.0, 1.0, 1.0],
            &PhyloParams::default(),
        );
        assert!(r.newick.contains('('));
        assert_eq!(r.labels.len(), 3);
    }
}
