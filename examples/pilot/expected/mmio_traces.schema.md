# Schema — `--mmio-traces` (JSON or YAML)

Array de objetos `MmioAccess`:

```json
[
  {
    "address": 1073954816,
    "value": 1,
    "access_type": "write",
    "function_name": "uart_init",
    "instruction_addr": 0
  }
]
```

| Campo | Tipo | Notas |
|-------|------|-------|
| `address` | u64 | Endereço absoluto |
| `value` | u64 \| null | Presente em writes; null em reads |
| `access_type` | `"read"` \| `"write"` | lowercase |
| `function_name` | string | Contexto / símbolo |
| `instruction_addr` | u64 | PC da instrução (0 se desconhecido) |

Extensões `.json`, `.yaml`, `.yml` aceitas pela CLI.

Exemplo: `examples/pilot/mmio.json`
