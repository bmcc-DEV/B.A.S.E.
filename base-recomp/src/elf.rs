//! Load `.text` (or executable section) from ELF32/ELF64 (x86 / x86_64 only).

use std::fs;
use std::path::Path;

use object::{Architecture, Object, ObjectSection};
use thiserror::Error;

use crate::lift::{lift_x86_32_at, LiftError};
use crate::sir::Module;

#[derive(Debug, Error)]
pub enum ElfError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("object parse: {0}")]
    Object(String),
    #[error("no .text or executable section found")]
    NoText,
    #[error("ELF architecture {0:?} not supported — need X86_64 or I386")]
    UnsupportedArch(Architecture),
    #[error(transparent)]
    Lift(#[from] LiftError),
}

#[derive(Debug, Clone)]
pub struct ElfText {
    pub bytes: Vec<u8>,
    pub vma: u64,
    pub section_name: String,
    pub is_64: bool,
    pub path: String,
    pub architecture: String,
}

fn arch_ok(arch: Architecture) -> bool {
    matches!(arch, Architecture::X86_64 | Architecture::I386)
}

/// Read ELF file and extract `.text` bytes (preferred) or first SHF_EXECINSTR section.
pub fn load_elf_text(path: &Path) -> Result<ElfText, ElfError> {
    let data = fs::read(path)?;
    let file = object::File::parse(&*data).map_err(|e| ElfError::Object(e.to_string()))?;
    let arch = file.architecture();
    if !arch_ok(arch) {
        return Err(ElfError::UnsupportedArch(arch));
    }
    let is_64 = file.is_64();
    let path_s = path.display().to_string();
    let architecture = format!("{arch:?}");

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
        });
    }

    Err(ElfError::NoText)
}

/// Load ELF `.text` and lift as x86 opcode stream (v1.8).
pub fn lift_elf_text(path: &Path, fn_name: &str) -> Result<(ElfText, Module), ElfError> {
    let text = load_elf_text(path)?;
    let mut module = lift_x86_32_at(&text.bytes, fn_name, text.vma)?;
    module.source = Some(format!("{}:{}", text.path, text.section_name));
    module.text_vma = Some(text.vma);
    Ok((text, module))
}
