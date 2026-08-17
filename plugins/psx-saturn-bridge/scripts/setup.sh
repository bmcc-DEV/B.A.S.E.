#!/usr/bin/env bash
# Setup isolado do plugin PSX→Saturn (psxrecomp vendor + SaturnRingLib).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VENDOR="$ROOT/vendor/psxrecomp"
SRL_DEFAULT="/media/bruno/Bruno/SaturnRingLib"
SRL_ROOT="${SRL_ROOT:-$SRL_DEFAULT}"

echo "==> plugin root: $ROOT"

if [[ ! -d "$VENDOR/.git" ]]; then
  echo "==> cloning mstan/psxrecomp (sparse)…"
  mkdir -p "$ROOT/vendor"
  git clone --depth 1 --filter=blob:none --sparse \
    https://github.com/mstan/psxrecomp.git "$VENDOR"
  (
    cd "$VENDOR"
    git sparse-checkout set docs accuracy/README.md recompiler runtime/include tools \
      README.md LICENSE PRINCIPLES.md CONTRIBUTING.md
  )
else
  echo "==> vendor/psxrecomp already present"
fi

# Evitar commits acidentais de saves/cartões
rm -f "$VENDOR"/card*.mcd "$VENDOR"/dummy*.mcr 2>/dev/null || true

if [[ -d "$SRL_ROOT" ]]; then
  ln -sfn "$SRL_ROOT" "$ROOT/SaturnRingLib"
  echo "==> SaturnRingLib → $SRL_ROOT"
else
  echo "!! SRL_ROOT não encontrado: $SRL_ROOT" >&2
  echo "   export SRL_ROOT=/caminho/para/SaturnRingLib && $0" >&2
  exit 1
fi

if [[ ! -f "$ROOT/SaturnRingLib/saturnringlib/srl.hpp" ]]; then
  echo "!! srl.hpp em falta sob SaturnRingLib" >&2
  exit 1
fi

echo "==> ok"
echo "    map:     $ROOT/mapping/psx_to_srl.yaml"
echo "    scaffold:$ROOT/templates/saturn_port_main.cxx"
echo "    docs SRL: https://srl.reye.me/"
echo "    honesty: runs_on_saturn=false"
