//! Verilog code generation primitives.

use std::fmt;

/// A generated Verilog core.
#[derive(Debug, Clone)]
pub struct VerilogCore {
    pub name: String,
    pub isa: String,
    pub width: u32,
    pub regs: u32,
    pub verilog: String,
    pub testbench: String,
}

impl VerilogCore {
    pub fn testbench(&self) -> String {
        format!(
            r#"`timescale 1ns/1ps

module tb_{name};
    reg clk;
    reg rst;
    reg [31:0] mem [0:255];
    wire [31:0] pc;
    wire [31:0] instr;
    wire stall;

    {name} dut (
        .clk(clk),
        .rst(rst),
        .pc_o(pc),
        .instr_i(instr),
        .stall_o(stall)
    );

    // Instruction memory
    assign instr = mem[pc[9:2]];

    integer i;
    initial begin
        clk = 0;
        rst = 1;
        for (i = 0; i < 256; i = i + 1) mem[i] = 32'h0000_0000;
        #10 rst = 0;
        #500;
        $display("PC=%h GPR[3]=%h", pc, dut.gpr[3]);
        if (dut.gpr[3] !== 32'h0000_0005)
            $display("FAIL: expected gpr[3]=5, got %h", dut.gpr[3]);
        else
            $display("PASS");
        $finish;
    end

    always #5 clk = ~clk;
endmodule
"#,
            name = self.name
        )
    }
}

impl fmt::Display for VerilogCore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.verilog)
    }
}
