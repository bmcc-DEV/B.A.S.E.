#!/usr/bin/env bash
# iMac G3 fase B — QEMU PowerPC smoke (dual target + referência). Opt-in.
# Ordem: QEMU_PPC_KERNEL (ref) → HAIKU_IMAGE → REACTOS_IMAGE
# Sem imagem → skip auditável (exit 0). ≠ OS turnkey.
set -euo pipefail
PILOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$PILOT/out_qemu"
mkdir -p "$OUT"
TIMEOUT_SEC="${QEMU_TIMEOUT_SEC:-8}"

write_receipt() {
  local name="$1" ok="$2" skipped="$3" reason="$4" kernel="${5:-}" rc="${6:--1}"
  # rc=-1 means omitted (skipped / no run)
  python3 -c "
import json
rc = int('$rc')
r = {
  'phase': 'B',
  'target': '$name',
  'ok': $ok,
  'skipped': $skipped,
  'reason': '''$reason''',
  'kernel': '''$kernel''',
  'qemu_exit': None if rc < 0 else rc,
  'production': False,
}
open('$OUT/qemu_boot_smoke_${name}.json', 'w').write(json.dumps(r, indent=2) + '\n')
print(r)
"
}

run_one() {
  local name="$1" img="$2"
  local log="$OUT/qemu_${name}.log"
  set +e
  timeout "$TIMEOUT_SEC" qemu-system-ppc \
    -M mac99 -m 256 -nographic -kernel "$img" \
    >"$log" 2>&1
  local rc=$?
  set -e
  write_receipt "$name" True False "" "$img" "$rc"
}

if ! command -v qemu-system-ppc >/dev/null 2>&1; then
  write_receipt "ref" False True "qemu-system-ppc not installed"
  write_receipt "haiku" False True "qemu-system-ppc not installed"
  write_receipt "reactos" False True "qemu-system-ppc not installed"
  echo "SKIP: qemu-system-ppc missing"
  exit 0
fi

REF="${QEMU_PPC_KERNEL:-}"
HAIKU="${HAIKU_IMAGE:-}"
REACTOS="${REACTOS_IMAGE:-}"

any=0

if [[ -n "$REF" ]]; then
  run_one "ref" "$REF"
  any=1
else
  write_receipt "ref" False True "set QEMU_PPC_KERNEL (LinuxPPC/OpenBSD reference)"
  echo "SKIP ref: no QEMU_PPC_KERNEL"
fi

if [[ -n "$HAIKU" ]]; then
  run_one "haiku" "$HAIKU"
  any=1
else
  write_receipt "haiku" False True "set HAIKU_IMAGE — see HAIKU_EXTERNAL.md"
  echo "SKIP haiku: no HAIKU_IMAGE"
fi

if [[ -n "$REACTOS" ]]; then
  run_one "reactos" "$REACTOS"
  any=1
else
  write_receipt "reactos" False True "set REACTOS_IMAGE — see REACTOS_EXTERNAL.md"
  echo "SKIP reactos: no REACTOS_IMAGE"
fi

python3 -c "
import json, pathlib
out = pathlib.Path('$OUT')
parts = {}
for name in ('ref', 'haiku', 'reactos'):
    p = out / f'qemu_boot_smoke_{name}.json'
    parts[name] = json.loads(p.read_text()) if p.exists() else None
summary = {
  'phase': 'B',
  'ok': any(p and p.get('ok') for p in parts.values()),
  'skipped': all(p and p.get('skipped') for p in parts.values() if p),
  'targets': parts,
  'production': False,
}
(out / 'qemu_boot_smoke.json').write_text(json.dumps(summary, indent=2) + '\n')
print('summary', summary['ok'], 'skipped_all', summary['skipped'])
"

if [[ "$any" -eq 0 ]]; then
  echo "SKIP: no PPC image — set QEMU_PPC_KERNEL and/or HAIKU_IMAGE / REACTOS_IMAGE"
  exit 0
fi

echo "iMac G3 fase B OK → $OUT"
