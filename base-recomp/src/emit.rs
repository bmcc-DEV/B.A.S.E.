//! Textual ASM emitters per [`TargetIsa`] (subset of SIR ops).

use crate::sir::{Module, Op, VReg};
use crate::target::{SuperHFlavor, TargetIsa};

pub fn emit_module(module: &Module, target: TargetIsa) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "; SIR emit target={} source={} gaps={}\n; {}\n",
        target,
        module.source_isa,
        module.lift_gaps,
        crate::honesty::BANNER
    ));
    for func in &module.functions {
        out.push_str(&emit_function(&func.name, &func.blocks[0].ops, target));
        out.push('\n');
    }
    out
}

fn emit_function(name: &str, ops: &[Op], target: TargetIsa) -> String {
    let mut s = String::new();
    match target {
        TargetIsa::X86_64 => {
            s.push_str(&format!(".globl {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_x86_64(op));
            }
        }
        TargetIsa::Arm => {
            s.push_str(&format!(".global {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_arm(op));
            }
        }
        TargetIsa::AArch64 => {
            s.push_str(&format!(".global {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_aarch64(op));
            }
        }
        TargetIsa::Mips => {
            s.push_str(&format!(".globl {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_mips(op));
            }
        }
        TargetIsa::Ppc => {
            s.push_str(&format!(".globl {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_ppc(op));
            }
        }
        TargetIsa::Sparc => {
            s.push_str(&format!(".global {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_sparc(op));
            }
        }
        TargetIsa::SuperH(flavor) => {
            let comment = match flavor {
                SuperHFlavor::Sh2 => "SH-2 (Saturn class)",
                SuperHFlavor::Sh4 => "SH-4 (Dreamcast class)",
            };
            s.push_str(&format!("! {comment}\n.global {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_superh(op));
            }
        }
        TargetIsa::Alpha => {
            s.push_str(&format!(".globl {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_alpha(op));
            }
        }
        TargetIsa::PaRisc => {
            s.push_str(&format!(".global {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_parisc(op));
            }
        }
        TargetIsa::M88k => {
            s.push_str(&format!(".global {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_m88k(op));
            }
        }
        TargetIsa::Ia64 => {
            s.push_str(&format!(".global {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_ia64(op));
            }
        }
        TargetIsa::I860 => {
            s.push_str(&format!(".global {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_i860(op));
            }
        }
        TargetIsa::ColdFire => {
            s.push_str(&format!(".globl {name}\n{name}:\n"));
            for op in ops {
                s.push_str(&emit_coldfire(op));
            }
        }
    }
    s
}

fn emit_x86_64(op: &Op) -> String {
    match op {
        Op::Nop => "  nop\n".into(),
        Op::Ret => "  ret\n".into(),
        Op::MovImm { dst, imm } => format!("  mov ${imm:#x}, %{}\n", x64_reg(*dst)),
        Op::AddImm { dst, imm } => format!("  add ${imm:#x}, %{}\n", x64_reg(*dst)),
        Op::SubImm { dst, imm } => format!("  sub ${imm:#x}, %{}\n", x64_reg(*dst)),
        Op::Clear { dst } => format!("  xor %{0}, %{0}\n", x64_reg(*dst)),
        Op::Inc { dst } => format!("  inc %{}\n", x64_reg(*dst)),
        Op::Dec { dst } => format!("  dec %{}\n", x64_reg(*dst)),
        Op::Push { src } => format!("  push %{}\n", x64_reg(*src)),
        Op::Pop { dst } => format!("  pop %{}\n", x64_reg(*dst)),
        Op::LdMem { dst, .. } => format!("  # ld {} (LdMem emit TODO)\n  nop\n", x64_reg(*dst)),
        Op::StMem { src, .. } => format!("  # st {} (StMem emit TODO)\n  nop\n", x64_reg(*src)),
        Op::CallRel {
            rel,
            target,
            symbol,
        } => {
            if let Some(name) = symbol {
                format!("  call {name}\n")
            } else {
                format!(
                    "  /* call rel={rel} target={target:?} — unresolved symbol */\n  ud2\n"
                )
            }
        }
        Op::JmpRel {
            rel,
            target,
            symbol,
        } => {
            if let Some(name) = symbol {
                format!("  jmp {name}\n")
            } else {
                format!(
                    "  /* jmp rel={rel} target={target:?} — unresolved label */\n  ud2\n"
                )
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  /* gap @{offset}: {note} */\n  ud2\n")
        }
        Op::Cmp { rd, rs } => format!("  cmp %{}, %{}\n", x64_reg(*rd), x64_reg(*rs)),
        Op::Test { rd, rs } => format!("  test %{}, %{}\n", x64_reg(*rd), x64_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "e",
                crate::sir::Cond::Ne => "ne",
                crate::sir::Cond::Lt => "l",
                crate::sir::Cond::Ge => "ge",
                crate::sir::Cond::Gt => "g",
                crate::sir::Cond::Le => "le",
                crate::sir::Cond::Cs => "b",  // CF=1 (below/carry)
                crate::sir::Cond::Cc => "nb", // CF=0 (not below/no carry)
                crate::sir::Cond::Mi => "s",  // SF=1 (sign/negative)
                crate::sir::Cond::Pl => "ns", // SF=0 (no sign)
                crate::sir::Cond::Vs => "o",  // OF=1 (overflow)
                crate::sir::Cond::Vc => "no", // OF=0 (no overflow)
                crate::sir::Cond::Hi => "a",  // CF=1 && ZF=0 (above)
                crate::sir::Cond::Ls => "na", // CF=0 || ZF=1 (not above)
            };
            format!("  j{cond_str} 0x{target:x}\n")
        }
    }
}

fn x64_reg(v: VReg) -> &'static str {
    match v.0 {
        0 => "eax",
        1 => "ecx",
        2 => "edx",
        3 => "ebx",
        4 => "esp",
        5 => "ebp",
        6 => "esi",
        7 => "edi",
        _ => "eax",
    }
}

fn emit_arm(op: &Op) -> String {
    match op {
        Op::Nop => "  nop\n".into(),
        Op::Ret => "  bx lr\n".into(),
        Op::MovImm { dst, imm } => format!("  mov {}, #{imm}\n", arm_reg(*dst)),
        Op::AddImm { dst, imm } => {
            let r = arm_reg(*dst);
            format!("  add {r}, {r}, #{imm}\n")
        }
        Op::SubImm { dst, imm } => {
            let r = arm_reg(*dst);
            format!("  sub {r}, {r}, #{imm}\n")
        }
        Op::Clear { dst } => format!("  mov {}, #0\n", arm_reg(*dst)),
        Op::Inc { dst } => {
            let r = arm_reg(*dst);
            format!("  add {r}, {r}, #1\n")
        }
        Op::Dec { dst } => {
            let r = arm_reg(*dst);
            format!("  sub {r}, {r}, #1\n")
        }
        Op::Push { src } => format!("  push {{{}}}\n", arm_reg(*src)),
        Op::Pop { dst } => format!("  pop {{{}}}\n", arm_reg(*dst)),
        Op::LdMem { dst, .. } => {
            format!("  # ld {} (LdMem emit TODO)\n  nop\n", arm_reg(*dst))
        }
        Op::StMem { src, .. } => {
            format!("  # st {} (StMem emit TODO)\n  nop\n", arm_reg(*src))
        },
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  bl {name}\n")
            } else {
                format!("  @ call rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  b {name}\n")
            } else {
                format!("  @ jmp rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::Unknown { offset, note, .. } => format!("  @ gap @{offset}: {note}\n  udf #0\n"),
        Op::Cmp { rd, rs } => format!("  cmp {}, {}\n", arm_reg(*rd), arm_reg(*rs)),
        Op::Test { rd, rs } => format!("  tst {}, {}\n", arm_reg(*rd), arm_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "eq",
                crate::sir::Cond::Ne => "ne",
                crate::sir::Cond::Cs => "cs",
                crate::sir::Cond::Cc => "cc",
                crate::sir::Cond::Mi => "mi",
                crate::sir::Cond::Pl => "pl",
                crate::sir::Cond::Vs => "vs",
                crate::sir::Cond::Vc => "vc",
                crate::sir::Cond::Hi => "hi",
                crate::sir::Cond::Ls => "ls",
                crate::sir::Cond::Ge => "ge",
                crate::sir::Cond::Lt => "lt",
                crate::sir::Cond::Gt => "gt",
                crate::sir::Cond::Le => "le",
            };
            format!("  b{cond_str} 0x{target:x}\n")
        }
    }
}

fn arm_reg(v: VReg) -> String {
    format!("r{}", v.0.min(12))
}

fn emit_aarch64(op: &Op) -> String {
    match op {
        Op::Nop => "  nop\n".into(),
        Op::Ret => "  ret\n".into(),
        Op::MovImm { dst, imm } => format!("  mov {}, #{imm}\n", a64_reg(*dst)),
        Op::AddImm { dst, imm } => {
            let r = a64_reg(*dst);
            format!("  add {r}, {r}, #{imm}\n")
        }
        Op::SubImm { dst, imm } => {
            let r = a64_reg(*dst);
            format!("  sub {r}, {r}, #{imm}\n")
        }
        Op::Clear { dst } => format!("  mov {}, wzr\n", a64_reg(*dst)),
        Op::Inc { dst } => {
            let r = a64_reg(*dst);
            format!("  add {r}, {r}, #1\n")
        }
        Op::Dec { dst } => {
            let r = a64_reg(*dst);
            format!("  sub {r}, {r}, #1\n")
        }
        Op::Push { src } => format!("  str {}, [sp, #-16]!\n", a64_x(*src)),
        Op::Pop { dst } => format!("  ldr {}, [sp], #16\n", a64_x(*dst)),
        Op::LdMem { dst, .. } => format!("  # ld {} (LdMem emit TODO)\n  nop\n", a64_x(*dst)),
        Op::StMem { src, .. } => format!("  # st {} (StMem emit TODO)\n  nop\n", a64_x(*src)),
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  bl {name}\n")
            } else {
                format!("  // call rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  b {name}\n")
            } else {
                format!("  // jmp rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  // gap @{offset}: {note}\n  brk #0\n")
        }
        Op::Cmp { rd, rs } => format!("  cmp {}, {}\n", a64_reg(*rd), a64_reg(*rs)),
        Op::Test { rd, rs } => format!("  tst {}, {}\n", a64_reg(*rd), a64_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "eq",
                crate::sir::Cond::Ne => "ne",
                crate::sir::Cond::Cs => "cs",
                crate::sir::Cond::Cc => "cc",
                crate::sir::Cond::Mi => "mi",
                crate::sir::Cond::Pl => "pl",
                crate::sir::Cond::Vs => "vs",
                crate::sir::Cond::Vc => "vc",
                crate::sir::Cond::Hi => "hi",
                crate::sir::Cond::Ls => "ls",
                crate::sir::Cond::Ge => "ge",
                crate::sir::Cond::Lt => "lt",
                crate::sir::Cond::Gt => "gt",
                crate::sir::Cond::Le => "le",
            };
            format!("  b.{cond_str} 0x{target:x}\n")
        }
    }
}

fn a64_reg(v: VReg) -> String {
    format!("w{}", v.0.min(30))
}

fn a64_x(v: VReg) -> String {
    format!("x{}", v.0.min(30))
}

fn emit_mips(op: &Op) -> String {
    match op {
        Op::Nop => "  nop\n".into(),
        Op::Ret => "  jr $ra\n  nop\n".into(),
        Op::MovImm { dst, imm } => format!("  li {}, {imm}\n", mips_reg(*dst)),
        Op::AddImm { dst, imm } => {
            let r = mips_reg(*dst);
            format!("  addiu {r}, {r}, {imm}\n")
        }
        Op::SubImm { dst, imm } => {
            let r = mips_reg(*dst);
            format!("  addiu {r}, {r}, -{imm}\n")
        }
        Op::Clear { dst } => format!("  move {}, $zero\n", mips_reg(*dst)),
        Op::Inc { dst } => {
            let r = mips_reg(*dst);
            format!("  addiu {r}, {r}, 1\n")
        }
        Op::Dec { dst } => {
            let r = mips_reg(*dst);
            format!("  addiu {r}, {r}, -1\n")
        }
        Op::Push { src } => format!(
            "  addiu $sp, $sp, -4\n  sw {}, 0($sp)\n",
            mips_reg(*src)
        ),
        Op::Pop { dst } => format!(
            "  lw {}, 0($sp)\n  addiu $sp, $sp, 4\n",
            mips_reg(*dst)
        ),
        Op::LdMem { dst, .. } => format!("  # ld {} (LdMem emit TODO)\n  nop\n", mips_reg(*dst)),
        Op::StMem { src, .. } => format!("  # st {} (StMem emit TODO)\n  nop\n", mips_reg(*src)),
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  jal {name}\n")
            } else {
                format!("  # call rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  j {name}\n")
            } else {
                format!("  # jmp rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  # gap @{offset}: {note}\n  break\n")
        }
        Op::Cmp { rd, rs } => format!("  # cmp {}, {} (emit TODO)\n  nop\n", mips_reg(*rd), mips_reg(*rs)),
        Op::Test { rd, rs } => format!("  # test {}, {} (emit TODO)\n  nop\n", mips_reg(*rd), mips_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "beq",
                crate::sir::Cond::Ne => "bne",
                crate::sir::Cond::Lt => "blt",
                crate::sir::Cond::Ge => "bge",
                crate::sir::Cond::Gt => "bgt",
                crate::sir::Cond::Le => "ble",
                _ => "b",
            };
            format!("  # {cond_str} 0x{target:x} (emit TODO)\n  nop\n")
        }
    }
}

fn mips_reg(v: VReg) -> String {
    format!("$t{}", v.0.min(7))
}

fn emit_ppc(op: &Op) -> String {
    match op {
        Op::Nop => "  nop\n".into(),
        Op::Ret => "  blr\n".into(),
        Op::MovImm { dst, imm } => format!("  li {}, {imm}\n", ppc_reg(*dst)),
        Op::AddImm { dst, imm } => {
            let r = ppc_reg(*dst);
            format!("  addi {r}, {r}, {imm}\n")
        }
        Op::SubImm { dst, imm } => {
            let r = ppc_reg(*dst);
            format!("  addi {r}, {r}, -{imm}\n")
        }
        Op::Clear { dst } => format!("  li {}, 0\n", ppc_reg(*dst)),
        Op::Inc { dst } => {
            let r = ppc_reg(*dst);
            format!("  addi {r}, {r}, 1\n")
        }
        Op::Dec { dst } => {
            let r = ppc_reg(*dst);
            format!("  addi {r}, {r}, -1\n")
        }
        Op::Push { src } => format!("  stwu {}, -16(1)\n", ppc_reg(*src)),
        Op::Pop { dst } => format!("  lwz {}, 0(1)\n  addi 1, 1, 16\n", ppc_reg(*dst)),
        Op::LdMem { dst, .. } => format!("  # ld {} (LdMem emit TODO)\n  nop\n", ppc_reg(*dst)),
        Op::StMem { src, .. } => format!("  # st {} (StMem emit TODO)\n  nop\n", ppc_reg(*src)),
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  jal {name}\n")
            } else {
                format!("  # call rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  j {name}\n")
            } else {
                format!("  # jmp rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  # gap @{offset}: {note}\n  trap\n")
        }
        Op::Cmp { rd, rs } => format!("  cmpw {}, {}\n", ppc_reg(*rd), ppc_reg(*rs)),
        Op::Test { rd, rs } => format!("  # test {}, {} (emit TODO)\n  nop\n", ppc_reg(*rd), ppc_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "beq",
                crate::sir::Cond::Ne => "bne",
                crate::sir::Cond::Lt => "blt",
                crate::sir::Cond::Ge => "bge",
                crate::sir::Cond::Gt => "bgt",
                crate::sir::Cond::Le => "ble",
                crate::sir::Cond::Cs => "bcs",
                crate::sir::Cond::Cc => "bcc",
                crate::sir::Cond::Mi => "bmi",
                crate::sir::Cond::Pl => "bpl",
                crate::sir::Cond::Vs => "bvs",
                crate::sir::Cond::Vc => "bvc",
                crate::sir::Cond::Hi => "bhi",
                crate::sir::Cond::Ls => "bls",
                _ => "b",
            };
            format!("  {cond_str} 0x{target:x}\n")
        }
    }
}

fn ppc_reg(v: VReg) -> String {
    // r0 reads as 0 in addi (RA field); r1/r2 ABI-special → window r3..r31.
    format!("r{}", 3 + v.0.min(28))
}

fn emit_sparc(op: &Op) -> String {
    match op {
        Op::Nop => "  nop\n".into(),
        Op::Ret => "  retl\n  nop\n".into(),
        Op::MovImm { dst, imm } => format!("  mov {imm}, {}\n", sparc_reg(*dst)),
        Op::AddImm { dst, imm } => {
            let r = sparc_reg(*dst);
            format!("  add {r}, {imm}, {r}\n")
        }
        Op::SubImm { dst, imm } => {
            let r = sparc_reg(*dst);
            format!("  sub {r}, {imm}, {r}\n")
        }
        Op::Clear { dst } => format!("  clr {}\n", sparc_reg(*dst)),
        Op::Inc { dst } => {
            let r = sparc_reg(*dst);
            format!("  inc {r}\n")
        }
        Op::Dec { dst } => {
            let r = sparc_reg(*dst);
            format!("  dec {r}\n")
        }
        Op::Push { src } => format!("  save %sp, -96, %sp\n  mov {}, %l0\n", sparc_reg(*src)),
        Op::Pop { dst } => format!("  mov %l0, {}\n  restore\n", sparc_reg(*dst)),
        Op::LdMem { dst, .. } => format!("  # ld {} (LdMem emit TODO)\n  nop\n", sparc_reg(*dst)),
        Op::StMem { src, .. } => format!("  # st {} (StMem emit TODO)\n  nop\n", sparc_reg(*src)),
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  ! call {name}\n  nop\n")
            } else {
                format!("  ! call rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  bra {name}\n  nop\n")
            } else {
                format!("  ! jmp rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  ! gap @{offset}: {note}\n  ta 1\n")
        }
        Op::Cmp { rd, rs } => format!("  cmp {}, {}\n", sparc_reg(*rd), sparc_reg(*rs)),
        Op::Test { rd, rs } => format!("  and {}, {}, %g0\n", sparc_reg(*rd), sparc_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "be",
                crate::sir::Cond::Ne => "bne",
                crate::sir::Cond::Lt => "bl",
                crate::sir::Cond::Ge => "bge",
                crate::sir::Cond::Gt => "bg",
                crate::sir::Cond::Le => "ble",
                crate::sir::Cond::Cs => "bcs",
                crate::sir::Cond::Cc => "bcc",
                crate::sir::Cond::Mi => "bneg",
                crate::sir::Cond::Pl => "bpos",
                crate::sir::Cond::Vs => "bvs",
                crate::sir::Cond::Vc => "bvc",
                crate::sir::Cond::Hi => "bgu",
                crate::sir::Cond::Ls => "bleu",
            };
            format!("  {cond_str} 0x{target:x}\n  nop\n")
        }
    }
}

fn sparc_reg(v: VReg) -> String {
    format!("%l{}", v.0.min(7))
}

fn emit_superh(op: &Op) -> String {
    match op {
        Op::Nop => "  nop\n".into(),
        Op::Ret => "  rts\n  nop\n".into(),
        Op::MovImm { dst, imm } => {
            if *imm <= 0x7f {
                format!("  mov #{imm}, {}\n", sh_reg(*dst))
            } else {
                format!(
                    "  ! mov imm {imm:#x} -> {} (literal pool TODO)\n  nop\n",
                    sh_reg(*dst)
                )
            }
        }
        Op::AddImm { dst, imm } => {
            if *imm <= 0x7f {
                format!("  add #{imm}, {}\n", sh_reg(*dst))
            } else {
                format!(
                    "  ! add imm {imm:#x} -> {} (literal pool TODO)\n  nop\n",
                    sh_reg(*dst)
                )
            }
        }
        Op::SubImm { dst, imm } => {
            if *imm <= 0x7f {
                format!("  add #-{imm}, {}\n", sh_reg(*dst))
            } else {
                format!(
                    "  ! sub imm {imm:#x} -> {} (literal pool TODO)\n  nop\n",
                    sh_reg(*dst)
                )
            }
        }
        Op::Clear { dst } => format!("  mov #0, {}\n", sh_reg(*dst)),
        Op::Inc { dst } => format!("  add #1, {}\n", sh_reg(*dst)),
        Op::Dec { dst } => format!("  add #-1, {}\n", sh_reg(*dst)),
        Op::Push { src } => format!("  mov.l {}, @-r15\n", sh_reg(*src)),
        Op::Pop { dst } => format!("  mov.l @r15+, {}\n", sh_reg(*dst)),
        Op::LdMem { dst, .. } => format!("  # ld {} (LdMem emit TODO)\n  nop\n", sh_reg(*dst)),
        Op::StMem { src, .. } => format!("  # st {} (StMem emit TODO)\n  nop\n", sh_reg(*src)),
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  ! call {name}\n  nop\n")
            } else {
                format!("  ! call rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  bra {name}\n  nop\n")
            } else {
                format!("  ! jmp rel {rel} target={target:?}\n  nop\n")
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  ! gap @{offset}: {note}\n  trapa #0\n")
        }
        Op::Cmp { rd, rs } => format!("  cmp/eq {}, {}\n", sh_reg(*rd), sh_reg(*rs)),
        Op::Test { rd, rs } => format!("  # test {}, {} (emit TODO)\n  nop\n", sh_reg(*rd), sh_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "bt",
                crate::sir::Cond::Ne => "bf",
                crate::sir::Cond::Cs => "bt",
                crate::sir::Cond::Cc => "bf",
                _ => "bf",
            };
            format!("  {cond_str} 0x{target:x}\n  nop\n")
        }
    }
}

fn sh_reg(v: VReg) -> String {
    format!("r{}", v.0.min(15))
}

fn emit_alpha(op: &Op) -> String {
    let disp = |imm: u32| format!("{}", imm as i32);
    match op {
        Op::Nop => "  nop\n".into(),
        Op::Ret => "  ret r31, (r26)\n".into(),
        Op::MovImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            format!("  lda r{}, {}(r31)\n", alpha_reg(*dst), disp(*imm))
        }
        Op::MovImm { dst, imm } => {
            format!(
                "  # mov imm {imm:#x} -> r{} (ldah/lda TODO)\n  nop\n",
                alpha_reg(*dst)
            )
        }
        Op::AddImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            format!("  lda r{}, {}(r{})\n", alpha_reg(*dst), disp(*imm), alpha_reg(*dst))
        }
        Op::AddImm { dst, imm } => {
            format!(
                "  # add imm {imm:#x} -> r{} (ldah/lda TODO)\n  nop\n",
                alpha_reg(*dst)
            )
        }
        Op::SubImm { dst, imm } if (*imm as i32) >= -32768 && (*imm as i32) <= 32767 => {
            format!(
                "  lda r{}, -{}(r{})\n",
                alpha_reg(*dst),
                -(*imm as i32),
                alpha_reg(*dst)
            )
        }
        Op::SubImm { dst, imm } => {
            format!(
                "  # sub imm {imm:#x} -> r{} (ldah/lda TODO)\n  nop\n",
                alpha_reg(*dst)
            )
        }
        Op::Clear { dst } => format!("  lda r{}, 0(r31)\n", alpha_reg(*dst)),
        Op::Inc { dst } => format!("  lda r{}, 1(r{})\n", alpha_reg(*dst), alpha_reg(*dst)),
        Op::Dec { dst } => format!("  lda r{}, -1(r{})\n", alpha_reg(*dst), alpha_reg(*dst)),
        Op::Push { src } => format!("  lda sp, -8(sp)\n  stq {}, 0(sp)\n", alpha_reg(*src)),
        Op::Pop { dst } => format!("  ldq {}, 0(sp)\n  lda sp, 8(sp)\n", alpha_reg(*dst)),
        Op::LdMem { dst, .. } => format!("  # ld {} (LdMem emit TODO)\n  nop\n", alpha_reg(*dst)),
        Op::StMem { src, .. } => format!("  # st {} (StMem emit TODO)\n  nop\n", alpha_reg(*src)),
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  jsr r26, {name}\n")
            } else {
                format!("  # call rel={rel} target={target:?} — unresolved symbol\n  nop\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  br r31, {name}\n")
            } else {
                format!("  # jmp rel={rel} target={target:?} — unresolved label\n  nop\n")
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  # gap @{offset}: {note}\n  call_pal 1\n")
        }
        Op::Cmp { rd, rs } => format!("  cmplt {}, {}, r31\n", alpha_reg(*rd), alpha_reg(*rs)),
        Op::Test { rd, rs } => format!("  # test {}, {} (emit TODO)\n  nop\n", alpha_reg(*rd), alpha_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "beq",
                crate::sir::Cond::Ne => "bne",
                crate::sir::Cond::Lt => "blt",
                crate::sir::Cond::Ge => "bge",
                crate::sir::Cond::Gt => "bgt",
                crate::sir::Cond::Le => "ble",
                _ => "br",
            };
            format!("  {cond_str} r31, 0x{target:x}\n")
        }
    }
}

