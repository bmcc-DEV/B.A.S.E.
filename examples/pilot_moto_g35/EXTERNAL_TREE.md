# Tree externo — caminho A → **postmarketOS**

> `generates_os: false` · flash **manual** · ≠ earlycon garantido.
> Alvo de OS: **postmarketOS** (Alpine, sem Android userspace) — ver [POSTMARKETOS.md](POSTMARKETOS.md).

## One-shot B.A.S.E.

```bash
./examples/pilot_moto_g35/run_path_a.sh
```

→ `out_real/handoff_external/` (DTSI + clocks resolvidos + receipt + lab watch)

## No postmarketOS (resumo)

1. `pmbootstrap init` → device **motorola-manila** (novo port)
2. Copia `dt/board-ums9620-wedge-merged.dtsi` para o kernel package
3. Cmdline: `cmdline/cmdline_earlycon.txt`
4. `pmbootstrap build` / `install` / `export`
5. Flash **manual** → `lab/lab_watch_assist.sh`

Guia completo: **[POSTMARKETOS.md](POSTMARKETOS.md)**

## UART0 (ums9620)

```
clocks = <&apapb_gate 7>, <&ap_clk 3>, <&ext_26m>;
earlycon=uart8250,mmio32,0x20200000,115200n8
```

## Continua manual

- Defconfig / downstream kernel Unisoc
- Flash fastboot/EDL
- UFS, display, modem (modem ≠ “sem blobs” na prática)
