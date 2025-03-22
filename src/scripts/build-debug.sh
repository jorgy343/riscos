#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# Print commands and their arguments as they are executed.
set -x

cd "$(dirname "$0")/.."

cargo build \
    --target riscv64gc-unknown-none-elf \
    --package boot

riscv64-unknown-elf-ld \
    --gc-sections \
    --no-print-gc-sections \
    -T linker.ld \
    -o target/riscv64gc-unknown-none-elf/debug/boot.elf \
    target/riscv64gc-unknown-none-elf/debug/libboot.a

riscv64-unknown-elf-objcopy \
    -O binary \
    target/riscv64gc-unknown-none-elf/debug/boot.elf \
    target/riscv64gc-unknown-none-elf/debug/kernel.bin