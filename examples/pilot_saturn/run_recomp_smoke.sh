#!/usr/bin/env bash
# Fase B — liga discovery ao pedaço CPU SH-2 do base-recomp.
# Encode subset only. ≠ VDP · ≠ jogo · runs_on_saturn=false
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$PILOT/out_recomp"
cd "$ROOT"

cargo build -p base-cli -q
BASE="$ROOT/target/debug/base"
rm -rf "$OUT"
mkdir -p "$OUT"

"$BASE" recomp lift --hex 31C0C3 --name saturn_cpu_smoke --target sh2 -o "$OUT/lift"
"$BASE" recomp encode --hex 31C0C3 --name saturn_cpu_smoke --target sh2 -o "$OUT/encode"
"$BASE" recomp runtime > "$OUT/runtime_stub.md"

python3 - <<PY
import json
from pathlib import Path
out = Path("$OUT")
r = {
  "phase": "B",
  "ok": True,
  "target": "sh2",
  "runs_on_saturn": False,
  "ports_games": False,
  "note": "CPU encode only — no VDP runtime; Mednafen/Yabause manual",
  "production": False,
}
(out / "recomp_smoke.json").write_text(json.dumps(r, indent=2) + "\n")
print(r)
PY

echo "Pilot Saturn fase B (recomp SH-2 smoke) OK → $OUT"
