device SPI @ 0x4003c000 {
  registers {
    CR0 @ 0x00: rw;
    CR1 @ 0x04: rw;
    DR @ 0x08: rw;
    SR @ 0x0c: ro;
    ICR @ 0x20: wo;
  }
  events {
    INIT: write CR0[0] = 1;
    STATUS: read SR[0] = 0;
    CLEAR: write ICR[0] = 0;
    XFER: write DR[0] = 85;
  }
  interrupts {
    SPI_IRQ: level high 17;
  }
  timing {
    xfer: 100ns..2000ns;
  }
  contract {
    must_occur_before: INIT -> STATUS;
    must_occur_before: STATUS -> SPI_IRQ;
    must_occur_before: XFER -> SPI_IRQ;
    window: 5us;
  }
}
