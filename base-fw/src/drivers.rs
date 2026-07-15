use base_core::spec::types::SynthesizedSpec;

/// Gerador de drivers para o novo hardware (soft / host-testable).
pub struct DriverGenerator;

impl DriverGenerator {
    pub fn generate_baremetal(&self, spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("/* B.A.S.E. Generated Bare-metal Drivers */\n\n");
        code.push_str("#include <stdint.h>\n");
        code.push_str("#include <stdbool.h>\n\n");

        for assignment in &spec.assignments {
            code.push_str(&self.generate_driver_for(assignment));
        }

        code
    }

    /// Gera `main.c` — entry point real referenciado pelo Makefile.
    pub fn generate_main(&self, spec: &SynthesizedSpec) -> String {
        let mut code = String::new();
        code.push_str("/* B.A.S.E. Generated main */\n");
        code.push_str("#include <stdint.h>\n\n");
        code.push_str("void bootloader_main(void);\n");
        code.push_str("uint32_t mmio_read(uint32_t addr);\n");
        code.push_str("void mmio_write(uint32_t addr, uint32_t value);\n\n");

        if !spec.original.blocks.is_empty() {
            let first = &spec.original.blocks[0];
            code.push_str(&format!(
                "/* Smoke: touch first block {} @ 0x{:08x} */\n",
                first.id, first.base_address
            ));
            code.push_str("static void peripheral_smoke(void) {\n");
            code.push_str(&format!(
                "    uint32_t v = mmio_read(0x{:08x});\n",
                first.base_address as u32
            ));
            code.push_str(&format!(
                "    mmio_write(0x{:08x}, v);\n",
                first.base_address as u32
            ));
            code.push_str("}\n\n");
        } else {
            code.push_str("static void peripheral_smoke(void) {}\n\n");
        }

        code.push_str("int main(void) {\n");
        code.push_str("    bootloader_main();\n");
        code.push_str("    peripheral_smoke();\n");
        code.push_str("#ifdef HOST_BUILD\n");
        code.push_str("    return 0;\n");
        code.push_str("#else\n");
        code.push_str("    for (;;) {}\n");
        code.push_str("#endif\n");
        code.push_str("}\n");
        code
    }

    pub fn generate_build_system(&self, _spec: &SynthesizedSpec) -> String {
        let mut mk = String::new();
        mk.push_str("# B.A.S.E. Generated Makefile\n\n");
        mk.push_str("SRCS = bootloader.c hal_mmio.c timing.c irq.c drivers.c main.c\n");
        mk.push_str("OBJS = $(SRCS:.c=.o)\n\n");
        mk.push_str(".PHONY: all host clean\n\n");
        mk.push_str("all: host\n\n");
        mk.push_str("# Host smoke build (no ARM toolchain required)\n");
        mk.push_str("host: CFLAGS = -std=c11 -Wall -Wextra -DHOST_BUILD -O0 -g\n");
        mk.push_str("host: CC ?= cc\n");
        mk.push_str("host: firmware_host\n\n");
        mk.push_str("firmware_host: $(SRCS)\n");
        mk.push_str("\t$(CC) $(CFLAGS) -o $@ $^\n\n");
        mk.push_str("# Optional cross build\n");
        mk.push_str("CROSS_CC ?= arm-none-eabi-gcc\n");
        mk.push_str("CROSS_CFLAGS = -mcpu=cortex-m33 -mthumb -O2 -Wall -Wextra\n");
        mk.push_str("CROSS_LDFLAGS = -T linker.ld --specs=nosys.specs\n\n");
        mk.push_str("firmware.elf: $(SRCS) linker.ld\n");
        mk.push_str("\t$(CROSS_CC) $(CROSS_CFLAGS) $(CROSS_LDFLAGS) -o $@ $(SRCS)\n\n");
        mk.push_str("clean:\n");
        mk.push_str("\trm -f $(OBJS) firmware.elf firmware_host\n");
        mk
    }

    pub fn generate_linker_script(&self, _spec: &SynthesizedSpec) -> String {
        let mut ld = String::new();
        ld.push_str("/* B.A.S.E. Generated Linker Script */\n\n");
        ld.push_str("MEMORY\n");
        ld.push_str("{\n");
        ld.push_str("    FLASH (rx)  : ORIGIN = 0x10000000, LENGTH = 4M\n");
        ld.push_str("    SRAM  (rwx) : ORIGIN = 0x20000000, LENGTH = 520K\n");
        ld.push_str("    PSRAM (rw)  : ORIGIN = 0x30000000, LENGTH = 8M\n");
        ld.push_str("}\n\n");
        ld.push_str("SECTIONS\n");
        ld.push_str("{\n");
        ld.push_str("    .text : { *(.text*) } > FLASH\n");
        ld.push_str("    .data : { *(.data*) } > SRAM AT > FLASH\n");
        ld.push_str("    .bss  : { *(.bss*)  } > SRAM\n");
        ld.push_str("    .fw_load : { _fw_load_addr = .; } > PSRAM\n");
        ld.push_str("}\n");
        ld
    }

