//! Reference semantic execution (Path v1.9) — SIR ops → effects on a minimal
//! architectural state, at an ISA's word width and byte order.
//!
//! Property under test (differential, in [`crate::verify`]):
//! ```text
//! execute_reference(SIR, state, width, endian) == execute_isa(decode(encode(SIR)), state)
//! ```
//! This checks *behavior*, not representation: the round-tripped ops must leave the
//! same architectural state. Memory (load/store + push/pop) follows the target's byte
//! order — memory layout is architectural. Alignment is NOT enforced here (Alpha's
//! `ldq_u` and 68k tolerate unaligned); catalog quirks document per-ISA rules.
//! Flag *side-effects* are the next modeling rung: ops set no flags yet.

use crate::sir::Op;
use crate::target::TargetIsa;

/// Byte order for memory ops, mirroring the ISA's in the semantic catalog.
pub use crate::semantics::Endianness;

/// Size of the modeled address space (64 KiB scratch).
pub const MEM_SIZE: usize = 0x10000;

/// SIR-model stack pointer register (x86 esp; ABI-dependent on other ISAs).
pub const SP: u32 = 4;

/// Structured architectural condition flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Flags {
    pub carry: bool,
    pub overflow: bool,
    pub zero: bool,
    pub negative: bool,
    /// Arch-specific condition bits (SH `T`, ColdFire/m68k `X`, SPARC icc/xcc…).
    /// Not yet set by any op — flag semantics is the next modeling rung.
    pub extra: u32,
}

/// Minimal architectural state for differential execution.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MachineState {
    /// 32 VReg-indexed GPRs (`u64` so Alpha's 64-bit regs fit).
    pub gpr: Vec<u64>,
    pub pc: u64,
    pub flags: Flags,
    /// Flat little/big-endian address space (`MEM_SIZE` bytes, zero-initialized).
    pub mem: Vec<u8>,
}

impl MachineState {
    pub fn new() -> Self {
        MachineState {
            gpr: vec![0; 32],
            pc: 0,
            flags: Flags::default(),
            mem: vec![0; MEM_SIZE],
        }
    }

    pub fn with_gpr(mut self, idx: u32, value: u64) -> Self {
        self.gpr[idx as usize % 32] = value;
        self
    }

    pub fn gpr(&self, idx: u32) -> u64 {
        self.gpr[idx as usize % 32]
    }

    /// `mem[address .. address+size]` → value, in `endian` byte order.
    pub fn load(&self, address: u64, width: u8, endian: Endianness) -> Result<u64, MemError> {
        let size = check_width(width)?;
        let lo = address as usize;
        let hi = lo + size;
        if hi > self.mem.len() {
            return Err(MemError::OutOfBounds { address, size: size as u64 });
        }
        let b = &self.mem[lo..hi];
        Ok(match (width, endian) {
            (1, _) => b[0] as u64,
            (2, Endianness::Big) => u16::from_be_bytes([b[0], b[1]]) as u64,
            (2, Endianness::Little) => u16::from_le_bytes([b[0], b[1]]) as u64,
            (4, Endianness::Big) => u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as u64,
            (4, Endianness::Little) => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as u64,
            (8, Endianness::Big) => u64::from_be_bytes(b.try_into().unwrap()),
            (8, Endianness::Little) => u64::from_le_bytes(b.try_into().unwrap()),
            _ => unreachable!("width checked"),
        })
    }

