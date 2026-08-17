#!/usr/bin/env bash
# Exporta Gerbers / drill / pos / BOM / PDF / STEP via kicad-cli.
# Recusa release se o projeto ainda tiver banner NOT FABRICABLE (use --force-draft).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROJECT="${PROJECT:-$ROOT/kicad/BASE}"
OUT_GERBER="$ROOT/gerbers"
OUT_BOM="$ROOT/bom"
OUT_EXPORT="$ROOT/exports"
OUT_ASM="$ROOT/assembly"
FORCE_DRAFT=0

for arg in "$@"; do
  case "$arg" in
    --force-draft) FORCE_DRAFT=1 ;;
    -h|--help)
      echo "Uso: $0 [--force-draft]"
      echo "  Requer kicad-cli e $PROJECT.kicad_pcb"
      exit 0
      ;;
  esac
done

if ! command -v kicad-cli >/dev/null 2>&1; then
  echo "erro: kicad-cli não encontrado no PATH" >&2
  exit 1
fi

PCB="${PROJECT}.kicad_pcb"
SCH="${PROJECT}.kicad_sch"

if [[ ! -f "$PCB" ]]; then
  echo "erro: falta $PCB — rode scripts/ingest-base-pcb.sh ou abra o KiCad" >&2
  exit 1
fi

if grep -q "NOT FABRICABLE" "$PCB" 2>/dev/null; then
  if [[ "$FORCE_DRAFT" -ne 1 ]]; then
    echo "erro: $PCB ainda é engineering_draft (NOT FABRICABLE)." >&2
    echo "      Claim B incompleto. Para export de rascunho: $0 --force-draft" >&2
    exit 2
  fi
  echo "aviso: exportando DRAFT com banner NOT FABRICABLE (--force-draft)" >&2
fi

mkdir -p "$OUT_GERBER" "$OUT_BOM" "$OUT_EXPORT" "$OUT_ASM/3d-model"

kicad-cli pcb export gerbers "$PCB" -o "$OUT_GERBER/"
kicad-cli pcb export drill "$PCB" -o "$OUT_GERBER/"
kicad-cli pcb export pos "$PCB" -o "$OUT_ASM/BASE-top.pos" --side front || \
  kicad-cli pcb export pos "$PCB" -o "$OUT_ASM/BASE-top.pos" --side top || true
kicad-cli pcb export pos "$PCB" -o "$OUT_ASM/BASE-bottom.pos" --side back || \
  kicad-cli pcb export pos "$PCB" -o "$OUT_ASM/BASE-bottom.pos" --side bottom || true

if [[ -f "$SCH" ]]; then
  kicad-cli sch export bom "$SCH" -o "$OUT_BOM/BASE-bom-kicad.csv" || true
  kicad-cli sch export pdf "$SCH" -o "$OUT_EXPORT/schematic.pdf" || true
fi

kicad-cli pcb export pdf "$PCB" -o "$OUT_EXPORT/pcb-layers.pdf" || true
kicad-cli pcb export step "$PCB" -o "$OUT_ASM/3d-model/BASE.step" || true

echo "ok: exportação em $OUT_GERBER / $OUT_BOM / $OUT_EXPORT / $OUT_ASM"
