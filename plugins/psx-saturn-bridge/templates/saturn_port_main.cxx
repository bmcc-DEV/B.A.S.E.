// Scaffold de port PS1 → Saturn via Saturn Ring Library
// Plugin: plugins/psx-saturn-bridge — ≠ jogo completo · runs_on_saturn=false no B.A.S.E.
//
// Build: makefile com SRL_INSTALL_ROOT apontando ao SaturnRingLib
// Docs: https://srl.reye.me/ · mapping/psx_to_srl.yaml

#include <srl.hpp>

using namespace SRL::Types;
using namespace SRL::Math::Types;
using namespace SRL::Input;

/** Mapa típico PS1 DualShock → Saturn Digital (ajustar por jogo). */
struct PsxPadMap
{
    static bool Cross(const Digital& p) { return p.IsHeld(Digital::Button::A); }
    static bool Circle(const Digital& p) { return p.IsHeld(Digital::Button::B); }
    static bool Triangle(const Digital& p) { return p.IsHeld(Digital::Button::C); }
    static bool Square(const Digital& p) { return p.IsHeld(Digital::Button::X); }
    static bool L1(const Digital& p) { return p.IsHeld(Digital::Button::L); }
    static bool R1(const Digital& p) { return p.IsHeld(Digital::Button::R); }
    static bool Start(const Digital& p) { return p.IsHeld(Digital::Button::START); }
};

int main()
{
    SRL::Core::Initialize(HighColor::Colors::Black);
    SRL::Debug::Print(1, 1, "psx-saturn-bridge scaffold");
    SRL::Debug::Print(1, 2, "Replace GPU/GTE/SPU with SRL::*");

    Digital pad(0);

    while (1)
    {
        if (pad.IsConnected())
        {
            if (PsxPadMap::Start(pad))
            {
                SRL::Debug::Print(1, 4, "START (PS1 Start)");
            }
            if (PsxPadMap::Cross(pad))
            {
                SRL::Debug::Print(1, 5, "A    (PS1 Cross)");
            }
        }

        // TODO: port draw path → SRL::Scene2D / VDP1 / VDP2
        // TODO: port assets → SRL::Cd::File
        SRL::Core::Synchronize();
    }

    return 0;
}
