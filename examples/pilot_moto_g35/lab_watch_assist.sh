#!/usr/bin/env bash
# Lab assist read-only: após flash MANUAL, observa earlycon/dmesg e actualiza receipt.
# NUNCA faz flash. generates_os=false · auto_flash_complete=false
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
# Works from handoff_external/lab/ or pilot/
if [[ -f "$HERE/hw_boot_receipt.json" ]]; then
  RECEIPT="$HERE/hw_boot_receipt.json"
  LOG_DIR="$HERE"
elif [[ -f "$HERE/out_real/handoff_external/lab/hw_boot_receipt.json" ]]; then
  RECEIPT="$HERE/out_real/handoff_external/lab/hw_boot_receipt.json"
  LOG_DIR="$HERE/out_real/handoff_external/lab"
elif [[ -f "$(dirname "$HERE")/lab/hw_boot_receipt.json" ]]; then
  RECEIPT="$(dirname "$HERE")/lab/hw_boot_receipt.json"
  LOG_DIR="$(dirname "$HERE")/lab"
else
  echo "ERR: hw_boot_receipt.json not found near $HERE"
  exit 1
fi

TIMEOUT_SEC="${LAB_WATCH_SEC:-45}"
SERIAL="${ADB_SERIAL:-}"
ADB=(adb)
[[ -n "$SERIAL" ]] && ADB=(adb -s "$SERIAL")

echo "lab_watch_assist — read-only · receipt=$RECEIPT"
echo "≠ flash · generates_os=false"

if ! command -v adb >/dev/null 2>&1; then
  echo "WARN: adb missing — só actualiza receipt com method=manual_pending"
  python3 - <<PY
import json, pathlib, datetime
p=pathlib.Path("$RECEIPT")
r=json.loads(p.read_text())
r["method"]="adb_missing"
r["result"]=r.get("result") or "not_run"
r["watched_at"]=datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")
p.write_text(json.dumps(r, indent=2)+"\n")
print("receipt updated (no adb)")
PY
  exit 0
fi

STATE=$("${ADB[@]}" get-state 2>/dev/null || echo none)
echo "adb state: $STATE"

LOG="$LOG_DIR/lab_watch_${TIMEOUT_SEC}s.log"
: > "$LOG"

set +e
if [[ "$STATE" == "device" ]]; then
  # dmesg + logcat snippets
  "${ADB[@]}" shell "dmesg 2>/dev/null | tail -n 400" >>"$LOG" 2>&1
  timeout "$TIMEOUT_SEC" "${ADB[@]}" logcat -d 2>>"$LOG" | tail -n 200 >>"$LOG"
else
  echo "device not in 'device' mode — waiting ${TIMEOUT_SEC}s for adb..." | tee -a "$LOG"
  timeout "$TIMEOUT_SEC" "${ADB[@]}" wait-for-device >>"$LOG" 2>&1
  "${ADB[@]}" shell "dmesg 2>/dev/null | tail -n 400" >>"$LOG" 2>&1
fi
set -e

python3 - <<PY
import json, pathlib, re, datetime
log=pathlib.Path("$LOG").read_text(errors="replace")
patterns=[
  r"earlycon",
  r"0x20200000",
  r"sprd.*uart",
  r"ttyS0",
  r"Kernel command line",
  r"console\s*\[",
]
hits=[]
for pat in patterns:
  if re.search(pat, log, re.I):
    hits.append(pat)
result="not_run"
if hits:
  result="earlycon_seen" if any("earlycon" in h or "20200000" in h or "ttyS0" in h for h in hits) else "log_hits"
# panic/hang heuristics
if re.search(r"Kernel panic|Internal error:|Watchdog bark", log, re.I):
  result="panic"
p=pathlib.Path("$RECEIPT")
r=json.loads(p.read_text())
r["method"]="adb_watch_readonly"
r["result"]=result
r["console_log_path"]=str(pathlib.Path("$LOG").resolve())
r["watch_hits"]=hits
r["watched_at"]=datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")
r["flashed"]=bool(r.get("flashed"))  # do not auto-set true
r["generates_os"]=False
r["auto_flash_complete"]=False
p.write_text(json.dumps(r, indent=2)+"\n")
print("result:", result, "hits:", hits)
print("log:", "$LOG")
print("receipt updated — still ≠ production / ≠ OS turnkey")
PY
