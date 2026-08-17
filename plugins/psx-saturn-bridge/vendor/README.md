# Vendor — psxrecomp

O clone **não** vai no git do B.A.S.E. (licença PolyForm NC + isolamento).

```bash
../scripts/setup.sh
# ou:
git clone --depth 1 --filter=blob:none --sparse \
  https://github.com/mstan/psxrecomp.git psxrecomp
cd psxrecomp && git sparse-checkout set \
  docs README.md LICENSE PRINCIPLES.md CONTRIBUTING.md \
  accuracy/README.md recompiler runtime/include tools
```

Upstream: https://github.com/mstan/psxrecomp  
Após clone, ler `docs/EXECUTION_MODEL.md` e `docs/ARCHITECTURE.md`.
