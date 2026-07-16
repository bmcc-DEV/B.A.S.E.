/*
 * base_virt_ndjson — QEMU TCG plugin → Specter Live NDJSON
 *
 * Build: make -C base-virt/plugin
 * Use:   -plugin ./libbase_virt_ndjson.so,outfile=/tmp/t.ndjson,io_only=1
 *
 * Args:
 *   outfile=PATH   NDJSON output (default: base_virt_trace.ndjson)
 *   io_only=0|1    only MMIO/IO phys (default: 1)
 *   base=HEX       optional phys base filter
 *   size=HEX       optional size with base (default 0x1000 if base set)
 *
 * Honesty: ≠ OS turnkey — só instrumentação de evidência.
 */
#include "qemu_plugin_api.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

QEMU_PLUGIN_EXPORT int qemu_plugin_version = 3;

static FILE *out_fp;
static int io_only = 1;
static int have_range;
static uint64_t range_base;
static uint64_t range_size = 0x1000;
static uint64_t seq;

static void vcpu_mem(unsigned int vcpu_index, qemu_plugin_meminfo_t info,
                     uint64_t vaddr, void *userdata) {
    (void)vcpu_index;
    (void)userdata;
    if (!out_fp) {
        return;
    }

    struct qemu_plugin_hwaddr *hw = qemu_plugin_get_hwaddr(info, vaddr);
    uint64_t addr = vaddr;
    int is_io = 0;
    if (hw) {
        is_io = qemu_plugin_hwaddr_is_io(hw) ? 1 : 0;
        addr = qemu_plugin_hwaddr_phys_addr(hw);
    }

    if (io_only && hw && !is_io) {
        return;
    }
    if (have_range) {
        if (addr < range_base || addr >= range_base + range_size) {
            return;
        }
    }

    int store = qemu_plugin_mem_is_store(info) ? 1 : 0;
    uint64_t ts = seq++;
    if (store) {
        fprintf(out_fp,
                "{\"op\":\"mmio_write\",\"addr\":\"0x%" PRIx64
                "\",\"ts_ns\":%" PRIu64
                ",\"meta\":{\"phys\":\"1\",\"io\":\"%d\"}}\n",
                addr, ts, is_io);
    } else {
        fprintf(out_fp,
                "{\"op\":\"mmio_read\",\"addr\":\"0x%" PRIx64
                "\",\"ts_ns\":%" PRIu64
                ",\"meta\":{\"phys\":\"1\",\"io\":\"%d\"}}\n",
                addr, ts, is_io);
    }
    if ((ts & 0xff) == 0) {
        fflush(out_fp);
    }
}

static void vcpu_tb_trans(qemu_plugin_id_t id, struct qemu_plugin_tb *tb) {
    (void)id;
    size_t n = qemu_plugin_tb_n_insns(tb);
    for (size_t i = 0; i < n; i++) {
        struct qemu_plugin_insn *insn = qemu_plugin_tb_get_insn(tb, i);
        qemu_plugin_register_vcpu_mem_cb(insn, vcpu_mem, QEMU_PLUGIN_CB_NO_REGS,
                                         QEMU_PLUGIN_MEM_RW, NULL);
    }
}

static int parse_u64(const char *s, uint64_t *out) {
    if (!s || !*s) {
        return -1;
    }
    char *end = NULL;
    unsigned long long v = strtoull(s, &end, 0);
    if (end == s) {
        return -1;
    }
    *out = (uint64_t)v;
    return 0;
}

QEMU_PLUGIN_EXPORT int qemu_plugin_install(qemu_plugin_id_t id,
                                           const qemu_info_t *info, int argc,
                                           char **argv) {
    (void)info;
    const char *outfile = "base_virt_trace.ndjson";

    for (int i = 0; i < argc; i++) {
        char *eq = strchr(argv[i], '=');
        if (!eq) {
            continue;
        }
        *eq = '\0';
        const char *k = argv[i];
        const char *v = eq + 1;
        if (strcmp(k, "outfile") == 0) {
            outfile = v;
        } else if (strcmp(k, "io_only") == 0) {
            io_only = (strcmp(v, "0") != 0 && strcmp(v, "false") != 0);
        } else if (strcmp(k, "base") == 0) {
            if (parse_u64(v, &range_base) == 0) {
                have_range = 1;
            }
        } else if (strcmp(k, "size") == 0) {
            parse_u64(v, &range_size);
        }
        *eq = '=';
    }

    out_fp = fopen(outfile, "w");
    if (!out_fp) {
        fprintf(stderr, "base_virt_ndjson: cannot open %s\n", outfile);
        return -1;
    }
    setvbuf(out_fp, NULL, _IOLBF, 0);

    qemu_plugin_register_vcpu_tb_trans_cb(id, vcpu_tb_trans);
    return 0;
}
