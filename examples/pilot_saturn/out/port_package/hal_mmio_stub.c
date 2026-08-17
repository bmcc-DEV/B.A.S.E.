/* B.A.S.E. Generated HAL — MMIO Translation Layer */
/* Target: hal_saturn_assist */

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef HOST_BUILD
/* Host smoke: never dereference fabricated MMIO addresses */
static uint32_t g_shadow_regs[1024];
#define REG32(addr) (g_shadow_regs[((uintptr_t)(addr) >> 2) & 1023u])
#else
#define REG32(addr) (*(volatile uint32_t *)(uintptr_t)(addr))
#endif

#define MMIO_TRAP_SIZE 256
static uint32_t g_trap_shadow[MMIO_TRAP_SIZE];
static uint32_t g_pio_shadow[64];

static const struct {
    uint32_t base;
    uint32_t size;
    uint32_t target_base;
    uint8_t strategy; /* 0=MMU, 1=TRAP, 2=PIO */
} mmio_translation[] = {
    { 0x25c00000, 0x0008, 0x25d00000, 1 }, /* RegisterFile_25c00 */
    { 0x25f00000, 0x0004, 0x26000000, 1 }, /* Doorbell_25f00 */
    { 0x20100000, 0x0008, 0x20200000, 1 }, /* RegisterFile_20100 */
    { 0, 0, 0, 0 }
};

static uint32_t handle_mmio_trap_read(uint32_t target, uint32_t offset) {
    (void)target;
    return g_trap_shadow[offset % MMIO_TRAP_SIZE];
}

static void handle_mmio_trap_write(uint32_t target, uint32_t offset, uint32_t value) {
    (void)target;
    g_trap_shadow[offset % MMIO_TRAP_SIZE] = value;
}

static uint32_t pio_emulation_read(uint32_t target) {
    return g_pio_shadow[(target >> 2) & 63u];
}

static void pio_emulation_write(uint32_t target, uint32_t value) {
    g_pio_shadow[(target >> 2) & 63u] = value;
}

static void gpu_reg_write(uint32_t offset, uint32_t value) {
    g_trap_shadow[offset % MMIO_TRAP_SIZE] = value;
}

uint32_t mmio_read(uint32_t addr) {
    for (int i = 0; mmio_translation[i].base != 0 || mmio_translation[i].size != 0; i++) {
        if (mmio_translation[i].size == 0) break;
        if (addr >= mmio_translation[i].base &&
            addr < mmio_translation[i].base + mmio_translation[i].size) {
            uint32_t offset = addr - mmio_translation[i].base;
            uint32_t target = mmio_translation[i].target_base + offset;
            switch (mmio_translation[i].strategy) {
                case 0: return REG32(target);
                case 1: return handle_mmio_trap_read(target, offset);
                case 2: return pio_emulation_read(target);
                default: return 0;
            }
        }
    }
    return 0;
}

void mmio_write(uint32_t addr, uint32_t value) {
    for (int i = 0; mmio_translation[i].base != 0 || mmio_translation[i].size != 0; i++) {
        if (mmio_translation[i].size == 0) break;
        if (addr >= mmio_translation[i].base &&
            addr < mmio_translation[i].base + mmio_translation[i].size) {
            uint32_t offset = addr - mmio_translation[i].base;
            uint32_t target = mmio_translation[i].target_base + offset;
            switch (mmio_translation[i].strategy) {
                case 0: REG32(target) = value; return;
                case 1: handle_mmio_trap_write(target, offset, value); return;
                case 2: pio_emulation_write(target, value); return;
            }
        }
    }
}

/* GPU Handler (Doorbell_25f00) @ 0x25f00000 */
static void handle_gpu_write(uint32_t offset, uint32_t value) {
    switch (offset) {
        case 0x0000: /* control */
            gpu_reg_write(offset, value);
            break;
        default: gpu_reg_write(offset, value); break;
    }
}

void gpu_dispatch_write(uint32_t offset, uint32_t value) {
    handle_gpu_write(offset, value);
}

