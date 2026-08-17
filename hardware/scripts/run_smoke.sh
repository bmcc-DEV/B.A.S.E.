#!/usr/bin/env bash
# Smoke: base pcb → ingest → validate (sem placeholders).
# Honesty: gera engineering_draft — NOT FABRICABLE.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SPEC="${1:-hardware/fixtures/synthesized_spec.yaml}"
OUT="${PCB_OUT:-/tmp/pcb_out_base}"
PROJECT="${PROJECT:-BASE}"

if [[ ! -f "$SPEC" ]]; then
  echo "erro: falta $SPEC" >&2
  echo "Uso: $0 [caminho/synthesized_spec.yaml]" >&2
  exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT"

echo "== pcb =="
cargo run -p base-cli --quiet -- pcb "$SPEC" --project "$PROJECT" -o "$OUT"

echo "== ingest =="
./hardware/scripts/ingest-base-pcb.sh "$OUT" "$PROJECT"

echo "== validate =="
./hardware/scripts/validate-project.sh

echo "ok: draft em hardware/kicad/${PROJECT}.kicad_{sch,pcb} (NOT FABRICABLE)"
