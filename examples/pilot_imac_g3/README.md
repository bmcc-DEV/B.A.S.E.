# Pilot — iMac G3 late 2001 (OS Port Validation Assist)

PowerPC G3 / OpenFirmware / MacIO. Dual alvo **externo**: ReactOS + Haiku.  
**≠** ReactOS PPC completo · **≠** Haiku turnkey · `generates_os=false`.

| Fase | Script |
|------|--------|
| A | `./run.sh` — contratos OF/MacIO + dual `port package` |
| B | `./run_qemu_smoke.sh` — QEMU `mac99` (ver env abaixo) |
| C | [SOP.md](SOP.md) — Late 2001 físico (depois de B verde) |

## Documentação externa

| Doc | Papel |
|-----|--------|
| [REACTOS_EXTERNAL.md](REACTOS_EXTERNAL.md) | ReactOS PPC retired |
| [HAIKU_EXTERNAL.md](HAIKU_EXTERNAL.md) | Haiku PPC experimental |
| [EXTERNAL_PORT_ROADMAP.md](EXTERNAL_PORT_ROADMAP.md) | Roadmap fora do repo |

## Fase B — QEMU

```bash
# Referência (caminho verde): LinuxPPC / OpenBSD
QEMU_PPC_KERNEL=/path/to/vmlinux ./run_qemu_smoke.sh

# Opcionais (skip auditável se ausentes)
HAIKU_IMAGE=/path/to/haiku.img REACTOS_IMAGE=/path/to/reactos.img ./run_qemu_smoke.sh
```

Comando mínimo equivalente:

```bash
qemu-system-ppc -M mac99 -m 256 -nographic -kernel "$QEMU_PPC_KERNEL"
```

Vault: [[06.04 iMac G3 late 2001]] · Path to v1.4