fn alpha_reg(v: VReg) -> String {
    format!("r{}", v.0.min(31))
}

fn emit_parisc(op: &Op) -> String {
    match op {
        Op::Nop => "  nop\n".into(),
        Op::Ret => "  bv %r0(%rp)\n  nop\n".into(),
        Op::MovImm { dst, imm } if (*imm as i32) >= -8192 && (*imm as i32) <= 8191 => {
            format!("  ldo {}(%r0), {}\n", *imm as i32, parisc_reg(*dst))
        }
        Op::MovImm { dst, imm } => {
            format!(
                "  # mov imm {imm:#x} -> {} (ldil/ldo TODO)\n  nop\n",
                parisc_reg(*dst)
            )
        }
        Op::AddImm { dst, imm } if (*imm as i32) >= -8192 && (*imm as i32) <= 8191 => {
            format!(
                "  ldo {}({}), {}\n",
                *imm as i32,
                parisc_reg(*dst),
                parisc_reg(*dst)
            )
        }
        Op::AddImm { dst, imm } => {
            format!(
                "  # add imm {imm:#x} -> {} (ldil/ldo TODO)\n  nop\n",
                parisc_reg(*dst)
            )
        }
        Op::SubImm { dst, imm } if (*imm as i32) >= -8192 && (*imm as i32) <= 8191 => {
            format!(
                "  ldo -{}({}), {}\n",
                *imm as i32,
                parisc_reg(*dst),
                parisc_reg(*dst)
            )
        }
        Op::SubImm { dst, imm } => {
            format!(
                "  # sub imm {imm:#x} -> {} (ldil/ldo TODO)\n  nop\n",
                parisc_reg(*dst)
            )
        }
        Op::Clear { dst } => format!("  ldo 0(%r0), {}\n", parisc_reg(*dst)),
        Op::Inc { dst } => format!("  ldo 1({}), {}\n", parisc_reg(*dst), parisc_reg(*dst)),
        Op::Dec { dst } => format!("  ldo -1({}), {}\n", parisc_reg(*dst), parisc_reg(*dst)),
        Op::Push { src } => {
            format!("  # push {} (stwm/ldwm TODO)\n  nop\n", parisc_reg(*src))
        }
        Op::Pop { dst } => {
            format!("  # pop {} (stwm/ldwm TODO)\n  nop\n", parisc_reg(*dst))
        }
        Op::LdMem { dst, .. } => format!("  # ld {} (LdMem emit TODO)\n  nop\n", parisc_reg(*dst)),
        Op::StMem { src, .. } => format!("  # st {} (StMem emit TODO)\n  nop\n", parisc_reg(*src)),
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  bl {name}, %rp\n")
            } else {
                format!("  # call rel={rel} target={target:?} — unresolved symbol\n  nop\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  b {name}\n  nop\n")
            } else {
                format!("  # jmp rel={rel} target={target:?} — unresolved label\n  nop\n")
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  # gap @{offset}: {note}\n  break 0,0\n")
        }
        Op::Cmp { rd, rs } => format!("  cmpib,= {}, {}, 0\n", parisc_reg(*rd), parisc_reg(*rs)),
        Op::Test { rd, rs } => format!("  # test {}, {} (emit TODO)\n  nop\n", parisc_reg(*rd), parisc_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "b",
                crate::sir::Cond::Ne => "b",
                crate::sir::Cond::Lt => "b",
                crate::sir::Cond::Ge => "b",
                crate::sir::Cond::Gt => "b",
                crate::sir::Cond::Le => "b",
                _ => "b",
            };
            format!("  {cond_str} 0x{target:x}\n  nop\n")
        }
    }
}

