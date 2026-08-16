| ISA | level | codec | semantic | differential |
|---|---|---|---|---|
| x86_64 | P5 — Evidence-sealed | enc 71% · dec 71% | 71% | 71% |
| arm | P4 — Behavior-preserved | enc 57% · dec 57% | 57% | 29% |
| aarch64 | P4 — Behavior-preserved | enc 71% · dec 71% | 71% | 43% |
| mips | P5 — Evidence-sealed | enc 71% · dec 71% | 71% | 71% |
| ppc | P5 — Evidence-sealed | enc 71% · dec 71% | 71% | 71% |
| sparc | P3 — Semantic-preserved | enc 29% · dec 29% | 29% | 29% |
| sh2 | P4 — Behavior-preserved | enc 57% · dec 57% | 57% | 29% |
| sh4 | P4 — Behavior-preserved | enc 57% · dec 57% | 57% | 29% |
| alpha | P5 — Evidence-sealed | enc 71% · dec 71% | 71% | 71% |
| parisc | P3 — Semantic-preserved | enc 14% · dec 14% | 14% | 14% |
| m88k | P1 — Documented | enc 0% · dec — | 0% | 0% |
| ia64 | P1 — Documented | enc 0% · dec — | 0% | 0% |
| i860 | P1 — Documented | enc 0% · dec — | 0% | 0% |
| coldfire | P5 — Evidence-sealed | enc 86% · dec 86% | 86% | 86% |


## Per-ISA evidence (generated — never hand-written)
Architecture Preservation Report
================================
Target: x86_64
  (no semantic catalog entry — encode-only target x86_64)
Preservation level: P5 — Evidence-sealed

Codec:
  encoder+decoder subset: round-trip literal 71% · semantic 71%

Semantic:
  integer ops: pass (71%)

Differential:
  sweep 256/256 match · 0 mismatch(es) · differential 71%

Known gaps:
  - 10 of 12 SIR op kinds round-trip semantically
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: arm
  family: ARM (A32/A64) · word: 32 bit · endian: Little · GPRs: 16 · flags: n/z/c/v · encode: Partial("Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec")
Preservation level: P4 — Behavior-preserved

Codec:
  encoder+decoder subset: round-trip literal 50% · semantic 57%

Semantic:
  integer ops: pass (57%)

Differential:
  sweep 104/104 match · 0 mismatch(es) · differential 29%

