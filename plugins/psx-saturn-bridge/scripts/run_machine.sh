#!/usr/bin/env bash
# Caminho MÁQUINA (default) — humano não é passo do pipeline.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

./scripts/setup.sh

echo "== machine_port (autonomous) =="
python3 tools/machine_port.py "$@"

echo "== verify_srl_apis =="
python3 tools/verify_srl_apis.py

echo "== merge_atlas (optional evidence) =="
python3 tools/merge_atlas.py || true

# Gates: máquina fechou o loop; fidelidade de jogo ainda subset
python3 - <<'PY'
import json
from pathlib import Path
proj = next(Path("out/machine_port").glob("*/MACHINE_RECEIPT.json"))
r = json.loads(proj.read_text())
assert r["human_required"] is False
assert r["machine_closed_loop"] is True
assert r["machine_emits_project"] is True
assert r["game_fidelity_complete"] is False  # honesty: subset ≠ jogo fiel
assert (Path(r["project"]) / "src" / "main.cxx").is_file()
assert (Path(r["project"]) / "makefile").is_file()
print("machine gates OK", r["project"])
PY

echo "Machine pipeline OK"