fn parisc_reg(v: VReg) -> String {
    // avoid %r0 (zero) · %r1 (assembler temp) · %r2 (rp) — VReg window starts at r3.
    format!("%r{}", 3 + v.0.min(28))
}

fn emit_m88k(op: &Op) -> String {
    match op {
        Op::Nop => "  or r0, r0, r0\n".into(),
        Op::Ret => "  jmp r1\n  nop\n".into(),
        Op::MovImm { dst, imm } if *imm <= 0xffff => {
            format!("  or {}, r0, {imm}\n", m88k_reg(*dst))
        }
        Op::MovImm { dst, imm } => {
            let hi = imm >> 16;
            let lo = imm & 0xffff;
            format!("  or.u {}, r0, {hi:#x}\n  or {}, {}, {lo:#x}\n", m88k_reg(*dst), m88k_reg(*dst), m88k_reg(*dst))
        }
        Op::AddImm { dst, imm } if *imm <= 0xffff => {
            format!("  addu {}, {}, {imm}\n", m88k_reg(*dst), m88k_reg(*dst))
        }
        Op::AddImm { dst, imm } => {
            format!(
                "  # add imm {imm:#x} -> {} (addu imm TODO)\n  nop\n",
                m88k_reg(*dst)
            )
        }
        Op::SubImm { dst, imm } if *imm <= 0xffff => {
            format!("  subu {}, {}, {imm}\n", m88k_reg(*dst), m88k_reg(*dst))
        }
        Op::SubImm { dst, imm } => {
            format!(
                "  # sub imm {imm:#x} -> {} (subu imm TODO)\n  nop\n",
                m88k_reg(*dst)
            )
        }
        Op::Clear { dst } => format!("  or {}, r0, 0\n", m88k_reg(*dst)),
        Op::Inc { dst } => format!("  addu {}, {}, 1\n", m88k_reg(*dst), m88k_reg(*dst)),
        Op::Dec { dst } => format!("  subu {}, {}, 1\n", m88k_reg(*dst), m88k_reg(*dst)),
        Op::Push { src } => format!("  subu r31, r31, 4\n  st {}, r31, r0\n", m88k_reg(*src)),
        Op::Pop { dst } => format!("  ld {}, r31, r0\n  addu r31, r31, 4\n", m88k_reg(*dst)),
        Op::LdMem { dst, .. } => format!("  # ld {} (LdMem emit TODO)\n  nop\n", m88k_reg(*dst)),
        Op::StMem { src, .. } => format!("  # st {} (StMem emit TODO)\n  nop\n", m88k_reg(*src)),
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  bsr {name}\n  nop\n")
            } else {
                format!("  # call rel={rel} target={target:?} — unresolved symbol\n  nop\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  br {name}\n  nop\n")
            } else {
                format!("  # jmp rel={rel} target={target:?} — unresolved label\n  nop\n")
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  # gap @{offset}: {note}\n  trap 1\n")
        }
        Op::Cmp { rd, rs } => format!("  cmp.eq {}, {}\n", m88k_reg(*rd), m88k_reg(*rs)),
        Op::Test { rd, rs } => format!("  # test {}, {} (emit TODO)\n  nop\n", m88k_reg(*rd), m88k_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "br.eq",
                crate::sir::Cond::Ne => "br.ne",
                crate::sir::Cond::Lt => "br.lt",
                crate::sir::Cond::Ge => "br.ge",
                crate::sir::Cond::Gt => "br.gt",
                crate::sir::Cond::Le => "br.le",
                _ => "br",
            };
            format!("  {cond_str} 0x{target:x}\n  nop\n")
        }
    }
}

