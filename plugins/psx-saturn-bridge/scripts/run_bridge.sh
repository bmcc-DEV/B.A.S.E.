#!/usr/bin/env bash
# Pipeline determinística do bridge — zero IA.
# emit YAML→stubs · verify SRL APIs · merge atlas B.A.S.E.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

./scripts/setup.sh

echo "== emit_from_map =="
python3 tools/emit_from_map.py --scaffold

echo "== verify_srl_apis =="
python3 tools/verify_srl_apis.py

echo "== merge_atlas =="
python3 tools/merge_atlas.py

# Honesty gate
python3 - <<'PY'
import json
from pathlib import Path
m = json.loads(Path("out/generated/subsystem_matrix.json").read_text())
h = m.get("honesty") or {}
assert h.get("runs_on_saturn") is False
assert h.get("auto_ports_games") is False
print("honesty OK")
PY

echo "Bridge pipeline OK → $ROOT/out/generated"
ls -la out/generated
