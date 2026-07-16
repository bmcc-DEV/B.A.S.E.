# Specter Live — QEMU TCG plugin (opt-in; not built by cargo / CI default)
CC ?= gcc
CFLAGS ?= -O2 -fPIC -fvisibility=hidden -Wall -Wextra
LDFLAGS ?= -shared

TARGET = libbase_virt_ndjson.so

.PHONY: all clean

all: $(TARGET)

$(TARGET): base_virt_ndjson.c qemu_plugin_api.h
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ base_virt_ndjson.c

clean:
	rm -f $(TARGET)
