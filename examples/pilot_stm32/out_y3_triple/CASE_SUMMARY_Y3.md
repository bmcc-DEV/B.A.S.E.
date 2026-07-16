# Y3 STM32 CASE SUMMARY

- Triple wedge: USART1 @ 0x40013800 + SPI2 @ 0x40003800 + I2C1 @ 0x40005400
- Classify: `0x40013000=uart,0x40003000=spi,0x40005000=i2c`
- Pins: PA9/10 + PB13/14/15 + PB6/7 (NOT FABRICABLE)
- Prefer manufacturer: STMicroelectronics → STM32F103C8
- Gates USART / SPI / I2C isolados intocados
- design bytes: 1337
- status: OK
