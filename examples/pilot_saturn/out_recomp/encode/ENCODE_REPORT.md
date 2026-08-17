# Encode (portable / WASM-friendly)

- target: `sh2`
- bytes: 6
- note: no host cross-as; SIR→machine code

## Honesty (static recomp)

- ≠ Wine / ≠ Win32 completo / ≠ runtime Saturn·DC: `static_recomp_complete: false` · `win32_abi_complete: false` · `runs_any_pe: false` · `runs_on_saturn: false` · `runs_on_dreamcast: false`
- `generates_os: false` · `auto_fix_complete: false`
- `static_recomp_complete: false` · `win32_abi_complete: false` · `runs_any_pe: false`
- `runs_on_saturn: false` · `runs_on_dreamcast: false`

## Runtime (stub)

- Saturn SH-2: `runs_on_saturn: false` — Encode SH-2 bytes only — no VDP1/VDP2 / CD / SMPC guest. Use external emulator (Mednafen/Yabause) manually.
- Dreamcast SH-4: `runs_on_dreamcast: false` — Encode SH-4 bytes only — no PowerVR / maple guest. Use external emulator (Flycast) manually.

