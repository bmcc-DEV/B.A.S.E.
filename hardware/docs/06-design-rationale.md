# Design rationale

## Por que RP2350?

Alinhado aos wedges RP UART/SPI do B.A.S.E. e ao template `rp2350_minimal` em `base-pcb`. Dual-core + PIO úteis para lab; não implica que todo target RE seja RP2350.

## Por que 2 camadas no draft?

Custo e simplicidade de fab no caminho de aprendizado. Designs densos ou impedância controlada podem subir para 4L sob EE (atualizar `fabrication-notes.md`).

## Por que USB-C?

Padrão de lab; template `usb_c` já no crate. Pinout CC/USB2.0 a validar no layout real (B3).

## Por que banner `NOT FABRICABLE`?

Honesty do Industrial Gate (Claim B): o CLI orquestra símbolos e labels de função; **não** substitui footprints revisados, DRC elétrico e aceite EE. Remover o banner só após B2–B5.

## Por que este pacote `hardware/` separado do output do pipeline?

Outputs efémeros (`out/06_pcb/`) não são fonte de verdade. Este diretório versiona o pacote OSHWA-style que sobrevive a limpezas de smoke e recebe o trabalho humano.