fn m88k_reg(v: VReg) -> String {
    // avoid r0 (zero) · r1 (link) — VReg window starts at r2.
    format!("r{}", 2 + v.0.min(29))
}

fn emit_ia64(op: &Op) -> String {
    match op {
        Op::Nop => "  nop.m 0\n".into(),
        Op::Ret => "  br.ret.sptk.many b0\n".into(),
        Op::MovImm { dst, imm } if (*imm as i32) >= -2097152 && (*imm as i32) <= 2097151 => {
            format!("  mov r{}, = {}\n", ia64_reg(*dst), *imm as i32)
        }
        Op::MovImm { dst, imm } => {
            format!(
                "  // mov imm {imm:#x} -> r{} (movl TODO)\n  nop.m 0\n",
                ia64_reg(*dst)
            )
        }
        Op::AddImm { dst, imm } if (*imm as i32) >= -2097152 && (*imm as i32) <= 2097151 => {
            format!("  adds r{}, = {}, r{}\n", ia64_reg(*dst), *imm as i32, ia64_reg(*dst))
        }
        Op::AddImm { dst, imm } => {
            format!(
                "  // add imm {imm:#x} -> r{} (movl/adds TODO)\n  nop.m 0\n",
                ia64_reg(*dst)
            )
        }
        Op::SubImm { dst, imm } if (*imm as i32) >= -2097152 && (*imm as i32) <= 2097151 => {
            format!(
                "  adds r{}, = -{}, r{}\n",
                ia64_reg(*dst),
                *imm as i32,
                ia64_reg(*dst)
            )
        }
        Op::SubImm { dst, imm } => {
            format!(
                "  // sub imm {imm:#x} -> r{} (movl/adds TODO)\n  nop.m 0\n",
                ia64_reg(*dst)
            )
        }
        Op::Clear { dst } => format!("  mov r{}, = r0\n", ia64_reg(*dst)),
        Op::Inc { dst } => format!("  adds r{}, = 1, r{}\n", ia64_reg(*dst), ia64_reg(*dst)),
        Op::Dec { dst } => format!("  adds r{}, = -1, r{}\n", ia64_reg(*dst), ia64_reg(*dst)),
        Op::Push { src } => {
            format!("  adds r12 = -16, r12\n  st8 [r12] = r{}\n", ia64_reg(*src))
        }
        Op::Pop { dst } => {
            format!("  ld8 r{} = [r12]\n  adds r12 = 16, r12\n", ia64_reg(*dst))
        }
        Op::LdMem { dst, .. } => format!("  // ld r{} (LdMem emit TODO)\n  nop.m 0\n", ia64_reg(*dst)),
        Op::StMem { src, .. } => format!("  // st r{} (StMem emit TODO)\n  nop.m 0\n", ia64_reg(*src)),
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  br.call.sptk.many b0 = {name}\n")
            } else {
                format!("  // call rel={rel} target={target:?} — unresolved symbol\n  nop.m 0\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  br.sptk.many {name}\n")
            } else {
                format!("  // jmp rel={rel} target={target:?} — unresolved label\n  nop.m 0\n")
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  // gap @{offset}: {note}\n  break.i 0\n")
        }
        Op::Cmp { rd, rs } => format!("  cmp.eq p{}, p{}, r{}, r{}\n", ia64_reg(*rd), ia64_reg(*rs), ia64_reg(*rd), ia64_reg(*rs)),
        Op::Test { rd, rs } => format!("  // test {}, {} (emit TODO)\n  nop.m 0\n", ia64_reg(*rd), ia64_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "br.cond",
                crate::sir::Cond::Ne => "br.cond",
                _ => "br.cond",
            };
            format!("  {cond_str} 0x{target:x}\n")
        }
    }
}

