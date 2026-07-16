# base_virt_ndjson (TCG plugin)

Opt-in. Não entra no `cargo build` / CI default.

```bash
make -C base-virt/plugin
# → libbase_virt_ndjson.so

qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 256M -nographic \
  -kernel Image \
  -plugin ./base-virt/plugin/libbase_virt_ndjson.so,outfile=/tmp/live.ndjson,io_only=1 \
  -qmp unix:/tmp/base-qmp.sock,server,nowait
```

Depois: `base virt ingest /tmp/live.ndjson` · `base virt qmp --socket /tmp/base-qmp.sock status`

Honesty: ≠ OS turnkey.