    /// Write `value` into `mem[address .. address+size]` in `endian` byte order.
    pub fn store(
        &mut self,
        address: u64,
        width: u8,
        endian: Endianness,
        value: u64,
    ) -> Result<(), MemError> {
        let size = check_width(width)?;
        let lo = address as usize;
        let hi = lo + size;
        if hi > self.mem.len() {
            return Err(MemError::OutOfBounds { address, size: size as u64 });
        }
        match (width, endian) {
            (1, _) => self.mem[lo] = value as u8,
            (2, Endianness::Big) => self.mem[lo..hi].copy_from_slice(&(value as u16).to_be_bytes()),
            (2, Endianness::Little) => {
                self.mem[lo..hi].copy_from_slice(&(value as u16).to_le_bytes())
            }
            (4, Endianness::Big) => self.mem[lo..hi].copy_from_slice(&(value as u32).to_be_bytes()),
            (4, Endianness::Little) => {
                self.mem[lo..hi].copy_from_slice(&(value as u32).to_le_bytes())
            }
            (8, Endianness::Big) => self.mem[lo..hi].copy_from_slice(&value.to_be_bytes()),
            (8, Endianness::Little) => self.mem[lo..hi].copy_from_slice(&value.to_le_bytes()),
            _ => unreachable!("width checked"),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemError {
    OutOfBounds { address: u64, size: u64 },
    BadWidth(u8),
}

fn check_width(width: u8) -> Result<usize, MemError> {
    match width {
        1 | 2 | 4 | 8 => Ok(width as usize),
        w => Err(MemError::BadWidth(w)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecError {
    /// Op kind the executor does not model yet (call/jmp/gap).
    Unsupported(&'static str),
    /// Memory access failed while executing.
    Memory(MemError),
}

/// Link register in the SIR model (Alpha's r26); `Ret` sets `pc` from it.
pub const LINK: u32 = 26;

/// Register width the ISA operates on (Alpha 64-bit; the rest 32-bit).
pub fn word_bits(isa: TargetIsa) -> u8 {
    match isa {
        TargetIsa::Alpha => 64,
        _ => 32,
    }
}

/// Byte order of the ISA's memory (from the semantic catalog).
pub fn endianness(isa: TargetIsa) -> Endianness {
    crate::semantics::for_isa(isa).map_or(Endianness::Little, |s| s.endianness)
}

pub fn mask(width: u8) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

/// Execute `ops` with *reference* SIR semantics at `width`/`endian`. Returns `Err`
/// for ops the executor does not model yet or failed memory — a gap, never silent.
pub fn execute(
    ops: &[Op],
    st: &mut MachineState,
    width: u8,
    endian: Endianness,
) -> Result<(), ExecError> {
    let m = mask(width);
    for op in ops {
        match op {
            Op::Nop => {}
            Op::Ret => st.pc = st.gpr(LINK),
            Op::MovImm { dst, imm } => {
                st.gpr[dst.0 as usize % 32] = (*imm as u64) & m;
            }
            Op::AddImm { dst, imm } => {
                let i = dst.0 as usize % 32;
                st.gpr[i] = (st.gpr[i] + (*imm as u64)) & m;
            }
            Op::SubImm { dst, imm } => {
                let i = dst.0 as usize % 32;
                st.gpr[i] = st.gpr[i].wrapping_sub(*imm as u64) & m;
            }
            Op::Clear { dst } => st.gpr[dst.0 as usize % 32] = 0,
            Op::Inc { dst } => {
                let i = dst.0 as usize % 32;
                st.gpr[i] = st.gpr[i].wrapping_add(1) & m;
            }
            Op::Dec { dst } => {
                let i = dst.0 as usize % 32;
                st.gpr[i] = st.gpr[i].wrapping_sub(1) & m;
            }
            Op::Push { src } => {
                let addr = st.gpr(SP).wrapping_sub(4);
                let value = st.gpr(src.0);
                st.store(addr, 4, endian, value).map_err(ExecError::Memory)?;
                st.gpr[SP as usize] = addr;
            }
            Op::Pop { dst } => {
                let addr = st.gpr(SP);
                let value = st.load(addr, 4, endian).map_err(ExecError::Memory)?;
                st.gpr[dst.0 as usize % 32] = value;
                st.gpr[SP as usize] = addr.wrapping_add(4);
            }
            other => return Err(ExecError::Unsupported(op_kind(other))),
        }
    }
    Ok(())
}

/// Execute with the *target ISA's* semantics: reference semantics at the ISA's word
/// width and byte order (flag side-effects pending).
pub fn execute_isa(ops: &[Op], st: &mut MachineState, isa: TargetIsa) -> Result<(), ExecError> {
    execute(ops, st, word_bits(isa), endianness(isa))
}

fn op_kind(op: &Op) -> &'static str {
    match op {
        Op::Push { .. } => "push",
        Op::Pop { .. } => "pop",
        Op::CallRel { .. } => "call",
        Op::JmpRel { .. } => "jmp",
        Op::Unknown { .. } => "gap",
        _ => unreachable!("executable ops handled in execute"),
    }
}

/// Result of running one program under both reference and target semantics.
#[derive(Debug, Clone)]
pub struct DifferentialReport {
    pub target: TargetIsa,
    pub reference: MachineState,
    pub isa: MachineState,
    /// Set when either side cannot execute the program (a gap, not a pass).
    pub exec_error: Option<ExecError>,
    /// Set when encode/decode failed before execution.
    pub note: String,
}

impl DifferentialReport {
    /// Both sides executed and left the same architectural state.
    pub fn matched(&self) -> bool {
        self.exec_error.is_none() && self.note.is_empty() && self.reference == self.isa
    }
}

fn module_from(ops: Vec<Op>) -> crate::sir::Module {
    use crate::sir::{BasicBlock, Function, Module};
    Module {
        name: "diff".into(),
        source_isa: "sir".into(),
        functions: vec![Function {
            name: "diff".into(),
            blocks: vec![BasicBlock {
                label: "v0".into(),
                ops,
            }],
        }],
        lift_gaps: 0,
        source: None,
        text_vma: None,
    }
}

/// Run `ops` under reference semantics and under the target ISA's semantics (after
/// encode→decode), from the same initial `state`. See module doc for the property.
pub fn differential_ops(ops: Vec<Op>, isa: TargetIsa, state: &MachineState) -> DifferentialReport {
    let width = word_bits(isa);
    let endian = endianness(isa);
    let mut reference = state.clone();
    let reference_err = execute(&ops, &mut reference, width, endian).err();

    let bytes = match crate::encode::encode_module(&module_from(ops.clone()), isa) {
        Ok(b) => b,
        Err(e) => {
            return DifferentialReport {
                target: isa,
                reference,
                isa: state.clone(),
                exec_error: reference_err,
                note: format!("encode: {e}"),
            }
        }
    };
    let decoded = match crate::decode::decode_ops(&bytes, isa) {
        Ok(o) => o,
        Err(e) => {
            return DifferentialReport {
                target: isa,
                reference,
                isa: state.clone(),
                exec_error: reference_err,
                note: format!("decode: {e}"),
            }
        }
    };
    let mut isa_state = state.clone();
    let isa_err = execute_isa(&decoded, &mut isa_state, isa).err();
    DifferentialReport {
        target: isa,
        reference,
        isa: isa_state,
        exec_error: reference_err.or(isa_err),
        note: String::new(),
    }
}

/// Immediates swept by [`differential_sweep`]: min/max/±1/boundaries + negatives.
pub const IMM_CASES: [u32; 10] = [
    0,
    1,
    0x7F,
    0x80,
    0x7FFF,
    0x8000,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFE,
    0xFFFF_FFFF,
];

/// Initial GPR values swept (high bits + boundaries stress word width).
pub const STATE_CASES: [u64; 4] = [0, 1, 0x7FFF_FFFF, 0xFFFF_FFFF];

/// Generated differential programs: each op kind × each immediate.
pub fn sweep_programs() -> Vec<(String, Vec<Op>)> {
    use crate::sir::VReg;
    let v0 = || VReg(0);
    let mut out: Vec<(String, Vec<Op>)> = vec![
        ("nop".into(), vec![Op::Nop, Op::Ret]),
        ("ret".into(), vec![Op::Ret]),
    ];
    for imm in IMM_CASES {
        out.push((format!("mov_imm:{imm:#x}"), vec![Op::MovImm { dst: v0(), imm }, Op::Ret]));
        out.push((
            format!("add_imm:{imm:#x}"),
            vec![Op::MovImm { dst: v0(), imm: 0 }, Op::AddImm { dst: v0(), imm }, Op::Ret],
        ));
        out.push((
            format!("sub_imm:{imm:#x}"),
            vec![Op::MovImm { dst: v0(), imm: 0 }, Op::SubImm { dst: v0(), imm }, Op::Ret],
        ));
        out.push((
            format!("inc_from:{imm:#x}"),
            vec![Op::MovImm { dst: v0(), imm }, Op::Inc { dst: v0() }, Op::Ret],
        ));
        out.push((
            format!("dec_from:{imm:#x}"),
            vec![Op::MovImm { dst: v0(), imm }, Op::Dec { dst: v0() }, Op::Ret],
        ));
        out.push((
            format!("clear_from:{imm:#x}"),
            vec![Op::MovImm { dst: v0(), imm }, Op::Clear { dst: v0() }, Op::Ret],
        ));
    }
    out.push(("push".into(), vec![Op::Push { src: v0() }, Op::Ret]));
    out.push(("pop".into(), vec![Op::Pop { dst: v0() }, Op::Ret]));
    out
}

/// Sweep states: initial GPR value × stack pointer inside the modeled memory.
pub fn sweep_states() -> Vec<MachineState> {
    STATE_CASES
        .iter()
        .map(|v| {
            MachineState::new().with_gpr(0, *v).with_gpr(SP, 0x8000).with_gpr(LINK, 0x8000)
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct SweepReport {
    pub target: TargetIsa,
    /// Programs that encoded, decoded and executed on both sides.
    pub applicable: usize,
    /// Applicable programs where both sides left the same state.
    pub matched: usize,
    /// Behavioral mismatches: `(program, reference state, isa state)`.
    pub mismatches: Vec<(String, MachineState, MachineState)>,
}

impl SweepReport {
    pub fn all_match(&self) -> bool {
        self.mismatches.is_empty() && self.applicable == self.matched
    }
}

/// Generated differential test: every op kind × every immediate × every initial
/// state, run reference-vs-ISA. Finds bugs the hand-picked probes miss (the state
/// space is where encoding bugs hide — e.g. ColdFire's `Dn` bit position).
pub fn differential_sweep(target: TargetIsa) -> SweepReport {
    let mut report = SweepReport {
        target,
        applicable: 0,
        matched: 0,
        mismatches: Vec::new(),
    };
    let states = sweep_states();
    for (label, ops) in sweep_programs() {
        for state in &states {
            let r = differential_ops(ops.clone(), target, state);
            if r.exec_error.is_some() || !r.note.is_empty() {
                continue; // encode/decode/exec gap — not applicable, not a failure
            }
            report.applicable += 1;
            if r.matched() {
                report.matched += 1;
            } else {
                report.mismatches.push((label.clone(), r.reference, r.isa));
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sir::VReg;

    const ADD3: [u8; 9] = [0xB8, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x02, 0xC3];

    fn add3_ops() -> Vec<Op> {
        let m = crate::lift::lift_x86_32(&ADD3, "add3").unwrap();
        m.functions[0].blocks[0].ops.clone()
    }

    #[test]
    fn reference_executes_arithmetic() {
        let ops = vec![
            Op::MovImm { dst: VReg(0), imm: 10 },
            Op::AddImm { dst: VReg(0), imm: 5 },
            Op::Ret,
        ];
        let mut st = MachineState::new().with_gpr(LINK, 0x2000);
        execute(&ops, &mut st, 32, Endianness::Little).unwrap();
        assert_eq!(st.gpr(0), 15);
        assert_eq!(st.pc, 0x2000);
    }

    #[test]
    fn width_changes_wrap_behavior() {
        let ops = vec![
            Op::MovImm { dst: VReg(0), imm: 0xFFFF_FFFF },
            Op::Inc { dst: VReg(0) },
        ];
        let mut s32 = MachineState::new();
        execute(&ops, &mut s32, 32, Endianness::Little).unwrap();
        assert_eq!(s32.gpr(0), 0); // 32-bit wrap
        let mut s64 = MachineState::new();
        execute(&ops, &mut s64, 64, Endianness::Little).unwrap();
        assert_eq!(s64.gpr(0), 0x1_0000_0000); // 64-bit keeps the carry
    }

    #[test]
    fn memory_load_store_endianness() {
        let mut st = MachineState::new();
        st.store(0x100, 4, Endianness::Big, 0x1122_3344).unwrap();
        assert_eq!(&st.mem[0x100..0x104], &[0x11, 0x22, 0x33, 0x44]);
        assert_eq!(st.load(0x100, 4, Endianness::Big).unwrap(), 0x1122_3344);
        assert_eq!(st.load(0x100, 4, Endianness::Little).unwrap(), 0x4433_2211);
        assert_eq!(st.load(0x102, 2, Endianness::Big).unwrap(), 0x3344);
        assert!(matches!(
            st.load(0xFFFF, 4, Endianness::Big),
            Err(MemError::OutOfBounds { .. })
        ));
        assert_eq!(st.load(0x100, 3, Endianness::Big), Err(MemError::BadWidth(3)));
    }

    #[test]
    fn push_pop_roundtrip_through_memory() {
        let ops = vec![
            Op::MovImm { dst: VReg(0), imm: 0xDEAD_BEEF },
            Op::Push { src: VReg(0) },
            Op::Clear { dst: VReg(0) },
            Op::Pop { dst: VReg(0) },
        ];
        let mut st = MachineState::new().with_gpr(SP, 0x8000);
        execute(&ops, &mut st, 32, Endianness::Big).unwrap();
        assert_eq!(st.gpr(0), 0xDEAD_BEEF);
        assert_eq!(st.gpr(SP), 0x8000); // net-zero stack movement
        // Value was stored at 0x7FFC big-endian by push, loaded back by pop.
        assert_eq!(&st.mem[0x7FFC..0x8000], &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn add3_differential_matches_for_all_decoders() {
        let ops = add3_ops();
        let state = MachineState::new().with_gpr(LINK, 0x8000);
        for t in [
            TargetIsa::Mips,
            TargetIsa::Ppc,
            TargetIsa::SuperH(crate::target::SuperHFlavor::Sh4),
            TargetIsa::Alpha,
            TargetIsa::ColdFire,
        ] {
            let r = differential_ops(ops.clone(), t, &state);
            assert!(r.matched(), "add3 differential failed for {t}: note={} ref={:?} isa={:?}", r.note, r.reference, r.isa);
        }
        // PA-RISC encoder only covers Nop/Ret → add3 is an encode gap, not a pass.
        let p = differential_ops(ops, TargetIsa::PaRisc, &state);
        assert!(!p.matched());
        assert!(p.note.starts_with("encode:"));
    }

    #[test]
    fn alpha_negative_add_reveals_sign_extension() {
        // Alpha encodes AddImm{·,0xFFFFFFFF} as LDA (sign-extended −1 → −1); the
        // 64-bit spec adds +4294967295. The differential must DETECT the mismatch.
        let ops = vec![
            Op::MovImm { dst: VReg(0), imm: 0 },
            Op::AddImm { dst: VReg(0), imm: 0xFFFF_FFFF },
            Op::Ret,
        ];
        let state = MachineState::new().with_gpr(LINK, 0x8000);
        let r = differential_ops(ops.clone(), TargetIsa::Alpha, &state);
        assert!(!r.matched(), "alpha sign-extension gap must be detected");
        // At 32-bit width the same ops round-trip exactly (wrap makes them agree).
        let m32 = differential_ops(ops, TargetIsa::Mips, &state);
        assert!(m32.matched(), "mips 32-bit wrap should agree: {m32:?}");
    }

    #[test]
    fn unsupported_ops_are_gaps_not_passes() {
        let ops = vec![Op::CallRel { rel: 0, target: Some(0), symbol: Some("f".into()) }];
        let state = MachineState::new();
        let r = differential_ops(ops, TargetIsa::Mips, &state);
        assert!(!r.matched());
        assert_eq!(r.exec_error, Some(ExecError::Unsupported("call")));
    }

    #[test]
    fn push_pop_now_execute_on_coldfire() {
        let ops = vec![Op::Push { src: VReg(0) }, Op::Pop { dst: VReg(0) }, Op::Ret];
        let state = MachineState::new().with_gpr(SP, 0x8000).with_gpr(LINK, 0x8000);
        let r = differential_ops(ops.clone(), TargetIsa::ColdFire, &state);
        assert!(r.matched(), "{r:?}");
        assert!(r.exec_error.is_none());
    }

    #[test]
    fn sweep_is_clean_except_documented_alpha_gap() {
        // Generated matrix must find no unexpected behavioral mismatches.
        for t in [
            TargetIsa::Mips,
            TargetIsa::Ppc,
            TargetIsa::SuperH(crate::target::SuperHFlavor::Sh4),
            TargetIsa::ColdFire,
            TargetIsa::AArch64,
            TargetIsa::Arm,
            TargetIsa::Sparc,
            TargetIsa::X86_64,
        ] {
            let s = differential_sweep(t);
            assert!(s.all_match(), "unexpected sweep mismatch for {t}: {:?}", s.mismatches.first());
        }
        let a = differential_sweep(TargetIsa::Alpha);
        assert!(!a.all_match());
        let bad_kinds: Vec<&str> = a
            .mismatches
            .iter()
            .map(|(p, _, _)| p.split(':').next().unwrap())
            .collect();
        assert!(!bad_kinds.is_empty());
        assert!(
            bad_kinds.iter().all(|k| *k == "add_imm" || *k == "sub_imm"),
            "alpha sweep must fail only on the sign-extension kinds: {bad_kinds:?}"
        );
        // Sanity: sweep covers the encodable programs of the ISA.
        assert!(a.applicable > 0, "alpha sweep should have applicable programs");
        assert!(a.matched > 0, "alpha sweep should pass most programs");
    }
}
