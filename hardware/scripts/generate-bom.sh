#!/usr/bin/env bash
# Gera BOM CSV a partir do esquemático KiCad (quando existir).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCH="${1:-$ROOT/kicad/BASE.kicad_sch}"
OUT="$ROOT/bom/BASE-bom-kicad.csv"

if ! command -v kicad-cli >/dev/null 2>&1; then
  echo "aviso: sem kicad-cli — mantendo template $ROOT/bom/BASE-bom.csv" >&2
  exit 0
fi

if [[ ! -f "$SCH" ]]; then
  echo "aviso: sem $SCH — template BOM permanece" >&2
  exit 0
fi

mkdir -p "$ROOT/bom"
kicad-cli sch export bom "$SCH" -o "$OUT"
echo "ok: $OUT"
echo "nota: substituir Manufacturer=TBD por MPN reais antes do Claim B5"
