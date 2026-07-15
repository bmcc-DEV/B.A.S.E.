device UART @ 0x40034000 {
  registers {
    DR @ 0x00: rw;
    FR @ 0x04: ro;
    ICR @ 0x18: wo;
  }
  events {
    INIT: write DR[0] = 1;
    STATUS: read FR[0] = 0;
    CLEAR: write ICR[0] = 0;
    TX: write DR[0] = 65;
  }
  interrupts {
    UART_IRQ: level high 16;
  }
  timing {
    char: 100ns..2000ns;
  }
  contract {
    must_occur_before: INIT -> STATUS;
    must_occur_before: STATUS -> UART_IRQ;
    must_occur_before: TX -> UART_IRQ;
    window: 5us;
  }
}
