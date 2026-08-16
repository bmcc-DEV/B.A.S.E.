//! Load `.text` from PE32/PE32+ (x86 / x86_64 only). ≠ Win32 ABI · ≠ runs_any_pe.

use std::fs;
use std::path::Path;

use object::{Architecture, Object, ObjectSection, ObjectSymbol};
use thiserror::Error;

use crate::lift::{lift_x86_32_at, LiftError};
use crate::sir::Module;
use crate::symbols::{resolve_symbols, SymbolMap};

#[derive(Debug, Error)]
pub enum PeError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("object parse: {0}")]
    Object(String),
    #[error("not a PE image")]
    NotPe,
    #[error("no .text / executable section")]
    NoText,
    #[error("PE architecture {0:?} not supported — need X86_64 or I386")]
    UnsupportedArch(Architecture),
    #[error(transparent)]
    Lift(#[from] LiftError),
}

#[derive(Debug, Clone)]
pub struct PeText {
    pub bytes: Vec<u8>,
    pub vma: u64,
    pub section_name: String,
    pub is_64: bool,
    pub path: String,
    pub architecture: String,
    pub symbols: SymbolMap,
}

fn arch_ok(arch: Architecture) -> bool {
    matches!(arch, Architecture::X86_64 | Architecture::I386)
}

/// Extract PE `.text` (+ symbol map). Does **not** load imports / run Win32.
pub fn load_pe_text(path: &Path) -> Result<PeText, PeError> {
    let data = fs::read(path)?;
    let file = object::File::parse(&*data).map_err(|e| PeError::Object(e.to_string()))?;
    if file.format() != object::BinaryFormat::Pe {
        return Err(PeError::NotPe);
    }
    let arch = file.architecture();
    if !arch_ok(arch) {
        return Err(PeError::UnsupportedArch(arch));
    }
    let is_64 = file.is_64();
    let path_s = path.display().to_string();
    let architecture = format!("{arch:?}");

    let mut symbols = SymbolMap::new();
    for sym in file.symbols() {
        if let Ok(name) = sym.name() {
            if !name.is_empty() {
                symbols.entry(sym.address()).or_insert_with(|| name.to_string());
            }
        }
    }

    if let Some(sec) = file.section_by_name(".text") {
        let bytes = sec
            .data()
            .map_err(|e| PeError::Object(e.to_string()))?
            .to_vec();
        if !bytes.is_empty() {
            return Ok(PeText {
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
            object::SectionFlags::Coff { characteristics } => {
                characteristics & object::pe::IMAGE_SCN_MEM_EXECUTE != 0
            }
            _ => false,
        };
        if !exec {
            continue;
        }
        let bytes = sec
            .data()
            .map_err(|e| PeError::Object(e.to_string()))?
            .to_vec();
        if bytes.is_empty() {
            continue;
        }
        return Ok(PeText {
            bytes,
            vma: sec.address(),
            section_name: sec.name().unwrap_or("exec").to_string(),
            is_64,
            path: path_s,
            architecture,
            symbols,
        });
    }

    Err(PeError::NoText)
}

pub fn lift_pe_text(path: &Path, fn_name: &str) -> Result<(PeText, Module), PeError> {
    let text = load_pe_text(path)?;
    let mut module = lift_x86_32_at(&text.bytes, fn_name, text.vma)?;
    resolve_symbols(&mut module, &text.symbols);
    module.source = Some(format!("{}:{}", text.path, text.section_name));
    module.text_vma = Some(text.vma);
    Ok((text, module))
}
