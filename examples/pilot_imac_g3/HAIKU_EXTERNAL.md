# Haiku — integração externa (iMac G3 / PowerPC)

B.A.S.E. **não** compila nem gera Haiku. Port PowerPC = tree **externa**.

## Factos

- BeOS histórico corria em PowerPC; Haiku moderno é sobretudo **x86 / x86_64**.
- Haiku PPC (se existir upstream ou fork) é experimental / incompleto — ≠ desktop turnkey no G3.
- Fase B recomenda primeiro um kernel de referência (LinuxPPC / OpenBSD) via `QEMU_PPC_KERNEL`.

## O que o Cliente/Prestador faz fora do repo

1. Obter tree Haiku + toolchain PowerPC (se disponível) **ou** usar OS de referência no G3 para fase B/C.
2. Apontar path da imagem: `HAIKU_IMAGE=…` (e opcionalmente `QEMU_PPC_KERNEL=…` para referência).
3. Correr fase A neste pilot; B com QEMU/`mac99`; C em Late 2001 físico só após B verde.

## O que o B.A.S.E. entrega

- Contratos OF/MacIO (fase A)
- `port package` com `--target-hal hal_haiku_ppc_assist`
- Checklist SOW / playbook / receipts — **não** o kernel Haiku

## Claims proibidos

- “B.A.S.E. porta Haiku turnkey no iMac G3”
- `generates_os=true` / `generates_haiku=true` / `production: true`

Ref: [EXTERNAL_PORT_ROADMAP.md](EXTERNAL_PORT_ROADMAP.md) · `base-vault/24 - Path to v1.4/24.21 - SOW OS-Port Checklist.md`
