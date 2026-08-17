# External port roadmap — Haiku + ReactOS → iMac G3 (Late 2001)

Trabalho **fora** do repo B.A.S.E. O piloto só entrega contratos OF/MacIO, atlas `port package`, smoke QEMU e SOP.

Honesty: `generates_os=false` · Capstone PPC fora desta path · ≠ turnkey.

## Ordem de boot

1. **QEMU** `qemu-system-ppc -M mac99` — referência verde primeiro
2. Imagens Haiku / ReactOS se existirem
3. **Late 2001 físico** só após B verde com pelo menos a referência

## ReactOS PPC (retired)

| Passo | Acção |
|-------|--------|
| 1 | Localizar tree/artefactos Art Yerkes / RosBE-PPC (histórico) |
| 2 | Toolchain cross PowerPC |
| 3 | Objetivo mínimo: stub kernel que imprime e retorna sob QEMU |
| 4 | Drivers MacIO / OpenFirmware = esforço humano longo |
| 5 | Smoke: `REACTOS_IMAGE=… ./run_qemu_smoke.sh` |

B.A.S.E. entrega: contratos + `hal_reactos_ppc_assist` atlas — **não** o kernel.

Ref: https://reactos.org/wiki/PowerPC · [REACTOS_EXTERNAL.md](REACTOS_EXTERNAL.md)

## Haiku PPC

| Passo | Acção |
|-------|--------|
| 1 | Inventariar estado upstream PowerPC / forks BeOS-era |
| 2 | Toolchain + imagem boot OF-compatible |
| 3 | Smoke QEMU antes de hardware |
| 4 | Smoke: `HAIKU_IMAGE=… ./run_qemu_smoke.sh` |

B.A.S.E. entrega: contratos + `hal_haiku_ppc_assist` atlas — **não** o kernel.

Ref: [HAIKU_EXTERNAL.md](HAIKU_EXTERNAL.md)

## Referência (caminho verde QEMU)

Usar LinuxPPC ou OpenBSD como `QEMU_PPC_KERNEL` para obter receipt B auditável enquanto as trees Haiku/ReactOS avançam.

```bash
QEMU_PPC_KERNEL=/path/to/vmlinux ./examples/pilot_imac_g3/run_qemu_smoke.sh
```

## Critério de promoção

- Smoke QEMU com log + receipt JSON (`production: false`)
- **Nunca** claim turnkey no README / SOW
- Fase C Late 2001: [SOP.md](SOP.md) + `hw_boot_receipt.json`
