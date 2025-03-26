#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# Print commands and their arguments as they are executed.
set -x

cd "$(dirname "$0")/.."

export RUSTFLAGS="-C relocation-model=pic --emit=asm"

cargo build \
    --target riscv64gc-unknown-none-elf \
    --package kernel

export RUSTFLAGS="-C relocation-model=static --emit=asm"

cargo build \
    --target riscv64gc-unknown-none-elf \
    --package boot

riscv64-unknown-elf-ld \
    --gc-sections \
    --no-print-gc-sections \
    -T kernel/linker.ld \
    -o target/riscv64gc-unknown-none-elf/debug/libkernel.elf \
    target/riscv64gc-unknown-none-elf/debug/libkernel.a

riscv64-unknown-elf-objcopy \
    -O binary \
    target/riscv64gc-unknown-none-elf/debug/libkernel.elf \
    target/riscv64gc-unknown-none-elf/debug/libkernel.bin

KERNEL_SIZE=$(stat -c %s target/riscv64gc-unknown-none-elf/debug/libkernel.bin)

riscv64-unknown-elf-ld \
    --gc-sections \
    --no-print-gc-sections \
    -T boot/linker.ld \
    --defsym=_kernel_size=$KERNEL_SIZE \
    -o target/riscv64gc-unknown-none-elf/debug/libboot.elf \
    target/riscv64gc-unknown-none-elf/debug/libboot.a

riscv64-unknown-elf-objcopy \
    -O binary \
    target/riscv64gc-unknown-none-elf/debug/libboot.elf \
    target/riscv64gc-unknown-none-elf/debug/libboot.bin

cat target/riscv64gc-unknown-none-elf/debug/libboot.bin \
    target/riscv64gc-unknown-none-elf/debug/libkernel.bin \
    > target/riscv64gc-unknown-none-elf/debug/kernel.bin

echo "BUILD SUCCESSFUL"