    fn generate_driver_for(
        &self,
        assignment: &base_core::spec::types::ComponentAssignment,
    ) -> String {
        match assignment.interface.as_str() {
            "spi" => self.gen_spi_driver(assignment),
            "i2c" => self.gen_i2c_driver(assignment),
            "uart" => self.gen_uart_driver(assignment),
            "usb" => self.gen_usb_driver(assignment),
            "gpio" => self.gen_gpio_driver(assignment),
            _ => String::new(),
        }
    }

    fn gen_spi_driver(&self, assignment: &base_core::spec::types::ComponentAssignment) -> String {
        format!(
            "/* SPI soft driver for {} ({}) */\n\
             static uint8_t spi_soft_buf[64];\n\
             static void spi_init(void) {{ /* soft */ }}\n\
             static uint8_t spi_xfer(uint8_t tx) {{\n\
                 static unsigned i = 0;\n\
                 uint8_t rx = spi_soft_buf[i & 63u];\n\
                 spi_soft_buf[i & 63u] = tx;\n\
                 i++;\n\
                 return rx;\n\
             }}\n\n",
            assignment.component, assignment.block_id
        )
    }

    fn gen_i2c_driver(&self, assignment: &base_core::spec::types::ComponentAssignment) -> String {
        format!(
            "/* I2C soft driver for {} ({}) */\n\
             static void i2c_init(void) {{ /* soft */ }}\n\n",
            assignment.component, assignment.block_id
        )
    }

    fn gen_uart_driver(&self, assignment: &base_core::spec::types::ComponentAssignment) -> String {
        format!(
            "/* UART soft driver for {} ({}) */\n\
             static void uart_init(void) {{ /* soft */ }}\n\
             static void uart_putc(char c) {{ (void)c; }}\n\n",
            assignment.component, assignment.block_id
        )
    }

    fn gen_usb_driver(&self, assignment: &base_core::spec::types::ComponentAssignment) -> String {
        format!(
            "/* USB soft driver for {} ({}) */\n\
             static void usb_init(void) {{ /* soft */ }}\n\
             static void usb_task(void) {{ /* soft */ }}\n\n",
            assignment.component, assignment.block_id
        )
    }

    fn gen_gpio_driver(&self, assignment: &base_core::spec::types::ComponentAssignment) -> String {
        format!(
            "/* GPIO soft driver for {} ({}) */\n\
             static uint32_t gpio_soft_state;\n\
             static void gpio_init_soft(void) {{ gpio_soft_state = 0; }}\n\n",
            assignment.component, assignment.block_id
        )
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
                    block_id: "spi_dev".into(),
                    component: "W5500".into(),
                    interface: "spi".into(),
                    config: Default::default(),
                },
                ComponentAssignment {
                    block_id: "i2c_dev".into(),
                    component: "PCM5102A".into(),
                    interface: "i2c".into(),
                    config: Default::default(),
                },
                ComponentAssignment {
                    block_id: "uart_debug".into(),
                    component: "CP2102N".into(),
                    interface: "uart".into(),
                    config: Default::default(),
                },
            ],
            netlist: None,
            constraints: SynthesisConstraints {
                max_bom_cost: None,
                preferred_manufacturer: None,
                preferred_package: None,
            },
        }
    }

    #[test]
    fn test_driver_generation() {
        let gen = DriverGenerator;
        let spec = mock_spec();
        let code = gen.generate_baremetal(&spec);
        assert!(code.contains("SPI soft driver"));
        assert!(code.contains("I2C soft driver"));
        assert!(code.contains("UART soft driver"));
    }

    #[test]
    fn test_main_generation() {
        let gen = DriverGenerator;
        let code = gen.generate_main(&mock_spec());
        assert!(code.contains("int main"));
        assert!(code.contains("bootloader_main"));
    }

    #[test]
    fn test_build_system() {
        let gen = DriverGenerator;
        let mk = gen.generate_build_system(&mock_spec());
        assert!(mk.contains("HOST_BUILD"));
        assert!(mk.contains("firmware_host"));
        assert!(mk.contains("main.c"));
    }

    #[test]
    fn test_linker_script() {
        let gen = DriverGenerator;
        let ld = gen.generate_linker_script(&mock_spec());
        assert!(ld.contains("MEMORY"));
        assert!(ld.contains("FLASH"));
    }
}
