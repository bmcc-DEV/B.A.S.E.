//! Load `.text` from ELF32/ELF64 + lift to SIR via ISA-specific decoder.

use std::fs;
use std::path::Path;

use object::{Architecture, Object, ObjectSection, ObjectSymbol};
use thiserror::Error;

use crate::decode::decode_ops;
use crate::lift::{lift_x86_32_at, LiftError};
use crate::sir::{BasicBlock, Function, Module};
use crate::symbols::{resolve_symbols, SymbolMap};
use crate::target::TargetIsa;

#[derive(Debug, Error)]
pub enum ElfError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("object parse: {0}")]
    Object(String),
    #[error("no .text or executable section found")]
    NoText,
    #[error("ELF architecture {0:?} not supported")]
    UnsupportedArch(Architecture),
    #[error(transparent)]
    Lift(#[from] LiftError),
    #[error("decode error: {0}")]
    Decode(String),
}

#[derive(Debug, Clone)]
pub struct ElfText {
    pub bytes: Vec<u8>,
    pub vma: u64,
    pub section_name: String,
    pub is_64: bool,
    pub path: String,
    pub architecture: String,
    pub symbols: SymbolMap,
}

fn arch_to_target_isa(arch: Architecture) -> Option<TargetIsa> {
    match arch {
        Architecture::X86_64 | Architecture::I386 => Some(TargetIsa::X86_64),
        Architecture::Mips => Some(TargetIsa::Mips),
        Architecture::PowerPc | Architecture::PowerPc64 => Some(TargetIsa::Ppc),
        Architecture::Arm => Some(TargetIsa::Arm),
        Architecture::Aarch64 => Some(TargetIsa::AArch64),
        Architecture::Sparc64 | Architecture::Sparc => Some(TargetIsa::Sparc),
        Architecture::Mips64 => Some(TargetIsa::Mips),
        _ => None,
    }
}

fn collect_symbols(file: &object::File<'_>) -> SymbolMap {
    let mut symbols = SymbolMap::new();
    for sym in file.symbols() {
        if let Ok(name) = sym.name() {
            if !name.is_empty() {
                symbols
                    .entry(sym.address())
                    .or_insert_with(|| name.to_string());
            }
        }
    }
    symbols
}

/// Collect function symbols (STT_FUNC) for per-function lifting.
fn collect_func_symbols(file: &object::File<'_>) -> Vec<(u64, u64, String)> {
    let mut funcs = Vec::new();
    for sym in file.symbols() {
        if sym.kind() == object::SymbolKind::Text {
            if let Ok(name) = sym.name() {
                if !name.is_empty() && sym.size() > 0 {
                    funcs.push((sym.address(), sym.size(), name.to_string()));
                }
            }
        }
    }
    funcs.sort_by_key(|&(addr, _, _)| addr);
    funcs
}

pub fn load_elf_text(path: &Path) -> Result<ElfText, ElfError> {
    let data = fs::read(path)?;
    let file = object::File::parse(&*data).map_err(|e| ElfError::Object(e.to_string()))?;
    let arch = file.architecture();
    if arch_to_target_isa(arch).is_none() {
        return Err(ElfError::UnsupportedArch(arch));
    }
    let is_64 = file.is_64();
    let path_s = path.display().to_string();
    let architecture = format!("{arch:?}");
    let symbols = collect_symbols(&file);

    if let Some(sec) = file.section_by_name(".text") {
        let bytes = sec
            .data()
            .map_err(|e| ElfError::Object(e.to_string()))?
            .to_vec();
        if !bytes.is_empty() {
            return Ok(ElfText {
                bytes,
                vma: sec.address(),
                section_name: ".text".into(),
                is_64,
                path: path_s,
                architecture,
                symbols,
            });
        }
    }

    for sec in file.sections() {
        let exec = match sec.flags() {
            object::SectionFlags::Elf { sh_flags } => {
                sh_flags & u64::from(object::elf::SHF_EXECINSTR) != 0
            }
            _ => false,
        };
        if !exec {
            continue;
        }
        let bytes = sec
            .data()
            .map_err(|e| ElfError::Object(e.to_string()))?
            .to_vec();
        if bytes.is_empty() {
            continue;
        }
        let name = sec.name().unwrap_or("exec").to_string();
        return Ok(ElfText {
            bytes,
            vma: sec.address(),
            section_name: name,
            is_64,
            path: path_s,
            architecture,
            symbols,
        });
    }

    Err(ElfError::NoText)
}

/// Lift entire .text as a single function (legacy path for x86).
fn lift_x86_elf(path: &Path, fn_name: &str) -> Result<(ElfText, Module), ElfError> {
    let text = load_elf_text(path)?;
    let mut module = lift_x86_32_at(&text.bytes, fn_name, text.vma)?;
    resolve_symbols(&mut module, &text.symbols);
    module.source = Some(format!("{}:{}", text.path, text.section_name));
    module.text_vma = Some(text.vma);
    Ok((text, module))
}

