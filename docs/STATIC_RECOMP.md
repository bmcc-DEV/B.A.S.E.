# Static Recompilation (Path to v1.7 + v1.8)

Vault: [`base-vault/27`](../base-vault/27%20-%20Path%20to%20v1.7/27.00%20-%20Index.md) · [`base-vault/28`](../base-vault/28%20-%20Path%20to%20v1.8/28.00%20-%20Index.md)  
Crate: `base-recomp`

## Pipeline

```text
x86-32 bytes | ELF .text → lift → SIR → emit → ASM (multi-ISA)
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
