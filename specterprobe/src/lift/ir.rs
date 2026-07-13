use crate::lift::types::{ArmOperand, BasicBlock, CondCode, Function, InstKind, Instruction, Reg, Size};
use std::collections::HashMap;

type ValId = usize;

#[derive(Clone, Debug)]
enum LlvmType {
    I(u8),
    Ptr,
    Void,
}

impl LlvmType {
    fn to_str(&self) -> &'static str {
        match self {
            LlvmType::I(8) => "i8",
            LlvmType::I(16) => "i16",
            LlvmType::I(32) => "i32",
            LlvmType::I(64) => "i64",
            LlvmType::I(128) => "i128",
            LlvmType::Ptr => "ptr",
            LlvmType::Void => "void",
            _ => "i64",
        }
    }
}

struct SsaBlock {
    label: String,
    instructions: Vec<String>,
    exit_value: String,
}

struct FunctionState {
    names: HashMap<Reg, ValId>,
    next_val: ValId,
    cond_on_entry: Option<String>,
    entry_block: u64,
    block_order: Vec<u64>,
    phis: Vec<(String, ValId, LlvmType, Vec<(ValId, String)>)>,
}

impl FunctionState {
    fn new() -> Self {
        Self {
            names: HashMap::new(),
            next_val: 1,
            cond_on_entry: None,
            entry_block: 0,
            block_order: Vec::new(),
            phis: Vec::new(),
        }
    }

    fn reg_val(&mut self, r: &Reg) -> ValId {
        match r {
            Reg::Xzr | Reg::Wzr => return 0,
            _ => {}
        }
        *self.names.entry(r.clone()).or_insert_with(|| {
            let id = self.next_val;
            self.next_val += 1;
            id
        })
    }

    fn reg_val_for_phi(&self, r: &Reg) -> Option<ValId> {
        match r {
            Reg::Xzr | Reg::Wzr => Some(0),
            _ => self.names.get(r).copied(),
        }
    }

    fn new_val(&mut self) -> ValId {
        let id = self.next_val;
        self.next_val += 1;
        id
    }

    fn val_ref(&self, id: ValId) -> String {
        if id == 0 { "0".into() } else { format!("%v{}", id) }
    }

    fn reg_ref(&mut self, r: &Reg) -> String {
        let id = self.reg_val(r);
        self.val_ref(id)
    }

    fn reg_type(r: &Reg) -> LlvmType {
        match r {
            Reg::W(_) | Reg::Wzr => LlvmType::I(32),
            _ => LlvmType::I(64),
        }
    }
}

fn block_label(addr: u64) -> String {
    format!("L{:x}", addr)
}

pub fn generate_ir(functions: &[Function], module_name: &str) -> String {
    let mut ir = String::new();
    ir.push_str(&format!("; ModuleID = '{}'\n", module_name));
    ir.push_str("target datalayout = \"e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128\"\n");
    ir.push_str("target triple = \"aarch64-unknown-linux-android\"\n\n");
    ir.push_str("declare void @putchar(i8)\n");
    ir.push_str("declare i64 @read()\n");
    ir.push_str("declare void @unimplemented()\n");
    ir.push_str("declare void @mmio_write(i64, i64)\n");
    ir.push_str("declare i64 @mmio_read(i64)\n\n");

    for func in functions {
        ir.push_str(&emit_function(func));
        ir.push('\n');
    }
    ir
}

fn emit_function(func: &Function) -> String {
    let fn_name = sanitize(&func.name);
    let mut ir = String::new();

    ir.push_str(&format!("define i64 @{}() {{\n", fn_name));

    let mut st = FunctionState::new();
    st.entry_block = func.blocks.first().map(|b| b.address).unwrap_or(0);

    let mut block_map: HashMap<u64, &BasicBlock> = HashMap::new();
    for block in &func.blocks {
        block_map.insert(block.address, block);
    }

    order_blocks(func, &mut st);

    let mut block_irs: Vec<(u64, SsaBlock)> = Vec::new();

    let block_order = st.block_order.clone();

    for &addr in &block_order {
        if let Some(block) = block_map.get(&addr) {
            let ssa = emit_block(block, &mut st, &block_map);
            block_irs.push((addr, ssa));
        }
    }

    let mut phi_lines: HashMap<String, Vec<String>> = HashMap::new();
    for (label, vid, ty, incoming) in &st.phis {
        let inc: Vec<String> = incoming.iter()
            .map(|(iv, lbl)| format!("[ {} %{} ]", st.val_ref(*iv), lbl))
            .collect();
        phi_lines.entry(label.clone()).or_default()
            .push(format!("  {} = phi {} {}", val_id(*vid), ty.to_str(), inc.join(", ")));
    }

    for (_addr, ssa) in &block_irs {
        ir.push_str(&format!("{}:\n", ssa.label));
        if let Some(phi_lines) = phi_lines.get(&ssa.label) {
            for line in phi_lines {
                ir.push_str(line);
                ir.push('\n');
            }
        }
        for inst in &ssa.instructions {
            ir.push_str("  ");
            ir.push_str(inst);
            ir.push('\n');
        }
        ir.push_str(&format!("  {}", ssa.exit_value));
    }

    ir.push_str("}\n");
    ir
}

