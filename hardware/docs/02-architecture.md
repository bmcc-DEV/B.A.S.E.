# Arquitetura — placa de referência B.A.S.E.

```text
                    ┌─────────────┐
         USB-C ────►│  5V → 3V3   │
                    │  (LDO TBD)  │
                    └──────┬──────┘
                           │ 3V3
              ┌────────────┼────────────┐
              │            ▼            │
              │     ┌────────────┐      │
   SWD ───────┼────►│  RP2350A   │◄─────┼─── cristal 12 MHz
              │     │   (U1)     │      │
              │     └─────┬──────┘      │
              │           │ QSPI        │
              │           ▼             │
              │     ┌────────────┐      │
              │     │ W25Q Flash │      │
              │     └────────────┘      │
              │                         │
              │   GPIO → UART/SPI/I2C   │
              └─────────────────────────┘
```

Blocos de template em `base-pcb/templates/`:

| Template | Papel |
|----------|-------|
| `rp2350_minimal.kicad_sch` | MCU + cristal + flash |
| `usb_c.kicad_sch` | Conector USB-C |
| `power_3v3.kicad_sch` | Trilho 3V3 |
| `ethernet.kicad_sch` | Opcional |
| `audio_codec.kicad_sch` | Opcional |

Orquestração lógica do draft ≠ netlist elétrico revisado (B3).
