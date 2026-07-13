use base_core::spec::types::{SynthesizedSpec};

/// Gerador de drivers para o novo hardware
pub struct DriverGenerator;

impl DriverGenerator {
    /// Gera drivers bare-metal em C
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

    /// Gera driver makefile ou CMakeLists
    pub fn generate_build_system(&self, _spec: &SynthesizedSpec) -> String {
        let mut mk = String::new();
        mk.push_str("# B.A.S.E. Generated Makefile\n\n");
        mk.push_str("CC = arm-none-eabi-gcc\n");
        mk.push_str("CFLAGS = -mcpu=cortex-m33 -mthumb -O2 -Wall -Wextra\n");
        mk.push_str("LDFLAGS = -T linker.ld --specs=nosys.specs\n\n");
        mk.push_str("SRCS = bootloader.c hal_mmio.c timing.c irq.c drivers.c main.c\n");
        mk.push_str("OBJS = $(SRCS:.c=.o)\n\n");
        mk.push_str("all: firmware.elf\n\n");
        mk.push_str("firmware.elf: $(OBJS)\n");
        mk.push_str("\t$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^\n\n");
        mk.push_str("clean:\n");
        mk.push_str("\trm -f $(OBJS) firmware.elf\n");
        mk
    }

    /// Gera linker script para o target
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

    fn generate_driver_for(&self, assignment: &base_core::spec::types::ComponentAssignment) -> String {
        match assignment.interface.as_str() {
            "spi" => self.gen_spi_driver(assignment),
            "i2c" => self.gen_i2c_driver(assignment),
            "uart" => self.gen_uart_driver(assignment),
            "usb" => self.gen_usb_driver(assignment),
            "gpio" => self.gen_gpio_driver(assignment),
            _ => String::new(),
        }
    }

    fn gen_spi_driver(&self, _assignment: &base_core::spec::types::ComponentAssignment) -> String {
        let mut code = String::new();
        code.push_str("// SPI Driver (RP2350)\n");
        code.push_str("static void spi_init(void) {\n");
        code.push_str("    spi_init(spi0, 10 * 1000 * 1000); // 10 MHz\n");
        code.push_str("    gpio_set_function(2, GPIO_FUNC_SPI); // SCK\n");
        code.push_str("    gpio_set_function(3, GPIO_FUNC_SPI); // TX\n");
        code.push_str("    gpio_set_function(4, GPIO_FUNC_SPI); // RX\n");
        code.push_str("    gpio_set_function(5, GPIO_FUNC_SPI); // CS\n");
        code.push_str("}\n\n");
        code.push_str("static uint8_t spi_xfer(uint8_t tx) {\n");
        code.push_str("    uint8_t rx = 0;\n");
        code.push_str("    spi_write_read_blocking(spi0, &tx, &rx, 1);\n");
        code.push_str("    return rx;\n");
        code.push_str("}\n\n");
        code
    }

    fn gen_i2c_driver(&self, _assignment: &base_core::spec::types::ComponentAssignment) -> String {
        let mut code = String::new();
        code.push_str("// I2C Driver (RP2350)\n");
        code.push_str("static void i2c_init(void) {\n");
        code.push_str("    i2c_init(i2c0, 400 * 1000); // 400 kHz\n");
        code.push_str("    gpio_set_function(0, GPIO_FUNC_I2C); // SDA\n");
        code.push_str("    gpio_set_function(1, GPIO_FUNC_I2C); // SCL\n");
        code.push_str("}\n\n");
        code
    }

    fn gen_uart_driver(&self, _assignment: &base_core::spec::types::ComponentAssignment) -> String {
        let mut code = String::new();
        code.push_str("// UART Driver (RP2350)\n");
        code.push_str("static void uart_init(void) {\n");
        code.push_str("    uart_init(uart0, 115200);\n");
        code.push_str("    gpio_set_function(0, GPIO_FUNC_UART); // TX\n");
        code.push_str("    gpio_set_function(1, GPIO_FUNC_UART); // RX\n");
        code.push_str("}\n\n");
        code.push_str("static void uart_putc(char c) {\n");
        code.push_str("    uart_putc_raw(uart0, c);\n");
        code.push_str("}\n\n");
        code
    }

    fn gen_usb_driver(&self, _assignment: &base_core::spec::types::ComponentAssignment) -> String {
        let mut code = String::new();
        code.push_str("// USB Driver (RP2350 device mode)\n");
        code.push_str("static void usb_init(void) {\n");
        code.push_str("    tusb_init();\n");
        code.push_str("}\n\n");
        code.push_str("static void usb_task(void) {\n");
        code.push_str("    tud_task();\n");
        code.push_str("}\n\n");
        code
    }

    fn gen_gpio_driver(&self, _assignment: &base_core::spec::types::ComponentAssignment) -> String {
        let mut code = String::new();
        code.push_str("// GPIO Driver (RP2350)\n");
        code.push_str("static void gpio_init(void) {\n");
        code.push_str("    // Configure GPIOs\n");
        code.push_str("    gpio_init(6);\n");
        code.push_str("    gpio_set_dir(6, GPIO_OUT);\n");
        code.push_str("    gpio_put(6, 0);\n");
        code.push_str("}\n\n");
        code
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
                    block_id: "spi_dev".into(), component: "W5500".into(),
                    interface: "spi".into(), config: Default::default(),
                },
                ComponentAssignment {
                    block_id: "i2c_dev".into(), component: "PCM5102A".into(),
                    interface: "i2c".into(), config: Default::default(),
                },
                ComponentAssignment {
                    block_id: "uart_debug".into(), component: "CP2102N".into(),
                    interface: "uart".into(), config: Default::default(),
                },
            ],
            netlist: None,
            constraints: SynthesisConstraints { max_bom_cost: None, preferred_manufacturer: None, preferred_package: None },
        }
    }

    #[test]
    fn test_driver_generation() {
        let gen = DriverGenerator;
        let spec = mock_spec();
        let code = gen.generate_baremetal(&spec);
        assert!(code.contains("SPI Driver"), "Should have SPI driver");
        assert!(code.contains("I2C Driver"), "Should have I2C driver");
        assert!(code.contains("UART Driver"), "Should have UART driver");
    }

    #[test]
    fn test_build_system() {
        let gen = DriverGenerator;
        let spec = mock_spec();
        let mk = gen.generate_build_system(&spec);
        assert!(mk.contains("Makefile"), "Should have Makefile");
        assert!(mk.contains("firmware.elf"), "Should have firmware target");
    }

    #[test]
    fn test_linker_script() {
        let gen = DriverGenerator;
        let spec = mock_spec();
        let ld = gen.generate_linker_script(&spec);
        assert!(ld.contains("MEMORY"), "Should have memory sections");
        assert!(ld.contains("FLASH"), "Should have FLASH section");
        assert!(ld.contains("SRAM"), "Should have SRAM section");
    }
}
