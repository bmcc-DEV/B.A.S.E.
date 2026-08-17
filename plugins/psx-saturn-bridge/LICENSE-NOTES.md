# Licenças — plugin isolado

## Vendor `psxrecomp`

- Upstream: https://github.com/mstan/psxrecomp
- Copyright (c) 2026 Matthew Stan
- License: **PolyForm Noncommercial License 1.0.0** (`vendor/psxrecomp/LICENSE`)
- Uso neste repo: **referência / estudo / hobby** — não sublicenciar no núcleo comercial B.A.S.E.
- Não incluir BIOS (`SCPH1001.BIN`), discos, saves ou `overlay_captures.json` no git.

## SaturnRingLib

- Path local: `/media/bruno/Bruno/SaturnRingLib` (symlink `SaturnRingLib/`)
- Upstream típico: https://github.com/ReyeMe/SaturnRingLib
- Docs: https://srl.reye.me/
- Código original do bridge (`mapping/`, `templates/`, `scripts/`) é do B.A.S.E.; APIs SRL pertencem aos autores da biblioteca.

## Isolamento

Código sob `plugins/psx-saturn-bridge/` **não** entra no workspace Cargo do núcleo.
Não linkar `vendor/psxrecomp/runtime` a `base-*` crates.