fn ia64_reg(v: VReg) -> String {
    // avoid r0 (zero) · r1 (gp) — VReg window starts at r4.
    format!("r{}", 4 + v.0.min(122))
}

fn emit_i860(op: &Op) -> String {
    match op {
        Op::Nop => "  nop\n".into(),
        Op::Ret => "  br r1\n  nop\n".into(),
        Op::MovImm { dst, imm } => format!(
            "  # mov imm {imm:#x} -> r{} (orh/or TODO)\n  nop\n",
            i860_reg(*dst)
        ),
        Op::AddImm { dst, imm } => format!(
            "  # add imm {imm:#x} -> r{} (add TODO)\n  nop\n",
            i860_reg(*dst)
        ),
        Op::SubImm { dst, imm } => format!(
            "  # sub imm {imm:#x} -> r{} (sub TODO)\n  nop\n",
            i860_reg(*dst)
        ),
        Op::Clear { dst } => format!("  # clear r{} (xor r0 TODO)\n  nop\n", i860_reg(*dst)),
        Op::Inc { dst } => format!("  # inc r{} (add 1 TODO)\n  nop\n", i860_reg(*dst)),
        Op::Dec { dst } => format!("  # dec r{} (sub 1 TODO)\n  nop\n", i860_reg(*dst)),
        Op::Push { src } => format!("  # push r{} (st TODO)\n  nop\n", i860_reg(*src)),
        Op::Pop { dst } => format!("  # pop r{} (ld TODO)\n  nop\n", i860_reg(*dst)),
        Op::LdMem { dst, .. } => format!("  # ld {} (LdMem emit TODO)\n  nop\n", i860_reg(*dst)),
        Op::StMem { src, .. } => format!("  # st {} (StMem emit TODO)\n  nop\n", i860_reg(*src)),
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  # call {name}\n  nop\n")
            } else {
                format!("  # call rel={rel} target={target:?} — unresolved symbol\n  nop\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  # jmp {name}\n  nop\n")
            } else {
                format!("  # jmp rel={rel} target={target:?} — unresolved label\n  nop\n")
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  # gap @{offset}: {note}\n  nop\n")
        }
        Op::Cmp { rd, rs } => format!("  # cmp {}, {} (emit TODO)\n  nop\n", i860_reg(*rd), i860_reg(*rs)),
        Op::Test { rd, rs } => format!("  # test {}, {} (emit TODO)\n  nop\n", i860_reg(*rd), i860_reg(*rs)),
        Op::BranchCond { cond, target } => format!("  # b.{} 0x{target:x} (emit TODO)\n  nop\n", format!("{:?}", cond).to_lowercase()),
    }
}

