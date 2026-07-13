//! Amiga CD32 — Example: generate HardwareSpec from MMIO observations.
//!
//! This example simulates the MMIO behavior of the Amiga CD32 chipset
//! (Alice, Lisa, Paula, Gayle) and runs the B.A.S.E. inference pipeline.
//!
//! Usage:
//!   cargo run --example amiga_cd32

use base_core::inference::extraction::{MmioAccess, MmioAccessType};
use base_core::inference::generate_spec;

fn main() {
    tracing_subscriber::fmt::init();

    // Amiga CD32 chipset MMIO map (from public documentation):
    //   Alice (video): 0xDFF000-0xDFF0FF
    //   Lisa (display): 0xDFF100-0xDFF1FF
    //   Paula (audio/IO): 0xDFF000-0xDFF0FF (shared with Alice)
    //   Gayle (chip select): 0xDA8000-0xDAFFFF
    //   CIAA: 0xBFE001
    //   CIAB: 0xBFD000

    let accesses = vec![
        // === INIT SEQUENCE ===
        // Disable DMA first
        MmioAccess { address: 0xDFF096, value: Some(0x0000), access_type: MmioAccessType::Write, function_name: "init_dma".into(), instruction_addr: 10 },
        // Wait for blitter idle
        MmioAccess { address: 0xDFF002, value: Some(0x0000), access_type: MmioAccessType::Read, function_name: "init_dma".into(), instruction_addr: 20 },
        // Set color palette (color[0] = black)
        MmioAccess { address: 0xDFF180, value: Some(0x0000), access_type: MmioAccessType::Write, function_name: "init_palette".into(), instruction_addr: 30 },
        // Color[1] = white
        MmioAccess { address: 0xDFF182, value: Some(0x0FFF), access_type: MmioAccessType::Write, function_name: "init_palette".into(), instruction_addr: 32 },
        // Set bitplane pointers (bpl0pt = framebuffer address)
        MmioAccess { address: 0xDFF0E0, value: Some(0x00200000), access_type: MmioAccessType::Write, function_name: "video_init".into(), instruction_addr: 40 },
        // Set bitplane modulo
        MmioAccess { address: 0xDFF0E4, value: Some(80), access_type: MmioAccessType::Write, function_name: "video_init".into(), instruction_addr: 44 },
        // Set display window start
        MmioAccess { address: 0xDFF08E, value: Some(0x2C81), access_type: MmioAccessType::Write, function_name: "video_init".into(), instruction_addr: 48 },
        // Set display window stop
        MmioAccess { address: 0xDFF090, value: Some(0xF4C1), access_type: MmioAccessType::Write, function_name: "video_init".into(), instruction_addr: 52 },
        // Set DDFSTRT (display data fetch start)
        MmioAccess { address: 0xDFF092, value: Some(0x0038), access_type: MmioAccessType::Write, function_name: "video_init".into(), instruction_addr: 56 },
        // Set DDFSTOP (display data fetch stop)
        MmioAccess { address: 0xDFF094, value: Some(0x00D0), access_type: MmioAccessType::Write, function_name: "video_init".into(), instruction_addr: 60 },

        // === AUDIO INIT ===
        // Set audio period (channel 0)
        MmioAccess { address: 0xDFF0A6, value: Some(124), access_type: MmioAccessType::Write, function_name: "audio_init".into(), instruction_addr: 70 },
        // Set audio volume (channel 0)
        MmioAccess { address: 0xDFF0A8, value: Some(64), access_type: MmioAccessType::Write, function_name: "audio_init".into(), instruction_addr: 72 },
        // Set audio length (channel 0)
        MmioAccess { address: 0xDFF0A4, value: Some(4096), access_type: MmioAccessType::Write, function_name: "audio_init".into(), instruction_addr: 74 },
        // Set audio pointer (channel 0)
        MmioAccess { address: 0xDFF0A0, value: Some(0x00300000), access_type: MmioAccessType::Write, function_name: "audio_init".into(), instruction_addr: 76 },

        // === COPPER LIST ===
        // Write copper instruction: wait for vertical blank
        MmioAccess { address: 0xDFF080, value: Some(0x01FC), access_type: MmioAccessType::Write, function_name: "copper".into(), instruction_addr: 80 },
        MmioAccess { address: 0xDFF082, value: Some(0xFFFE), access_type: MmioAccessType::Write, function_name: "copper".into(), instruction_addr: 82 },
        // Move color[2] = red
        MmioAccess { address: 0xDFF184, value: Some(0x0F00), access_type: MmioAccessType::Write, function_name: "copper".into(), instruction_addr: 84 },

        // === ENABLE DMA ===
        // Enable bitplane DMA + copper DMA + audio DMA
        MmioAccess { address: 0xDFF096, value: Some(0x8320), access_type: MmioAccessType::Write, function_name: "enable_dma".into(), instruction_addr: 90 },

        // === POLLING LOOP (vertical blank) ===
        MmioAccess { address: 0xDFF004, value: Some(0x0000), access_type: MmioAccessType::Read, function_name: "vblank_wait".into(), instruction_addr: 100 },
        MmioAccess { address: 0xDFF004, value: Some(0x0000), access_type: MmioAccessType::Read, function_name: "vblank_wait".into(), instruction_addr: 104 },
        MmioAccess { address: 0xDFF004, value: Some(0x8000), access_type: MmioAccessType::Read, function_name: "vblank_wait".into(), instruction_addr: 108 },

        // === INTERRUPT ===
        // Read interrupt request register (INTREQR)
        MmioAccess { address: 0xDFF01C, value: Some(0x0008), access_type: MmioAccessType::Read, function_name: "irq_handler".into(), instruction_addr: 120 },
        // Acknowledge interrupt (INTREQ = INTREQ)
        MmioAccess { address: 0xDFF09C, value: Some(0x0008), access_type: MmioAccessType::Write, function_name: "irq_handler".into(), instruction_addr: 124 },
    ];

    tracing::info!("Generated {} MMIO accesses for Amiga CD32 chipset", accesses.len());

    let spec = generate_spec(&accesses, "Amiga CD32 (simulated)");
    let yaml = spec.to_yaml().expect("Should serialize to YAML");

    println!("=== HardwareSpec ===");
    println!("Blocks: {}", spec.blocks.len());
    println!("Overall confidence: {:.2}", spec.confidence);
    println!();
    println!("{}", yaml);

    // Write to file
    std::fs::write("examples/amiga_cd32/hardware_spec.yaml", &yaml).ok();
    println!("Written to examples/amiga_cd32/hardware_spec.yaml");
}