fn val_id(v: ValId) -> String {
    format!("%v{}", v)
}

fn order_blocks(func: &Function, st: &mut FunctionState) {
    let mut visited = std::collections::HashSet::new();
    let mut order = Vec::new();

    fn dfs(addr: u64, block_map: &HashMap<u64, &BasicBlock>, visited: &mut std::collections::HashSet<u64>, order: &mut Vec<u64>) {
        if !visited.insert(addr) { return; }
        order.push(addr);
        if let Some(block) = block_map.get(&addr) {
            for succ in &block.successors {
                dfs(*succ, block_map, visited, order);
            }
        }
    }

    let block_map: HashMap<u64, &BasicBlock> = func.blocks.iter().map(|b| (b.address, b)).collect();
    dfs(func.entry, &block_map, &mut visited, &mut order);

    for b in &func.blocks {
        if !visited.contains(&b.address) {
            order.push(b.address);
        }
    }

    st.block_order = order;
}

fn emit_block(
    block: &BasicBlock,
    st: &mut FunctionState,
    block_map: &HashMap<u64, &BasicBlock>,
) -> SsaBlock {
    let label = block_label(block.address);
    let mut instructions = Vec::new();
    let mut cond: Option<(String, Reg, ArmOperand)> = None;

    let preds = find_predecessors(block.address, block_map);

    for (pred_addr, _) in &preds {
        if *pred_addr == block.address { continue; }
        let pred_block = match block_map.get(pred_addr) { Some(b) => b, None => continue };
        let pred_last = match pred_block.instructions.last() { Some(i) => i, None => continue };
        let written_regs = get_written_regs(pred_last);
        for r in written_regs {
            if let Some(val) = st.reg_val_for_phi(&r) {
                let pred_label = block_label(*pred_addr);
                let phi_vid = st.new_val();
                st.phis.push((label.clone(), phi_vid, FunctionState::reg_type(&r), vec![(val, pred_label)]));
            }
        }
    }

    let _saved_next = st.next_val;

    for insn in &block.instructions {
        let line = emit_ssa_inst(insn, st, &mut cond);
        if let Some(l) = line {
            if st.next_val > _saved_next + 1 {
                let mut line_with_comment = String::new();
                let v = st.next_val - 1;
                line_with_comment.push_str(&val_id(v));
                line_with_comment.push_str(" = ");
                line_with_comment.push_str(&l);
                line_with_comment.push_str(&format!(" ; {}", insn.mnemonic));
                if !insn.op_str.is_empty() {
                    line_with_comment.push_str(&format!(" {}", insn.op_str));
                }
                instructions.push(line_with_comment);
            } else {
                let mut line_with_comment = String::new();
                line_with_comment.push_str(&format!("{} ; {} {}", l, insn.mnemonic, insn.op_str));
                instructions.push(line_with_comment);
            }
        } else {
            let skip = matches!(&insn.kind, InstKind::Nop | InstKind::Ret(_));
            if !skip {
                instructions.push(format!("; {} {} ; (skipped in SSA)", insn.mnemonic, insn.op_str));
            }
        }
    }

    let exit_value = emit_terminator(block, st, &cond, block_map);

    SsaBlock { label, instructions, exit_value }
}

fn find_predecessors<'a>(addr: u64, block_map: &'a HashMap<u64, &BasicBlock>) -> Vec<(u64, &'a BasicBlock)> {
    let mut preds = Vec::new();
    for (_, block) in block_map {
        if block.successors.contains(&addr) || block.cond_successor == Some(addr) {
            preds.push((block.address, *block));
        }
    }
    preds
}

fn get_written_regs(insn: &Instruction) -> Vec<Reg> {
    match &insn.kind {
        InstKind::Mov(_, dst, _)
        | InstKind::Add(_, dst, _, _)
        | InstKind::Sub(_, dst, _, _)
        | InstKind::Mul(_, dst, _, _)
        | InstKind::Sdiv(_, dst, _, _)
        | InstKind::Udiv(_, dst, _, _)
        | InstKind::And_(_, dst, _, _)
        | InstKind::Orr(_, dst, _, _)
        | InstKind::Eor(_, dst, _, _)
        | InstKind::Lsl(_, dst, _, _)
        | InstKind::Lsr(_, dst, _, _)
        | InstKind::Asr(_, dst, _, _)
        | InstKind::Load(_, dst, _)
        | InstKind::Adrp(dst, _)
        | InstKind::Adr(dst, _) => vec![dst.clone()],
        InstKind::LoadPair(_, r1, r2, _) => vec![r1.clone(), r2.clone()],
        InstKind::Cmp(_, _, _) => vec![], // implicit flags
        _ => vec![],
    }
}

