# Pilot CASE_SUMMARY — template (stable fields)

| Check | Expected |
|-------|----------|
| Fixtures SHA256 | OK |
| Design CPU | MCU with uart (e.g. RP2040) — not TBD/ECP5 |
| Contracts | ≥70% (wedge: 3/3) |
| Check skip | NO_NEW_TRACE |
| Check dual | TIMING_VIOLATION |
| Event-graph / prove goldens | match `expected/` (`diff`, no overwrite) |
| FW host | exit 0 |

Full narrative: `base-vault/12 - Path to Real/12.20 - Pilot Case Study.md`
