#!/bin/bash

# Make build scripts executable.
chmod +x /workspaces/riscos/src/scripts/build-debug.sh
chmod +x /workspaces/riscos/src/scripts/build-release.sh

# Check if the build dependencies are available.
command -v riscv64-unknown-elf-ld >/dev/null 2>&1 || { echo "RISC-V toolchain not installed"; exit 1; }
command -v qemu-system-riscv64 >/dev/null 2>&1 || { echo "QEMU RISC-V not installed"; exit 1; }

[ -f /opt/opensbi/share/opensbi/lp64/generic/firmware/fw_jump.bin ] || { echo "OpenSBI firmware not found"; exit 1; }