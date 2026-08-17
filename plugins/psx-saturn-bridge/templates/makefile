# Scaffold makefile — PS1→Saturn port via SaturnRingLib
# Copiar para o teu projeto Saturn e ajustar SOURCES / CD_NAME.

SRL_MAX_TEXTURES = 100
SRL_MODE = NTSC
SRL_HIGH_RES = 0
SRL_FRAMERATE = 1
SRL_MAX_CD_BACKGROUND_JOBS = 1
SRL_MAX_CD_FILES = 256
SRL_MAX_CD_RETRIES = 5
SRL_MALLOC_METHOD = TLSF

SRL_USE_SGL_SOUND_DRIVER = 1
SRL_ENABLE_FREQ_ANALYSIS = 0

SGL_MAX_VERTICES = 2500
SGL_MAX_POLYGONS = 1500
SGL_MAX_EVENTS = 1
SGL_MAX_WORKS = 1

CD_NAME = PsxSaturnPort
BUILD_DROP = ./BuildDrop

# Default: symlink do plugin; override com SRL_ROOT
SRL_INSTALL_ROOT ?= $(abspath ../SaturnRingLib)
ifeq ($(wildcard $(SRL_INSTALL_ROOT)/saturnringlib/shared.mk),)
  SRL_INSTALL_ROOT ?= /media/bruno/Bruno/SaturnRingLib
endif

SOURCES = $(patsubst ./%,%,$(shell find src/ -name '*.c' 2>/dev/null))
SOURCES += $(patsubst ./%,%,$(shell find src/ -name '*.cxx' 2>/dev/null))

SDK_ROOT = $(SRL_INSTALL_ROOT)/saturnringlib
include $(SDK_ROOT)/shared.mk