fn i860_reg(v: VReg) -> String {
    format!("r{}", v.0.min(31))
}

fn emit_coldfire(op: &Op) -> String {
    match op {
        Op::Nop => "  nop\n".into(),
        Op::Ret => "  rts\n".into(),
        Op::MovImm { dst, imm } => {
            if (*imm as i32) >= -128 && (*imm as i32) <= 127 {
                format!("  moveq #{}, {}\n", *imm as i32, coldfire_reg(*dst))
            } else {
                format!("  move.l #{imm}, {}\n", coldfire_reg(*dst))
            }
        }
        Op::AddImm { dst, imm } => format!("  addi.l #{imm}, {}\n", coldfire_reg(*dst)),
        Op::SubImm { dst, imm } => format!("  subi.l #{imm}, {}\n", coldfire_reg(*dst)),
        Op::Clear { dst } => format!("  clr.l {}\n", coldfire_reg(*dst)),
        Op::Inc { dst } => format!("  addq.l #1, {}\n", coldfire_reg(*dst)),
        Op::Dec { dst } => format!("  subq.l #1, {}\n", coldfire_reg(*dst)),
        Op::Push { src } => format!("  move.l {}, -(A7)\n", coldfire_reg(*src)),
        Op::Pop { dst } => format!("  move.l (A7)+, {}\n", coldfire_reg(*dst)),
        Op::LdMem { dst, .. } => format!("  # ld {} (LdMem emit TODO)\n  nop\n", coldfire_reg(*dst)),
        Op::StMem { src, .. } => format!("  # st {} (StMem emit TODO)\n  nop\n", coldfire_reg(*src)),
        Op::CallRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  jsr {name}\n")
            } else {
                format!("  | call rel={rel} target={target:?} — unresolved symbol\n  illegal\n")
            }
        }
        Op::JmpRel { rel, target, symbol } => {
            if let Some(name) = symbol {
                format!("  jmp {name}\n")
            } else {
                format!("  | jmp rel={rel} target={target:?} — unresolved label\n  illegal\n")
            }
        }
        Op::Unknown { offset, note, .. } => {
            format!("  | gap @{offset}: {note}\n  illegal\n")
        }
        Op::Cmp { rd, rs } => format!("  cmp.l {}, {}\n", coldfire_reg(*rd), coldfire_reg(*rs)),
        Op::Test { rd, rs } => format!("  # test {}, {} (emit TODO)\n  nop\n", coldfire_reg(*rd), coldfire_reg(*rs)),
        Op::BranchCond { cond, target } => {
            let cond_str = match cond {
                crate::sir::Cond::Eq => "beq",
                crate::sir::Cond::Ne => "bne",
                crate::sir::Cond::Lt => "blt",
                crate::sir::Cond::Ge => "bge",
                crate::sir::Cond::Gt => "bgt",
                crate::sir::Cond::Le => "ble",
                crate::sir::Cond::Cs => "bcs",
                crate::sir::Cond::Cc => "bcc",
                crate::sir::Cond::Mi => "bmi",
                crate::sir::Cond::Pl => "bpl",
                crate::sir::Cond::Vs => "bvs",
                crate::sir::Cond::Vc => "bvc",
                crate::sir::Cond::Hi => "bhi",
                crate::sir::Cond::Ls => "bls",
                _ => "bra",
            };
            format!("  {cond_str} 0x{target:x}\n")
        }
    }
}

