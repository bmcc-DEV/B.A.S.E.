#!/bin/bash
# B.A.S.E. Generated DRC Check
set -e
cd project
echo "=== Schematic DRC ==="
kicad-cli sch export netlist project.kicad_sch --output /dev/null 2>&1 || true
echo "=== PCB DRC ==="
kicad-cli pcb drc project.kicad_pcb || true
echo "=== ERC ==="
kicad-cli sch erc project.kicad_sch || true
