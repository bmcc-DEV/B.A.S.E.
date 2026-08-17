#!/usr/bin/env bash
# Valida estrutura do pacote hardware + honesty do draft.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAIL=0

req() {
  if [[ ! -e "$1" ]]; then
    echo "FALTA: $1"
    FAIL=1
  else
    echo "ok: $1"
  fi
}

echo "== estrutura =="
req "$ROOT/README.md"
req "$ROOT/docs/00-overview.md"
req "$ROOT/docs/01-specifications.md"
req "$ROOT/docs/02-architecture.md"
req "$ROOT/docs/03-assembly-guide.md"
req "$ROOT/docs/04-testing-procedures.md"
req "$ROOT/docs/05-troubleshooting.md"
req "$ROOT/docs/06-design-rationale.md"
req "$ROOT/docs/CHECKLIST.md"
req "$ROOT/fabrication/fabrication-notes.md"
req "$ROOT/fabrication/stackup.txt"
req "$ROOT/bom/BASE-bom.csv"
req "$ROOT/kicad/fp-lib-table"
req "$ROOT/kicad/sym-lib-table"
req "$ROOT/kicad/BASE.kicad_dru"
req "$ROOT/scripts/export-gerbers.sh"
req "$ROOT/scripts/generate-bom.sh"
req "$ROOT/scripts/ingest-base-pcb.sh"

echo "== honesty README =="
if grep -q "NOT FABRICABLE" "$ROOT/README.md"; then
  echo "ok: README declara NOT FABRICABLE"
else
  echo "FALTA: README deve declarar NOT FABRICABLE enquanto Claim B incompleto"
  FAIL=1
fi

echo "== BOM template =="
if head -1 "$ROOT/bom/BASE-bom.csv" | grep -q "NOT FABRICABLE\|TBD"; then
  echo "ok: BOM marca draft/TBD"
else
  echo "aviso: BOM sem marcador draft — confirme MPN reais só pós-EE"
fi

PCB="$ROOT/kicad/BASE.kicad_pcb"
if [[ -f "$PCB" ]]; then
  if grep -q "NOT FABRICABLE" "$PCB"; then
    echo "ok: $PCB ainda draft (banner presente)"
  else
    echo "aviso: $PCB sem banner — só OK se Claim B2–B5 verdes (ver docs/CHECKLIST.md)"
  fi
else
  echo "info: ainda sem BASE.kicad_pcb (esperado no scaffold) — use ingest-base-pcb.sh"
fi

if [[ "$FAIL" -ne 0 ]]; then
  echo "validate: FALHOU"
  exit 1
fi
echo "validate: OK (scaffold)"