fn coldfire_reg(v: VReg) -> String {
    format!("D{}", v.0.min(7))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lift::lift_x86_32;

    #[test]
    fn emit_all_targets_nop_ret() {
        let m = lift_x86_32(&[0x90, 0xC3], "demo").unwrap();
        for t in TargetIsa::all_canonical() {
            let asm = emit_module(&m, *t);
            assert!(asm.contains("demo"), "missing label for {t}");
            assert!(
                asm.contains("static_recomp_complete"),
                "honesty banner missing for {t}"
            );
        }
    }

    #[test]
    fn emit_clear_x86_64() {
        let m = lift_x86_32(&[0x31, 0xC0, 0xC3], "z").unwrap();
        let a = emit_module(&m, TargetIsa::X86_64);
        assert!(a.contains("xor %eax, %eax"));
    }

    #[test]
    fn emit_preservation_isas_mnemonics() {
        let m = lift_x86_32(&[0x90, 0xC3], "demo").unwrap();
        let checks = [
            (TargetIsa::Alpha, "ret r31, (r26)"),
            (TargetIsa::PaRisc, "bv %r0(%rp)"),
            (TargetIsa::M88k, "jmp r1"),
            (TargetIsa::Ia64, "br.ret.sptk.many b0"),
            (TargetIsa::I860, "br r1"),
            (TargetIsa::ColdFire, "rts"),
        ];
        for (isa, needle) in checks {
            let asm = emit_module(&m, isa);
            assert!(asm.contains(needle), "missing `{needle}` for {isa}");
        }
    }

    #[test]
    fn emit_coldfire_add3() {
        let m = lift_x86_32(&[0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3], "add3")
            .unwrap();
        let asm = emit_module(&m, TargetIsa::ColdFire);
        assert!(asm.contains("moveq #1, D0"));
        assert!(asm.contains("addi.l #2, D0"));
        assert!(asm.contains("rts"));
    }
}
