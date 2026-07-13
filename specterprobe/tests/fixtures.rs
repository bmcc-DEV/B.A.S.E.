/// Test fixture: minimal ARM64 binary blobs for snapshot tests.

/// A minimal ARM64 function: `add x0, x0, #1; ret`
/// Encoding: 0x91000400 (add x0, x0, #1), 0xD65F03C0 (ret)
pub fn minimal_add_one() -> Vec<u8> {
    vec![
        0x00, 0x04, 0x00, 0x91, // add x0, x0, #1
        0xC0, 0x03, 0x5F, 0xD6, // ret
    ]
}

/// A function with prologue: `stp fp,lr,[sp,#-16]!; mov x0,#42; ldp fp,lr,[sp],#16; ret`
pub fn function_with_prologue() -> Vec<u8> {
    vec![
        0xFD, 0x7B, 0xBF, 0xA9, // stp x29, x30, [sp, #-16]!
        0x00, 0x00, 0x80, 0xD2, // mov x0, #0
        0xE0, 0x03, 0x00, 0xAA, // mov x0, x0 (nop)
        0xFD, 0x7B, 0xC1, 0xA8, // ldp x29, x30, [sp], #16
        0xC0, 0x03, 0x5F, 0xD6, // ret
    ]
}

/// A function with conditional branch: `mov w0, #0; b #4; ret`
pub fn conditional_branch() -> Vec<u8> {
    vec![
        0x00, 0x00, 0x80, 0x52, // mov w0, #0
        0x00, 0x00, 0x00, 0x14, // b #0 (infinite loop to self)
        0xC0, 0x03, 0x5F, 0xD6, // ret
    ]
}

/// Stub ARM32 binary in ELF format
pub fn arm32_stub() -> Vec<u8> {
    let mut elf = vec![
        0x7F, 0x45, 0x4C, 0x46, // ELF magic
        0x01, 0x28,             // 32-bit, ARM
        0x01, 0x00, 0x00, 0x00, // ELF version
    ];
    elf.extend_from_slice(&[
        0x00, 0x00, 0xA0, 0xE3, // mov r0, #0
        0x1E, 0xFF, 0x2F, 0xE1, // bx lr
    ]);
    elf
}