Known gaps:
  - encoder normalizes forms (e.g. Clear → mov #0) — semantic preserved, literal not
  - 8 of 12 SIR op kinds round-trip semantically
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: aarch64
  (no semantic catalog entry — encode-only target aarch64)
Preservation level: P4 — Behavior-preserved

Codec:
  encoder+decoder subset: round-trip literal 71% · semantic 71%

Semantic:
  integer ops: pass (71%)

Differential:
  sweep 144/144 match · 0 mismatch(es) · differential 43%

Known gaps:
  - 10 of 12 SIR op kinds round-trip semantically
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: mips
  family: MIPS · word: 32 bit · endian: Big · GPRs: 32 · flags:  · encode: Partial("Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec, LdMem, StMem")
Preservation level: P5 — Evidence-sealed

Codec:
  encoder+decoder subset: round-trip literal 71% · semantic 71%

Semantic:
  integer ops: pass (71%)

Differential:
  sweep 184/184 match · 0 mismatch(es) · differential 71%

Known gaps:
  - 10 of 12 SIR op kinds round-trip semantically
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: ppc
  family: Power / PowerPC · word: 32 bit · endian: Big · GPRs: 32 · flags: cr/xer/so/ov/ca · encode: Partial("Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec, LdMem, StMem")
Preservation level: P5 — Evidence-sealed

Codec:
  encoder+decoder subset: round-trip literal 64% · semantic 71%

Semantic:
  integer ops: pass (71%)

Differential:
  sweep 184/184 match · 0 mismatch(es) · differential 71%

Known gaps:
  - encoder normalizes forms (e.g. Clear → mov #0) — semantic preserved, literal not
  - 10 of 12 SIR op kinds round-trip semantically
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: sparc
  family: Sun SPARC · word: 32 bit · endian: Big · GPRs: 32 · flags: icc/xcc: z/n/c/v · encode: Partial("Nop, Ret, Clear, MovImm")
Preservation level: P3 — Semantic-preserved

Codec:
  encoder+decoder subset: round-trip literal 29% · semantic 29%

Semantic:
  integer ops: pass (29%)

Differential:
  sweep 56/56 match · 0 mismatch(es) · differential 29%

Known gaps:
  - 4 of 12 SIR op kinds round-trip semantically
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: sh2
  family: Hitachi/Renesas SuperH (SH-1..SH-4) · word: 32 bit · endian: Little · GPRs: 16 · flags: t · encode: Partial("Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec")
Preservation level: P4 — Behavior-preserved

Codec:
  encoder+decoder subset: round-trip literal 50% · semantic 57%

Semantic:
  integer ops: pass (57%)

Differential:
  sweep 80/80 match · 0 mismatch(es) · differential 29%

Known gaps:
  - encoder normalizes forms (e.g. Clear → mov #0) — semantic preserved, literal not
  - 8 of 12 SIR op kinds round-trip semantically
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: sh4
  family: Hitachi/Renesas SuperH (SH-1..SH-4) · word: 32 bit · endian: Little · GPRs: 16 · flags: t · encode: Partial("Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec")
Preservation level: P4 — Behavior-preserved

Codec:
  encoder+decoder subset: round-trip literal 50% · semantic 57%

Semantic:
  integer ops: pass (57%)

Differential:
  sweep 80/80 match · 0 mismatch(es) · differential 29%

Known gaps:
  - encoder normalizes forms (e.g. Clear → mov #0) — semantic preserved, literal not
  - 8 of 12 SIR op kinds round-trip semantically
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: alpha
  family: DEC Alpha (AXP) · word: 64 bit · endian: Little · GPRs: 32 · flags:  · encode: Partial("Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec, LdMem, StMem")
Preservation level: P5 — Evidence-sealed

Codec:
  encoder+decoder subset: round-trip literal 64% · semantic 71%

Semantic:
  integer ops: pass (71%)

Differential:
  sweep 184/184 match · 0 mismatch(es) · differential 71%

Known gaps:
  - encoder normalizes forms (e.g. Clear → mov #0) — semantic preserved, literal not
  - 10 of 12 SIR op kinds round-trip semantically
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: parisc
  family: HP PA-RISC (HPPA) · word: 32 bit · endian: Big · GPRs: 32 · flags:  · encode: Partial("Nop, Ret")
Preservation level: P3 — Semantic-preserved

Codec:
  encoder+decoder subset: round-trip literal 14% · semantic 14%

Semantic:
  integer ops: pass (14%)

Differential:
  sweep 8/8 match · 0 mismatch(es) · differential 14%

Known gaps:
  - 2 of 12 SIR op kinds round-trip semantically
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: m88k
  family: Motorola 88000 (88100/88110) · word: 32 bit · endian: Big · GPRs: 32 · flags: z/n/c/v · encode: None("encoder pending — emit text only")
Preservation level: P1 — Documented

Codec:
  encoder only — decoder pending

Semantic:
  integer ops: not verified

Differential:
  not applicable — no decoder/encoder round-trip

Known gaps:
  - decoder pending
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: ia64
  family: Intel Itanium (EPIC) · word: 64 bit · endian: Little · GPRs: 128 · flags:  · encode: None("bundle encoder pending — emit text only")
Preservation level: P1 — Documented

Codec:
  encoder only — decoder pending

Semantic:
  integer ops: not verified

Differential:
  not applicable — no decoder/encoder round-trip

Known gaps:
  - decoder pending
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: i860
  family: Intel i860 (80860) · word: 32 bit · endian: Little · GPRs: 32 · flags: cc (integer compare)/fcc (FP compare) · encode: None("encoder pending — emit text only")
Preservation level: P1 — Documented

Codec:
  encoder only — decoder pending

Semantic:
  integer ops: not verified

Differential:
  not applicable — no decoder/encoder round-trip

Known gaps:
  - decoder pending
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)

Architecture Preservation Report
================================
Target: coldfire
  family: Motorola/Freescale ColdFire (68k family) · word: 32 bit · endian: Big · GPRs: 16 · flags: z/n/c/v/x · encode: Partial("Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec, Push, Pop, LdMem, StMem")
Preservation level: P5 — Evidence-sealed

Codec:
  encoder+decoder subset: round-trip literal 86% · semantic 86%

Semantic:
  integer ops: pass (86%)

Differential:
  sweep 268/268 match · 0 mismatch(es) · differential 86%

Known gaps:
  - 12 of 12 SIR op kinds round-trip semantically
  - abi/privileged/mmu/system not modeled (0%)

Claims:
  hardware_validated: false
  complete: false
  abi=0% · privileged=0% · mmu=0% · system=0% (not modeled — separate axes)


