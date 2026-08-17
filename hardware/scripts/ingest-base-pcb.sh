#!/usr/bin/env bash
# Copia output de `base pcb` / pipeline 06_pcb para hardware/kicad/ (fonte versionável).
# Uso:
#   ./scripts/ingest-base-pcb.sh /caminho/para/06_pcb
#   ./scripts/ingest-base-pcb.sh /caminho/para/06_pcb BASE
#   ./scripts/run_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${1:-}"
NAME="${2:-BASE}"
DEST="$ROOT/kicad"

if [[ -z "$SRC" || ! -d "$SRC" ]]; then
  cat >&2 <<EOF
Uso: $0 <dir-output-pcb> [nome-projeto]

Exemplo:
  cargo run -p base-cli -- pcb hardware/fixtures/synthesized_spec.yaml --project BASE -o /tmp/pcb_out
  $0 /tmp/pcb_out BASE

Ou: ./scripts/run_smoke.sh

O draft deve conter o banner engineering_draft — NOT FABRICABLE.
EOF
  exit 1
fi

mkdir -p "$DEST"
copied=0

copy_one() {
  local src="$1" dst="$2"
  [[ -f "$src" ]] || return 0
  cp -v "$src" "$dst"
  copied=1
}

copy_one "$SRC/${NAME}.kicad_sch" "$DEST/${NAME}.kicad_sch"
copy_one "$SRC/${NAME}.kicad_pcb" "$DEST/${NAME}.kicad_pcb"
copy_one "$SRC/bom.csv" "$DEST/bom.csv"
copy_one "$SRC/check_drc.sh" "$DEST/check_drc.sh"

if [[ ! -f "$DEST/${NAME}.kicad_sch" ]]; then
  shopt -s nullglob
  for f in "$SRC"/*.kicad_sch; do
    copy_one "$f" "$DEST/${NAME}.kicad_sch"
    break
  done
  for f in "$SRC"/*.kicad_pcb; do
    copy_one "$f" "$DEST/${NAME}.kicad_pcb"
    break
  done
  shopt -u nullglob
fi

if [[ "$copied" -eq 0 ]]; then
  echo "erro: nenhum .kicad_sch/.kicad_pcb em $SRC" >&2
  exit 1
fi

for f in "$DEST/$NAME.kicad_sch" "$DEST/$NAME.kicad_pcb"; do
  if [[ -f "$f" ]] && ! grep -q "NOT FABRICABLE" "$f"; then
    echo "aviso: $f sem banner NOT FABRICABLE — confirme se Claim B já liberou remoção" >&2
  fi
done

if [[ -f "$DEST/bom.csv" ]]; then
  cp -v "$DEST/bom.csv" "$ROOT/bom/BASE-bom-from-cli.csv"
fi

# limpar artefacto antigo do ingest duplicado
rm -f "$DEST/${NAME}.csv"

echo "ok: fontes em $DEST — próximo: EE (CHECKLIST B2–B5) ou validate-project.sh"
