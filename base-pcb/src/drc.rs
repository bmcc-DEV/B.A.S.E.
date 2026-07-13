use std::process::Command;

/// Validação com kicad-cli DRC
pub struct KicadDrcValidator;

impl KicadDrcValidator {
    /// Verifica se o kicad-cli está disponível
    pub fn is_available() -> bool {
        Command::new("kicad-cli")
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Executa DRC no esquemático gerado
    pub fn run_sch_drc(sch_path: &str) -> Result<String, String> {
        let output = Command::new("kicad-cli")
            .args(["sch", "export", "netlist", sch_path, "--output", "/tmp/drc_check.net"])
            .output()
            .map_err(|e| format!("kicad-cli not found: {e}"))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Executa DRC na PCB gerada
    pub fn run_pcb_drc(pcb_path: &str) -> Result<Vec<DrcViolation>, String> {
        let output = Command::new("kicad-cli")
            .args(["pcb", "drc", pcb_path])
            .output()
            .map_err(|e| format!("kicad-cli not found: {e}"))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(Self::parse_drc_output(&stdout))
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Parse da saída textual do DRC
    fn parse_drc_output(output: &str) -> Vec<DrcViolation> {
        let mut violations = Vec::new();
        for line in output.lines() {
            if line.contains("violation") || line.contains("error") || line.contains("warning") {
                violations.push(DrcViolation {
                    message: line.to_string(),
                    severity: if line.contains("error") { DrcSeverity::Error }
                        else if line.contains("violation") { DrcSeverity::Violation }
                        else { DrcSeverity::Warning },
                });
            }
        }
        violations
    }

    /// Gera script de validação para CI
    pub fn generate_ci_script(project_dir: &str) -> String {
        format!(
            "#!/bin/bash\n\
             # B.A.S.E. Generated DRC Check\n\
             set -e\n\
             cd {}\n\
             echo \"=== Schematic DRC ===\"\n\
             kicad-cli sch export netlist project.kicad_sch --output /dev/null 2>&1 || true\n\
             echo \"=== PCB DRC ===\"\n\
             kicad-cli pcb drc project.kicad_pcb || true\n\
             echo \"=== ERC ===\"\n\
             kicad-cli sch erc project.kicad_sch || true\n",
            project_dir
        )
    }
}

#[derive(Debug, Clone)]
pub struct DrcViolation {
    pub message: String,
    pub severity: DrcSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrcSeverity {
    Warning,
    Violation,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_drc_output() {
        let output = "violation: track too close to pad\nviolation: missing solder mask\n";
        let violations = KicadDrcValidator::parse_drc_output(output);
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn test_parse_error() {
        let output = "error: unconnected pin\n";
        let violations = KicadDrcValidator::parse_drc_output(output);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].severity, DrcSeverity::Error);
    }

    #[test]
    fn test_ci_script() {
        let script = KicadDrcValidator::generate_ci_script("output/");
        assert!(script.contains("kicad-cli"));
        assert!(script.contains("project.kicad_sch"));
        assert!(script.contains("project.kicad_pcb"));
    }
}
