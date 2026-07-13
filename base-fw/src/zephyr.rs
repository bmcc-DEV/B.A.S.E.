use base_core::spec::types::{SynthesizedSpec};

/// Gerador de módulo Zephyr RTOS
pub struct ZephyrGenerator;

impl ZephyrGenerator {
    /// Gera Kconfig para o módulo
    pub fn generate_kconfig(&self, _spec: &SynthesizedSpec) -> String {
        let mut kconfig = String::new();
        kconfig.push_str("# B.A.S.E. Generated Zephyr Module\n\n");
        kconfig.push_str("config BASE_FW_SYNTH\n");
        kconfig.push_str("\tbool \"B.A.S.E. Synthetic Firmware\"\n");
        kconfig.push_str("\tdefault y\n");
        kconfig.push_str("\thelp\n");
        kconfig.push_str("\t  Behavioral ASIC Synthesis Engine firmware\n\n");
        kconfig.push_str("config BASE_MMIO_TRANSLATION\n");
        kconfig.push_str("\tbool \"MMIO Translation Layer\"\n");
        kconfig.push_str("\ty\n\n");
        kconfig.push_str("config BASE_TIMING_COMPENSATION\n");
        kconfig.push_str("\tbool \"Timing Compensation\"\n");
        kconfig.push_str("\ty\n");
        kconfig
    }

    /// Gera devicetree overlay (.dts)
    pub fn generate_devicetree(&self, spec: &SynthesizedSpec) -> String {
        let mut dts = String::new();
        dts.push_str("/* B.A.S.E. Generated Devicetree Overlay */\n");
        dts.push_str("/ {\n");
        dts.push_str("    base-fw {\n");
        dts.push_str("        compatible = \"base,synthetic-firmware\";\n\n");

        for assignment in &spec.assignments {
            dts.push_str(&format!(
                "        {}: {} {{\n",
                assignment.block_id, assignment.interface
            ));
            dts.push_str(&format!(
                "            compatible = \"base,{}\";\n",
                assignment.component.to_lowercase()
            ));
            dts.push_str("            status = \"okay\";\n");
            dts.push_str("        };\n\n");
        }

        dts.push_str("    };\n");
        dts.push_str("};\n");
        dts
    }

    /// Gera CMakeLists.txt para o módulo Zephyr
    pub fn generate_cmake(&self, _spec: &SynthesizedSpec) -> String {
        let mut cmake = String::new();
        cmake.push_str("# B.A.S.E. Generated Zephyr CMakeLists\n\n");
        cmake.push_str("cmake_minimum_required(VERSION 3.20)\n");
        cmake.push_str("find_package(Zephyr REQUIRED HINTS $ENV{ZEPHYR_BASE})\n");
        cmake.push_str("project(base-fw)\n\n");
        cmake.push_str("target_sources(app PRIVATE\n");
        cmake.push_str("    bootloader.c\n");
        cmake.push_str("    hal_mmio.c\n");
        cmake.push_str("    timing.c\n");
        cmake.push_str("    irq.c\n");
        cmake.push_str("    drivers.c\n");
        cmake.push_str("    main.c\n");
        cmake.push_str(")\n\n");
        cmake.push_str("zephyr_include_directories(include)\n");
        cmake
    }

    /// Gera prj.conf para o módulo Zephyr
    pub fn generate_prj_conf(&self, _spec: &SynthesizedSpec) -> String {
        let mut conf = String::new();
        conf.push_str("# B.A.S.E. Generated Zephyr prj.conf\n\n");
        conf.push_str("CONFIG_BASE_FW_SYNTH=y\n");
        conf.push_str("CONFIG_BASE_MMIO_TRANSLATION=y\n");
        conf.push_str("CONFIG_BASE_TIMING_COMPENSATION=y\n");
        conf.push_str("CONFIG_GPIO=y\n");
        conf.push_str("CONFIG_SPI=y\n");
        conf.push_str("CONFIG_I2C=y\n");
        conf.push_str("CONFIG_UART_INTERRUPT_DRIVEN=y\n");
        conf.push_str("CONFIG_LOG=y\n");
        conf
    }

    /// Gera estrutura completa do módulo Zephyr
    pub fn generate_module(&self, spec: &SynthesizedSpec) -> Vec<(String, String)> {
        vec![
            ("Kconfig".into(), self.generate_kconfig(spec)),
            ("base-fw.overlay".into(), self.generate_devicetree(spec)),
            ("CMakeLists.txt".into(), self.generate_cmake(spec)),
            ("prj.conf".into(), self.generate_prj_conf(spec)),
        ]
    }
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
                    block_id: "gpu_spi".into(), component: "RP2350A".into(),
                    interface: "spi".into(), config: Default::default(),
                },
                ComponentAssignment {
                    block_id: "audio_i2c".into(), component: "PCM5102A".into(),
                    interface: "i2c".into(), config: Default::default(),
                },
            ],
            netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        }
    }

    #[test]
    fn test_kconfig() {
        let gen = ZephyrGenerator;
        assert!(gen.generate_kconfig(&mock_spec()).contains("BASE_FW_SYNTH"));
    }

    #[test]
    fn test_devicetree() {
        let gen = ZephyrGenerator;
        let dts = gen.generate_devicetree(&mock_spec());
        assert!(dts.contains("gpu_spi"));
        assert!(dts.contains("audio_i2c"));
        assert!(dts.contains("base,synthetic-firmware"));
    }

    #[test]
    fn test_module_structure() {
        let gen = ZephyrGenerator;
        let files = gen.generate_module(&mock_spec());
        assert_eq!(files.len(), 4);
        assert!(files.iter().any(|(n, _)| n == "Kconfig"));
        assert!(files.iter().any(|(n, _)| n == "CMakeLists.txt"));
    }
}
