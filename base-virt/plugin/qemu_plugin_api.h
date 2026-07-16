/* Minimal QEMU TCG plugin API (subset) — resolves symbols at load time.
 * Compatible with QEMU plugin API version 3/4 (QEMU ≥ 8 / 10).
 * No glib dependency. SPDX: GPL-2.0-or-later (same as QEMU plugin interface).
 */
#ifndef BASE_VIRT_QEMU_PLUGIN_API_H
#define BASE_VIRT_QEMU_PLUGIN_API_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define QEMU_PLUGIN_EXPORT __attribute__((visibility("default")))
#define QEMU_PLUGIN_API

typedef uint64_t qemu_plugin_id_t;
typedef uint32_t qemu_plugin_meminfo_t;

struct qemu_plugin_tb;
struct qemu_plugin_insn;
struct qemu_plugin_hwaddr;

typedef struct qemu_info_t {
    const char *target_name;
    struct {
        int min;
        int cur;
    } version;
    bool system_emulation;
    union {
        struct {
            int smp_vcpus;
            int max_vcpus;
        } system;
    };
} qemu_info_t;

enum qemu_plugin_cb_flags {
    QEMU_PLUGIN_CB_NO_REGS,
    QEMU_PLUGIN_CB_R_REGS,
    QEMU_PLUGIN_CB_RW_REGS,
};

enum qemu_plugin_mem_rw {
    QEMU_PLUGIN_MEM_R = 1,
    QEMU_PLUGIN_MEM_W,
    QEMU_PLUGIN_MEM_RW,
};

enum qemu_plugin_mem_value_type {
    QEMU_PLUGIN_MEM_VALUE_U8,
    QEMU_PLUGIN_MEM_VALUE_U16,
    QEMU_PLUGIN_MEM_VALUE_U32,
    QEMU_PLUGIN_MEM_VALUE_U64,
    QEMU_PLUGIN_MEM_VALUE_U128,
};

typedef struct {
    enum qemu_plugin_mem_value_type type;
    union {
        uint8_t u8;
        uint16_t u16;
        uint32_t u32;
        uint64_t u64;
        struct {
            uint64_t low;
            uint64_t high;
        } u128;
    } data;
} qemu_plugin_mem_value;

typedef void (*qemu_plugin_vcpu_tb_trans_cb_t)(qemu_plugin_id_t id,
                                              struct qemu_plugin_tb *tb);
typedef void (*qemu_plugin_vcpu_mem_cb_t)(unsigned int vcpu_index,
                                         qemu_plugin_meminfo_t info,
                                         uint64_t vaddr, void *userdata);

QEMU_PLUGIN_API void qemu_plugin_register_vcpu_tb_trans_cb(
    qemu_plugin_id_t id, qemu_plugin_vcpu_tb_trans_cb_t cb);
QEMU_PLUGIN_API size_t qemu_plugin_tb_n_insns(const struct qemu_plugin_tb *tb);
QEMU_PLUGIN_API struct qemu_plugin_insn *
qemu_plugin_tb_get_insn(const struct qemu_plugin_tb *tb, size_t idx);
QEMU_PLUGIN_API void qemu_plugin_register_vcpu_mem_cb(
    struct qemu_plugin_insn *insn, qemu_plugin_vcpu_mem_cb_t cb,
    enum qemu_plugin_cb_flags flags, enum qemu_plugin_mem_rw rw,
    void *userdata);
QEMU_PLUGIN_API bool qemu_plugin_mem_is_store(qemu_plugin_meminfo_t info);
QEMU_PLUGIN_API struct qemu_plugin_hwaddr *
qemu_plugin_get_hwaddr(qemu_plugin_meminfo_t info, uint64_t vaddr);
QEMU_PLUGIN_API bool
qemu_plugin_hwaddr_is_io(const struct qemu_plugin_hwaddr *haddr);
QEMU_PLUGIN_API uint64_t
qemu_plugin_hwaddr_phys_addr(const struct qemu_plugin_hwaddr *haddr);

QEMU_PLUGIN_EXPORT int qemu_plugin_install(qemu_plugin_id_t id,
                                           const qemu_info_t *info, int argc,
                                           char **argv);

#endif