/// Lift ELF for non-x86 ISAs using the ISA-specific decoder.
/// Slices .text into per-function ranges using ELF symbol table.
fn lift_isa_elf(path: &Path, target: TargetIsa, fn_name: &str) -> Result<(ElfText, Module), ElfError> {
    let data = fs::read(path)?;
    let file = object::File::parse(&*data).map_err(|e| ElfError::Object(e.to_string()))?;
    let arch = file.architecture();
    let is_64 = file.is_64();
    let path_s = path.display().to_string();
    let architecture = format!("{arch:?}");
    let symbols = collect_symbols(&file);
    let func_syms = collect_func_symbols(&file);

    // Find .text section
    let text_sec = file.section_by_name(".text")
        .or_else(|| {
            for sec in file.sections() {
                let exec = match sec.flags() {
                    object::SectionFlags::Elf { sh_flags } => {
                        sh_flags & u64::from(object::elf::SHF_EXECINSTR) != 0
                    }
                    _ => false,
                };
                if exec && sec.data().map(|d| !d.is_empty()).unwrap_or(false) {
                    return Some(sec);
                }
            }
            None
        })
        .ok_or(ElfError::NoText)?;

    let text_bytes = text_sec.data()
        .map_err(|e| ElfError::Object(e.to_string()))?
        .to_vec();
    let vma = text_sec.address();
    let section_name = text_sec.name().unwrap_or(".text").to_string();

    let text = ElfText {
        bytes: text_bytes.clone(),
        vma,
        section_name: section_name.clone(),
        is_64,
        path: path_s.clone(),
        architecture,
        symbols: symbols.clone(),
    };

    // Slice into per-function ranges using symbol table
    let mut functions = Vec::new();
    if func_syms.is_empty() {
        // No symbols — lift entire .text as one function
        let ops = decode_ops(&text_bytes, target)
            .map_err(|e| ElfError::Decode(e.to_string()))?;
        functions.push(Function {
            name: fn_name.to_string(),
            blocks: vec![BasicBlock {
                label: "entry".into(),
                ops,
            }],
        });
    } else {
        for (addr, size, name) in &func_syms {
            let offset = (addr - vma) as usize;
            if offset >= text_bytes.len() {
                continue;
            }
            let end = (offset + *size as usize).min(text_bytes.len());
            let func_bytes = &text_bytes[offset..end];
            let ops = decode_ops(func_bytes, target)
                .map_err(|e| ElfError::Decode(format!("{name}: {e}")))?;
            functions.push(Function {
                name: name.clone(),
                blocks: vec![BasicBlock {
                    label: "entry".into(),
                    ops,
                }],
            });
        }
        // If no functions matched, lift entire .text
        if functions.is_empty() {
            let ops = decode_ops(&text_bytes, target)
                .map_err(|e| ElfError::Decode(e.to_string()))?;
            functions.push(Function {
                name: fn_name.to_string(),
                blocks: vec![BasicBlock {
                    label: "entry".into(),
                    ops,
                }],
            });
        }
    }

    let lift_gaps = functions.iter()
        .flat_map(|f| f.blocks.iter())
        .flat_map(|b| b.ops.iter())
        .filter(|o| matches!(o, crate::sir::Op::Unknown { .. }))
        .count();

    let mut module = Module {
        name: fn_name.to_string(),
        source_isa: target.as_str().to_string(),
        functions,
        lift_gaps,
        source: Some(format!("{}:{}", path_s, section_name)),
        text_vma: Some(vma),
    };
    resolve_symbols(&mut module, &symbols);
    Ok((text, module))
}

pub fn lift_elf_text(path: &Path, fn_name: &str) -> Result<(ElfText, Module), ElfError> {
    let text = load_elf_text(path)?;
    // Re-parse to get Architecture enum from the ELF file.
    let data = fs::read(path)?;
    let file = object::File::parse(&*data).map_err(|e| ElfError::Object(e.to_string()))?;
    let arch = file.architecture();
    let target = arch_to_target_isa(arch)
        .ok_or(ElfError::UnsupportedArch(arch))?;

    if matches!(target, TargetIsa::X86_64) {
        lift_x86_elf(path, fn_name)
    } else {
        lift_isa_elf(path, target, fn_name)
    }
}

/// Lift ELF targeting a specific ISA (for cross-ISA lift + encode).
pub fn lift_elf_text_for_target(path: &Path, fn_name: &str, target: TargetIsa) -> Result<(ElfText, Module), ElfError> {
    if matches!(target, TargetIsa::X86_64) {
        lift_x86_elf(path, fn_name)
    } else {
        lift_isa_elf(path, target, fn_name)
    }
}
