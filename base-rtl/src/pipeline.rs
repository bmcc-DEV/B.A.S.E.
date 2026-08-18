//! Shared pipeline components for RTL generation.

/// Generate a program counter register with reset and increment.
pub fn gen_pc_reg(name: &str, width: u32) -> String {
    format!(
        r#"    // ── Program Counter ──
    reg [{w}:0] pc;
    always @(posedge clk) begin
        if (rst)
            pc <= {w}'b0;
        else if (!stall_i)
            pc <= pc_next;
    end
    wire [{w}:0] pc_next;
    "#,
        w = width - 1
    )
}

/// Generate a GPR file with read/write ports.
pub fn gen_gpr_file(name: &str, width: u32, regs: u32, read_ports: u32) -> String {
    let mut s = format!(
        r#"    // ── General Purpose Registers ──
    reg [{w}:0] gpr [0:{n}];
    "#,
        w = width - 1,
        n = regs - 1
    );
    for i in 0..read_ports {
        s.push_str(&format!(
            "    wire [{w}:0] gpr_r{p};\n",
            w = width - 1,
            p = i
        ));
    }
    s
}

/// Generate an ALU with basic operations.
pub fn gen_alu(width: u32) -> String {
    let w = width - 1;
    format!(
        r#"    // ── ALU ──
    reg [{w}:0] alu_result;
    reg alu_zero;
    reg alu_negative;
    "#,
        w = w
    )
}

/// Generate instruction memory interface.
pub fn gen_imem(name: &str, width: u32, depth_bits: u32) -> String {
    let w = width - 1;
    let depth = 1 << depth_bits;
    format!(
        r#"    // ── Instruction Memory ──
    reg [{w}:0] imem [0:{depth}];
    wire [{w}:0] instr = imem[pc[{d}:2]];
    "#,
        w = w,
        d = depth_bits - 1
    )
}

/// Generate data memory interface.
pub fn gen_dmem(name: &str, width: u32, depth_bits: u32) -> String {
    let w = width - 1;
    let depth = 1 << depth_bits;
    format!(
        r#"    // ── Data Memory ──
    reg [{w}:0] dmem [0:{depth}];
    wire [{w}:0] dmem_rdata = dmem[dmem_addr[{d}:2]];
    reg [{w}:0] dmem_wdata;
    reg dmem_we;
    wire [{w}:0] dmem_addr;
    "#,
        w = w,
        d = depth_bits - 1
    )
}
