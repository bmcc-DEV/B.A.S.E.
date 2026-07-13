use base_core::spec::types::{NetSegment, SynthesizedSpec};

#[derive(Debug, Clone, Copy)]
struct Vec2 {
    x: f64,
    y: f64,
}

impl Vec2 {
    fn new(x: f64, y: f64) -> Self { Self { x, y } }
    fn length(self) -> f64 { (self.x * self.x + self.y * self.y).sqrt() }
    fn normalize(self) -> Self {
        let len = self.length().max(1.0);
        Self::new(self.x / len, self.y / len)
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self { Self::new(self.x + other.x, self.y + other.y) }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self { Self::new(self.x - other.x, self.y - other.y) }
}

impl std::ops::Mul<f64> for Vec2 {
    type Output = Self;
    fn mul(self, s: f64) -> Self { Self::new(self.x * s, self.y * s) }
}

#[derive(Debug, Clone, Copy)]
struct Pos {
    x: f64,
    y: f64,
}

impl Pos {
    fn new(x: f64, y: f64) -> Self { Self { x, y } }
}

impl std::ops::Add<Vec2> for Pos {
    type Output = Pos;
    fn add(self, v: Vec2) -> Pos { Pos::new(self.x + v.x, self.y + v.y) }
}

impl std::ops::Sub for Pos {
    type Output = Vec2;
    fn sub(self, other: Pos) -> Vec2 { Vec2::new(self.x - other.x, self.y - other.y) }
}

/// Posicionamento por força (Fruchterman-Reingold adaptado para PCB)
pub struct ForceDirectedPlacer {
    iterations: usize,
    attraction: f64,
    repulsion: f64,
    board_width: f64,
    board_height: f64,
}

impl ForceDirectedPlacer {
    pub fn new() -> Self {
        Self {
            iterations: 100,
            attraction: 5.0,
            repulsion: 10000.0,
            board_width: 100.0,
            board_height: 80.0,
        }
    }

