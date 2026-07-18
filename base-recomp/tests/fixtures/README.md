# Fixtures — base-recomp

| File | Role |
|------|------|
| `add3.s` | Source: `mov $1,%eax; add $2,%eax; ret` |
| `add3.o` | ELF64 x86_64 object (`as --64`) |

Regenerate:

```bash
as --64 -o add3.o add3.s
```
