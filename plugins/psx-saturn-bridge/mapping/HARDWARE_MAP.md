# Hardware map — PS1 → Saturn (via SRL)

Referência rápida para port humano. Detalhe machine-readable: [`psx_to_srl.yaml`](psx_to_srl.yaml).

| PS1 | psxrecomp (conceito) | Saturn | SaturnRingLib |
|-----|----------------------|--------|---------------|
| R3000A | MIPS→C codegen | SH-2 ×2 | código C++ + SGL |
| GTE | inline GTE ops | CPU math | `SRL::Math`, `Scene3D` |
| GPU | MMIO + renderer | VDP1 + VDP2 | `SRL::VDP1`, `VDP2`, `Scene2D` |
| SPU | SPU core | SCSP | `SRL::Sound::*` |
| CD-ROM / XA | cdrom + iso | GFS / CD | `SRL::Cd` |
| SIO0 pad | sio / input.ini | SMPC | `SRL::Input::Digital` |
| MDEC | mdec | Cinepak / custom | `SRL::Cinepak` |
| BIOS LLE | recompiled SCPH1001 | SGL bootstrap | `SRL::Core::Initialize` |

## Loop mínimo SRL (destino)

```cpp
SRL::Core::Initialize(SRL::Types::HighColor::Colors::Black);
SRL::Input::Digital pad(0);
while (1) {
    // desenhar / lógica portada
    SRL::Core::Synchronize();
}
```

## O que reaproveitar do psxrecomp

- Modelo de execução: static first, overlays como inventário ([`docs/EXECUTION_MODEL.md`](../vendor/psxrecomp/docs/EXECUTION_MODEL.md)).
- Princípios de debug: first divergence ([`PRINCIPLES.md`](../vendor/psxrecomp/PRINCIPLES.md)).
- **Não** o runtime C/SDL nem o BIOS recompilado.

## O que vem do B.A.S.E.

Atlas MMIO Saturn: `examples/pilot_saturn/out/port_package/` após `./run.sh`.
