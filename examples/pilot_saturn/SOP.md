# Lab SOP — Sega Saturn (fase C)

> Dumps/traces reais. Assistência de descoberta HW. ≠ jogos a correr.

## Pré-requisitos

- [ ] Fase A: `./examples/pilot_saturn/run.sh`
- [ ] Fase B: `./examples/pilot_saturn/run_recomp_smoke.sh`
- [ ] Emulador (Mednafen/Yabause) ou hardware Saturn no lab

## Checklist

- [ ] Capturar MMIO VDP1/VDP2/SMPC (NDJSON/JSON no formato do piloto)
- [ ] Substituir `mmio.json` synth → traces reais
- [ ] Re-correr fase A; diff atlas vs synth
- [ ] Documentar gaps no runtime externo (VDP/CD/áudio)

## Receipt

```json
{
  "phase": "C",
  "device": "sega_saturn",
  "runs_on_saturn": false,
  "ports_games": false,
  "production": false,
  "trace_sha256": "_______________"
}
```

## Proibido

- Claim de jogo PS1/x86 a correr no Saturn via B.A.S.E.
- `production: true`
