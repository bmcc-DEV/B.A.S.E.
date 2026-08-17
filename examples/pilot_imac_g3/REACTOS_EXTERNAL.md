# ReactOS — integração externa (iMac G3 / PowerPC)

B.A.S.E. **não** compila nem revive o port PowerPC do ReactOS.

## Factos

- Port PPC ReactOS: histórico (Art Yerkes); código *retired* upstream.
- Tree ativa: x86 / x64 (+ ARM lento).
- RosBE-PPC: artefacto histórico para builds antigos.
- Fase B recomenda primeiro um kernel de referência (LinuxPPC / OpenBSD) via `QEMU_PPC_KERNEL`.

## O que o Cliente/Prestador faz fora do repo

1. Obter tree ReactOS + toolchain PPC (se disponível) **ou** usar OS alternativo no G3 (ex. OpenBSD/LinuxPPC) para fase B/C.
2. Apontar path da imagem no SOW (`REACTOS_IMAGE=…` / `QEMU_PPC_KERNEL=…`).
3. Correr fase A neste pilot; B/C com QEMU/`mac99` ou Late 2001 físico.

## O que o B.A.S.E. entrega

- Contratos OF/MacIO (fase A)
- `port package` com `--target-hal hal_reactos_ppc_assist`
- Checklist SOW / playbook
- Goldens `diff` — não o kernel ReactOS

## Claims proibidos

- “B.A.S.E. porta ReactOS turnkey no iMac G3”
- `generates_os=true` / `generates_reactos=true` / `production: true`

Ref: https://reactos.org/wiki/PowerPC · [EXTERNAL_PORT_ROADMAP.md](EXTERNAL_PORT_ROADMAP.md) · `base-vault/24 - Path to v1.4/24.21 - SOW OS-Port Checklist.md`
