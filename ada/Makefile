# FELIX | PHOENIX — Ada/SPARK 2022 Compiler Arc (M0-M5) + Hardware Benchmarks
# Toolchain: alr (alire) -> gnat_native + gprbuild + gnatprove

ALIRE_BIN := $(HOME)/.local/share/alire/toolchains
GNAT_DIR  := $(wildcard $(ALIRE_BIN)/gnat_native_*)
GPR_DIR   := $(wildcard $(ALIRE_BIN)/gprbuild_*)
PATH      := $(GNAT_DIR)/bin:$(GPR_DIR)/bin:$(PATH)

GPRBUILD  ?= gprbuild
GNATPROVE ?= $(HOME)/.local/share/alire/extras/gnatprove-16.1.0/bin/gnatprove
MODE      ?= Release

all:
	PATH="$(PATH)" $(GPRBUILD) -p -P felix.gpr -XMODE=$(MODE)

test: all
	./test_harness.sh

bench: all
	./obj/run_benchmarks 10000 1000

prove:
	PATH="$(PATH)" $(GNATPROVE) -P felix.gpr --mode=check_all

clean:
	rm -rf obj

.PHONY: all test bench prove clean
