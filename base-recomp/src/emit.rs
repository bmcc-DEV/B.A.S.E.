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
        Op::CallRel { rel, target } => {
            // Unresolved external — do not emit fake absolute `call imm` (invalid gas).
            format!(
                "  /* call rel={rel} target={target:?} — unresolved symbol */\n  ud2\n"
            )
        }
        Op::JmpRel { rel, target } => {
            format!(
                "  /* jmp rel={rel} target={target:?} — unresolved label */\n  ud2\n"
            )
        }
        Op::Unknown { offset, note, .. } => {
            format!("  /* gap @{offset}: {note} */\n  ud2\n")
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
        Op::CallRel { rel, target } => {
            format!("  @ call rel {rel} target={target:?}\n  nop\n")
        }
        Op::JmpRel { rel, target } => {
            format!("  @ jmp rel {rel} target={target:?}\n  nop\n")
        }
        Op::Unknown { offset, note, .. } => format!("  @ gap @{offset}: {note}\n  udf #0\n"),
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
        Op::CallRel { rel, target } => {
            format!("  // call rel {rel} target={target:?}\n  nop\n")
        }
        Op::JmpRel { rel, target } => {
            format!("  // jmp rel {rel} target={target:?}\n  nop\n")
        }
        Op::Unknown { offset, note, .. } => {
            format!("  // gap @{offset}: {note}\n  brk #0\n")
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
        Op::CallRel { rel, target } => {
            format!("  # call rel {rel} target={target:?}\n  nop\n")
        }
        Op::JmpRel { rel, target } => {
            format!("  # jmp rel {rel} target={target:?}\n  nop\n")
        }
        Op::Unknown { offset, note, .. } => {
            format!("  # gap @{offset}: {note}\n  break\n")
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
        Op::CallRel { rel, target } => {
            format!("  # call rel {rel} target={target:?}\n  nop\n")
        }
        Op::JmpRel { rel, target } => {
            format!("  # jmp rel {rel} target={target:?}\n  nop\n")
        }
        Op::Unknown { offset, note, .. } => {
            format!("  # gap @{offset}: {note}\n  trap\n")
        }
    }
}

fn ppc_reg(v: VReg) -> String {
    format!("r{}", v.0.min(31))
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
        Op::CallRel { rel, target } => {
            format!("  ! call rel {rel} target={target:?}\n  nop\n")
        }
        Op::JmpRel { rel, target } => {
            format!("  ! jmp rel {rel} target={target:?}\n  nop\n")
        }
        Op::Unknown { offset, note, .. } => {
            format!("  ! gap @{offset}: {note}\n  ta 1\n")
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
        Op::CallRel { rel, target } => {
            format!("  ! call rel {rel} target={target:?}\n  nop\n")
        }
        Op::JmpRel { rel, target } => {
            format!("  ! jmp rel {rel} target={target:?}\n  nop\n")
        }
        Op::Unknown { offset, note, .. } => {
            format!("  ! gap @{offset}: {note}\n  trapa #0\n")
        }
    }
}

fn sh_reg(v: VReg) -> String {
    format!("r{}", v.0.min(15))
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
}