fn emit_ssa_inst(insn: &Instruction, st: &mut FunctionState, cond: &mut Option<(String, Reg, ArmOperand)>) -> Option<String> {
    match &insn.kind {
        InstKind::Mov(sz, dst, src) => {
            let src_v = operand_value(src, st);
            let ty = size_to_llvm(sz);
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            Some(format!("{} = {} {}", val_id(st.next_val - 1), "add" , format!("{} {}, 0", ty.to_str(), src_v)))
        }
        InstKind::Add(sz, dst, src, op2) => {
            let src_v = st.reg_ref(src);
            let op2_v = operand_value(op2, st);
            let ty = size_to_llvm(sz);
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            Some(format!("{} = {} {}", val_id(st.next_val - 1), "add" , format!("{} {}, {}", ty.to_str(), src_v, op2_v)))
        }
        InstKind::Sub(sz, dst, src, op2) => {
            let src_v = st.reg_ref(src);
            let op2_v = operand_value(op2, st);
            let ty = size_to_llvm(sz);
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            Some(format!("{} = {} {}", val_id(st.next_val - 1), "sub" , format!("{} {}, {}", ty.to_str(), src_v, op2_v)))
        }
        InstKind::Cmp(_sz, r, op2) => {
            let rv = st.reg_ref(r);
            let op2_v = operand_value(op2, st);
            let cond_str = format!("{} {}", rv, op2_v);
            *cond = Some((cond_str, r.clone(), op2.clone()));
            st.next_val += 1;
            Some(format!("; cmp {} {}", rv, op2_v))
        }
        InstKind::Mul(_sz, dst, src, op2) => {
            let src_v = st.reg_ref(src);
            let op2_v = operand_value(op2, st);
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            Some(format!("{} = mul i64 {}, {}", val_id(st.next_val - 1), src_v, op2_v))
        }
        InstKind::And_(sz, dst, src, op2) => {
            let src_v = st.reg_ref(src);
            let op2_v = operand_value(op2, st);
            let ty = size_to_llvm(sz);
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            Some(format!("{} = and {} {}, {}", val_id(st.next_val - 1), ty.to_str(), src_v, op2_v))
        }
        InstKind::Orr(sz, dst, src, op2) => {
            let src_v = st.reg_ref(src);
            let op2_v = operand_value(op2, st);
            let ty = size_to_llvm(sz);
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            Some(format!("{} = or {} {}, {}", val_id(st.next_val - 1), ty.to_str(), src_v, op2_v))
        }
        InstKind::Eor(sz, dst, src, op2) => {
            let src_v = st.reg_ref(src);
            let op2_v = operand_value(op2, st);
            let ty = size_to_llvm(sz);
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            Some(format!("{} = xor {} {}, {}", val_id(st.next_val - 1), ty.to_str(), src_v, op2_v))
        }
        InstKind::Lsl(sz, dst, src, op2) => {
            let src_v = st.reg_ref(src);
            let op2_v = operand_value(op2, st);
            let ty = size_to_llvm(sz);
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            Some(format!("{} = shl {} {}, {}", val_id(st.next_val - 1), ty.to_str(), src_v, op2_v))
        }
        InstKind::Lsr(sz, dst, src, op2) => {
            let src_v = st.reg_ref(src);
            let op2_v = operand_value(op2, st);
            let ty = size_to_llvm(sz);
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            Some(format!("{} = lshr {} {}, {}", val_id(st.next_val - 1), ty.to_str(), src_v, op2_v))
        }
        InstKind::Asr(sz, dst, src, op2) => {
            let src_v = st.reg_ref(src);
            let op2_v = operand_value(op2, st);
            let ty = size_to_llvm(sz);
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            Some(format!("{} = ashr {} {}, {}", val_id(st.next_val - 1), ty.to_str(), src_v, op2_v))
        }
        InstKind::Load(sz, dst, addr) => {
            let base_v = st.reg_ref(&addr.base);
            let ty = size_to_llvm(sz);
            let _ptr_v = st.new_val();
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            let load = format!("%v{} = load {}, ptr null ; base={}", st.next_val - 1, ty.to_str(), base_v);
            Some(format!("{} ; ldr {} [{}]", load, reg_name_short(&dst), reg_name_short(&addr.base)))
        }
        InstKind::Store(sz, addr, val) => {
            let base_v = st.reg_ref(&addr.base);
            let val_v = st.reg_ref(val);
            let ty = size_to_llvm(sz);
            let _ptr_v = st.new_val();
            let store = format!("store {} {}, ptr null ; base={}", ty.to_str(), val_v, base_v);
            Some(format!("{} ; str {} [{}]", store, reg_name_short(val), reg_name_short(&addr.base)))
        }
        InstKind::Adrp(dst, page) => {
            st.names.insert(dst.clone(), st.next_val);
            st.next_val += 1;
            Some(format!("{} = add i64 0, {}", val_id(st.next_val - 1), page))
        }
        InstKind::BranchLink(target) => {
            st.next_val += 1;
            Some(format!("call void @sub_{:x}()", target))
        }
        InstKind::Svc(n) => {
            st.next_val += 1;
            Some(format!("call void @unimplemented() ; svc #{}", n))
        }
        InstKind::Nop => None,
        InstKind::Ret(_) => None,
        _ => None,
    }
}

