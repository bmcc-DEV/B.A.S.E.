# Static Recompilation (Path to v1.7 + v1.8)

Vault: [`base-vault/27`](../base-vault/27%20-%20Path%20to%20v1.7/27.00%20-%20Index.md) · [`base-vault/28`](../base-vault/28%20-%20Path%20to%20v1.8/28.00%20-%20Index.md)  
Crate: `base-recomp`

## Pipeline

```text
x86-32 bytes | ELF .text → lift → SIR → emit → ASM (multi-ISA)
```

## Path v1.9 — preservação de semântica

```bash
base recomp targets            # 14 alvos (amd64 ≡ x86_64)
base recomp semantics          # catálogo semântico das 11 ISAs + JSON
base recomp verify --hex B80100000083C002C3 --target mips -o output/verify  # SIR→encode→decode→SIR′
base recomp verify --all       # preservation score por alvo (computado, não subjetivo)
base recomp encode --hex B80100000083C002C3 --target alpha -o output/enc   # bytes reais
base recomp encode --hex B80100000083C002C3 --target coldfire -o output/enc
base recomp encode --hex 90C3 --target parisc -o output/enc
```

**Preservar a semântica, não apenas executar o binário** — `base-recomp` carrega o
modelo semântico (SIR) de cada arquitetura: registradores, endianness, delay slots,
flags, ABI e quirks (fonte: `base-recomp/src/semantics.rs`).

| ISA | Encoder (`encode`) | Decoder (`verify`) |
|-----|--------------------|--------------------|
| **x86_64** | `Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec, Push, Pop` (imm32/imm8, ModRM subset) | ✅ round-trip |
| **SPARC** | `Nop, Ret, MovImm, Clear` (%l0..%l7, BE) | ✅ round-trip |
| **ARM** | `Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec` (imm8, cond AL) | ✅ round-trip |
| **AArch64** | `Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec` (W-regs, LE) | ✅ round-trip |
| **Alpha** (DEC AXP) | `Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec` | ✅ round-trip |
| **PA-RISC** (HPPA) | `Nop, Ret` | ✅ round-trip |
| **ColdFire** (68k) | `Nop, Ret, MovImm, AddImm, SubImm, Clear, Inc, Dec, Push, Pop, LdMem, StMem` | ✅ round-trip |
| **M88k** · **IA-64** · **i860** | encode pendente — emit texto + catálogo semântico | — (pendente) |
| **MIPS** · **PPC** · **SuperH** | subset | ✅ round-trip literal + semântico |

**Verifier** (`base-recomp/src/verify.rs`): `SIR ─encode→ bytes ─decode→ SIR′` e compara
duas formas — literal (`SIR == SIR′`) e semântica (`semantic_key(SIR) == semantic_key(SIR′)`,
normalizando `Clear`→`movimm(·,0)`, `Dec`→`addimm(·,-1)`, imms → i32 do domínio SIR…).
Palavra não reconhecida → `Op::Unknown` (gap) — nunca mis-decode silencioso.

**Execução semântica** (`base-recomp/src/semexec.rs`) — o salto de *representação* para
*comportamento*:

```text
execute_reference(SIR, state, width, endian)  ==  execute_isa(decode(encode(SIR)), state)
```

`MachineState { gpr, pc, flags, mem }` · width = 64 (Alpha) / 32 (demais) · memória de
64 KiB com `load`/`store` (endianness do ISA vinda do catálogo semântico, widths 1/2/4/8,
alignment não imposto — Alpha tolera unaligned) · `Push`/`Pop` executáveis via `SP = VReg 4` ·
`Flags { carry, overflow, zero, negative, extra }` estruturado (ops ainda não setam flags —
próximo rung). A dimensão `differential` roda cada op kind com imediato de borda
(`0xFFFFFFFF`) e estado de borda, e só passa se os dois lados deixam o **mesmo estado
arquitetural**. Isso já pegou um bug real: encoder ColdFire escrevia `Dn` em bits 5-3 sem
shift (`clr.l d3` viraria `0x4283` = `clr.l (a3)+`) — só `D0` passava; roundtrip antigo
usava VReg 0 e não via. E o diferencial **revelou uma inconsistência de modelo** que virou
fix: o executor de referência tratava imeds SIR como `u32` quando o domínio do SIR é `i32`
(`AddImm{0xFFFFFFFF}` ≡ `Dec`, diz `semantic_key`). O encoder Alpha (LDA) já sign-extendia
o disp; o executor somava `+4294967295` — o "gap Alpha" era isso. `semexec::execute` agora
sign-extende i32 → largura (`0xFFFFFFFF` = −1 a 64 bits); differential Alpha 50% → 67%,
sweep limpo. O round-trip literal+semântico **combinado ao diferencial** é a única forma
de provar isso — o representacional sozinho não teria visto nem o ColdFire nem o Alpha.

**Sweep gerado** (`base recomp verify --sweep --target <isa>`) — matriz de programas
(kinds × imms `{0,1,0x7f,0x80,0x7fff,0x8000,0x7fffffff,0x80000000,0xfffffffe,0xffffffff}` ×
estados iniciais `{0,1,0x7fffffff,0xffffffff}`) executada referência-vs-ISA. O espaço de
estados é onde bugs de encoding se escondem:

```text
coldfire: 268 aplicáveis · 268 match · 0 mismatches   (incl. push/pop/ld/st)
x86_64:   256 aplicáveis · 256 match · 0 mismatches   (imm32 total + push/pop)
mips:     176 aplicáveis · 176 match · 0 mismatches
alpha:    176 aplicáveis · 176 match · 0 mismatches   (após fix i32 do executor de referência)
aarch64:  136 aplicáveis · 136 match · 0 mismatches
arm:      104 aplicáveis · 104 match · 0 mismatches
sparc:     56 aplicáveis ·  56 match · 0 mismatches
```

**Cobertura por dimensão** (`base recomp verify --all`) — nunca um score único que possa
ser lido como "80% do MIPS preservado":

```text
| ISA | enc | dec | literal | semantic | exec | differential | status |
| mips     | 57% | 57% | 57% | 57% | 86% | 57% | PARTIAL |
| ppc      | 57% | 57% | 50% | 57% | 86% | 57% | PARTIAL |
| sh4      | 57% | 57% | 50% | 57% | 86% | 29% | PARTIAL |
| alpha    | 57% | 57% | 50% | 57% | 86% | 57% | PARTIAL |
| parisc   | 14% | 14% | 14% | 14% | 86% | 14% | PARTIAL |
| coldfire | 86% | 86% | 86% | 86% | 86% | 86% | PARTIAL |
| aarch64  | 57% | 57% | 57% | 57% | 86% | 29% | PARTIAL |
| arm      | 57% | 57% | 50% | 57% | 86% | 29% | PARTIAL |
| sparc    | 29% | 29% | 29% | 29% | 86% | 29% | PARTIAL |
| x86_64   | 71% | 71% | 71% | 71% | 86% | 71% | PARTIAL |
| m88k     |  0% |  0% |  0% |  0% | 86% |  0% | NONE    |
```

`literal < semantic` acontece quando o encoder normaliza (`Clear`→`mov #0` decodifica como
`MovImm{·,0}`): o significado preserva-se, a forma não. AArch64/ARM: `differential` 29%
porque os immediates de borda `0xFFFFFFFF` não cabem em MOVZ (16-bit)/ADD (12-bit) nem em
ARM imm8 — mesma limitação honesta do SH; ARM `literal < semantic` porque `Clear` vira
`MOV #0`; formas fora do subset (ARM rotate/S=1, AArch64 `lsl #12`) são gaps, nunca
mis-decode. SPARC: encoder cobre só `Nop/Ret/MovImm/Clear` (%l0..%l7); imm13 sign-extend
faz o edge `0xFFFFFFFF` (= −1) encodar — differential 29%. x86_64: imm32 total + push/pop
dedicados → 71%; faltam `call`/`jmp` (reloc precisa de linker — gap honesto) e ld/st
(encodings de memória ModRM). ColdFire: **única ISA com load/store no SIR** — `move.l
(An),Dn`/`move.l Dn,(An)` (capstone-verificados) + push/pop = 86% em todas as dimensões;
os encodings antigos de push/pop estavam errados (`0x29C0` = `move.l d0,(a4)+`) e só D0
passava — fix com capstone. Alpha alcança 57% de differential (14 kinds, incl. os 2 novos
ld/st que não encoda). `exec` mede o executor de referência (incl. push/pop/ld/st;
independente do ISA). `abi`/`privileged`/`mmu`/`system` são eixos separados, todos `0%`
(não modelados) — a tabela deixa isso explícito.

Honestidade: `static_recomp_complete: false` — o catálogo é o contrato semântico;
encode/decoder/executor por ISA são parciais até validados contra cross-as/QEMU. Fixes:
encoder PPC movido de `r0` para `r3..r31` (r0 lê como 0 no RA de `addi`); encoder
ColdFire `Dn` em bits 5-3 (`<< 3` no grupo addi/subi/clr/addq/subq/push); executor de
referência sign-extende imeds SIR (domínio i32) para a largura do ISA.

## Path v1.9

```bash
base recomp encode --hex B80100000083C002C3 --target sh2 -o output/enc   # sem cross-as
base recomp encode --hex 90C3 --target mips -o output/enc_mips
base recomp pe --input game.exe --name start --target x86_64 -o output/pe  # só .text
base recomp runtime   # Saturn/DC = false
# Capstone: cargo test -p base-recomp --features capstone
```

## Path v1.8

```bash
base recomp elf --input base-recomp/tests/fixtures/add3.o --name add3 --target sh2 -o output/v18
base recomp lift --hex 31C0C3 --target x86_64 -o output/clear  # xor eax,eax; ret
```

## Smoke (v1.7)

```bash
cargo test -p base-recomp
base recomp lift --hex 90c3 --target x86_64 -o output/recomp_smoke
base recomp roundtrip --hex B8010000000502000000C3 --name add3 --expect 3 -o output/r2
```

## Honesty

`static_recomp_complete: false` · `win32_abi_complete: false` · `runs_any_pe: false`  
Fora: Wine, PE/Win32, runtime Saturn.