    /// Calcula posições para cada componente baseado na netlist
    pub fn place(
        &self,
        spec: &SynthesizedSpec,
        netlist: &[NetSegment],
    ) -> Vec<(String, Pos)> {
        let n = spec.assignments.len();
        if n == 0 {
            return Vec::new();
        }

        // Inicializa posições aleatórias
        let mut positions: Vec<Pos> = (0..n)
            .map(|i| {
                let angle = i as f64 * 2.39996; // golden angle
                let radius = 10.0 + (i as f64 * 5.0);
                Pos::new(
                    self.board_width / 2.0 + angle.cos() * radius,
                    self.board_height / 2.0 + angle.sin() * radius,
                )
            })
            .collect();

        // Constrói matriz de adjacência (peso das conexões)
        let mut adj = vec![vec![0.0f64; n]; n];
        let names: Vec<&str> = spec.assignments.iter().map(|a| a.block_id.as_str()).collect();

        for net in netlist {
            if let (Some(ai), Some(bi)) = (
                names.iter().position(|n| *n == net.from.as_str()),
                names.iter().position(|n| *n == net.to.as_str()),
            ) {
                adj[ai][bi] += 1.0;
                adj[bi][ai] += 1.0;
            }
        }

        // Fruchterman-Reingold
        let mut temp = self.board_width.max(self.board_height) / 3.0;
        let k = (self.board_width * self.board_height / n as f64).sqrt();

        for _iter in 0..self.iterations {
            let mut forces = vec![Vec2::new(0.0, 0.0); n];

            // Repulsão (todos os pares)
            for i in 0..n {
                for j in (i + 1)..n {
                    let delta = positions[i] - positions[j];
                    let dist = delta.length().max(1.0);
                    let force = delta.normalize() * (self.repulsion * k / dist);
                    forces[i] = forces[i] + force;
                    forces[j] = forces[j] - force;
                }
            }

            // Atração (componentes conectados)
            for i in 0..n {
                for j in 0..n {
                    if adj[i][j] > 0.0 {
                        let delta = positions[j] - positions[i];
                        let dist = delta.length().max(1.0);
                        let weight = adj[i][j];
                        let force = delta.normalize() * (dist / k).ln() * self.attraction * weight;
                        forces[i] = forces[i] + force;
                    }
                }
            }

            // Força de borda (mantém dentro do board)
            let center = Pos::new(self.board_width / 2.0, self.board_height / 2.0);
            for i in 0..n {
                let from_center = positions[i] - center;
                let margin = 5.0;
                if (positions[i].x - margin) < 0.0 {
                    forces[i].x += 1.0;
                }
                if (positions[i].x + margin) > self.board_width {
                    forces[i].x -= 1.0;
                }
                if (positions[i].y - margin) < 0.0 {
                    forces[i].y += 1.0;
                }
                if (positions[i].y + margin) > self.board_height {
                    forces[i].y -= 1.0;
                }
            }

            // Aplica forças com cooling
            for i in 0..n {
                let f_len = forces[i].length();
                let max_move = temp.min(f_len);
                if f_len > 0.001 {
                    positions[i] = positions[i] + forces[i].normalize() * max_move;
                }
            }

            temp *= 0.95; // cooling
        }

        spec.assignments
            .iter()
            .enumerate()
            .map(|(i, a)| (a.block_id.clone(), positions[i]))
            .collect()
    }
}

impl Default for ForceDirectedPlacer {
    fn default() -> Self {
        Self::new()
    }
}

/// Gera o conteúdo do arquivo .kicad_pcb
pub fn generate_pcb_layout(spec: &SynthesizedSpec, netlist: &[NetSegment]) -> String {
    let placer = ForceDirectedPlacer::new();
    let placements = placer.place(spec, netlist);

    let mut pcb = String::new();
    pcb.push_str("(kicad_pcb (version 20231121) (generator \"base-pcb\")\n");
    pcb.push_str("  (page \"A4\")\n");
    pcb.push_str(&format!(
        "  (setup (stackup (layer \"F.Cu\") (layer \"B.Cu\"))))\n"
    ));

    // Gera footprints posicionados
    for (name, pos) in &placements {
        pcb.push_str(&format!(
            "  (footprint \"Package:Generic\"\n\
               (at {} {})\n\
               (layer \"F.Cu\")\n\
               (property \"Reference\" \"{}\" (at 0 0))\n\
             )\n",
            pos.x, pos.y, name
        ));
    }

    pcb.push_str(")\n");
    pcb
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::spec::types::*;

    fn mock_spec() -> SynthesizedSpec {
        SynthesizedSpec {
            original: HardwareSpec::empty(),
            assignments: vec![
                ComponentAssignment {
                    block_id: "cpu".into(),
                    component: "RP2350A".into(),
                    interface: "spi".into(),
                    config: Default::default(),
                },
                ComponentAssignment {
                    block_id: "gpu".into(),
                    component: "RP2350A".into(),
                    interface: "spi".into(),
                    config: Default::default(),
                },
                ComponentAssignment {
                    block_id: "audio".into(),
                    component: "PCM5102A".into(),
                    interface: "i2c".into(),
                    config: Default::default(),
                },
            ],
            netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        }
    }

    #[test]
    fn test_force_directed_placement() {
        let spec = mock_spec();
        let netlist = vec![];
        let placer = ForceDirectedPlacer::new();
        let placements = placer.place(&spec, &netlist);

        assert_eq!(placements.len(), 3, "Should place all components");
        for (name, pos) in &placements {
            assert!(pos.x.is_finite() && pos.y.is_finite(),
                "Position for {} should be finite: ({}, {})", name, pos.x, pos.y);
        }
    }

    #[test]
    fn test_pcb_layout_generation() {
        let spec = mock_spec();
        let netlist = vec![];
        let pcb = generate_pcb_layout(&spec, &netlist);
        assert!(pcb.contains("kicad_pcb"), "Should have KiCad header");
        assert!(pcb.contains("cpu"), "Should contain cpu footprint");
        assert!(pcb.contains("gpu"), "Should contain gpu footprint");
    }
}