fn emit_terminator(
    block: &BasicBlock,
    st: &mut FunctionState,
    cond: &Option<(String, Reg, ArmOperand)>,
    block_map: &HashMap<u64, &BasicBlock>,
) -> String {
    let last = match block.instructions.last() {
        Some(i) => i,
        None => return String::new(),
    };

    match &last.kind {
        InstKind::BranchAlways(target) => {
            let label = block_label(*target);
            if *target == block.address {
                format!("br label %{}\n", label)
            } else {
                format!("br label %{}\n", label)
            }
        }
        InstKind::Branch(cc, target) => {
            let cond_label = block_label(*target);
            let _ = cond;
            let _ = cc;
            format!("br label %{} ; br (conditional) {}\n", cond_label, last.mnemonic)
        }
        InstKind::CompareBranch(is_nz, r, target) => {
            let rv = st.reg_ref(r);
            let cond_label = block_label(*target);
            let pred = if *is_nz { "ne" } else { "eq" };
            let cmp = st.new_val();
            format!("{} = icmp {} i64 {}, 0\n  br i1 {}, label %{}, label %{}\n",
                val_id(cmp), pred, rv, val_id(cmp), cond_label, cond_label)
        }
        InstKind::BranchLink(_) | InstKind::BranchReg(_) => {
            "br label %end\n".to_string()
        }
        InstKind::Ret(_) => {
            st.next_val += 1;
            format!("ret i64 0\n")
        }
        _ => {
            let fallthrough = find_fallthrough(block, block_map);
            if let Some(ft) = fallthrough {
                format!("br label %{}\n", block_label(ft))
            } else {
                String::new()
            }
        }
    }
}

fn find_fallthrough(block: &BasicBlock, block_map: &HashMap<u64, &BasicBlock>) -> Option<u64> {
    let next_addr = block.address + 4;
    if block_map.contains_key(&next_addr) {
        Some(next_addr)
    } else {
        None
    }
}

fn condcode_to_llvm(cc: &CondCode) -> &'static str {
    match cc {
        CondCode::Eq => "eq", CondCode::Ne => "ne",
        CondCode::Cs => "uge", CondCode::Cc => "ult",
        CondCode::Mi => "slt", CondCode::Pl => "sge",
        CondCode::Vs => "o", CondCode::Vc => "no",
        CondCode::Hi => "ugt", CondCode::Ls => "ule",
        CondCode::Ge => "sge", CondCode::Lt => "slt",
        CondCode::Gt => "sgt", CondCode::Le => "sle",
        CondCode::Al => "true", CondCode::Nv => "false",
    }
}

fn operand_value(op: &ArmOperand, st: &mut FunctionState) -> String {
    match op {
        ArmOperand::Reg(r) => st.reg_ref(r),
        ArmOperand::Imm(val) => format!("{}", val),
        ArmOperand::Mem(addr) => st.reg_ref(&addr.base),
        _ => "0".into(),
    }
}

fn size_to_llvm(s: &Size) -> LlvmType {
    match s {
        Size::B8 => LlvmType::I(8),
        Size::B16 => LlvmType::I(16),
        Size::B32 => LlvmType::I(32),
        Size::B64 => LlvmType::I(64),
        Size::B128 => LlvmType::I(128),
    }
}

fn reg_name_short(r: &Reg) -> String {
    match r {
        Reg::X(n) => format!("x{n}"), Reg::W(n) => format!("w{n}"),
        Reg::R(n) => format!("r{n}"),
        Reg::Sp => "sp".into(), Reg::Fp => "fp".into(), Reg::Lr => "lr".into(),
        Reg::Xzr => "xzr".into(), Reg::Wzr => "wzr".into(), Reg::Pc => "pc".into(),
    }
}

fn sanitize(name: &str) -> String {
    name.replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
}
