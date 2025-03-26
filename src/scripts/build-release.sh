#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# Print commands and their arguments as they are executed.
set -x

cd "$(dirname "$0")/.."

export RUSTFLAGS="-C relocation-model=pic --emit=asm"

cargo build \
    --target riscv64gc-unknown-none-elf \
    --package kernel \
    --release

export RUSTFLAGS="-C relocation-model=static --emit=asm"

cargo build \
    --target riscv64gc-unknown-none-elf \
    --package boot \
    --release

riscv64-unknown-elf-ld \
    --gc-sections \
    --no-print-gc-sections \
    -T kernel/linker.ld \
    -o target/riscv64gc-unknown-none-elf/release/libkernel.elf \
    target/riscv64gc-unknown-none-elf/release/libkernel.a

riscv64-unknown-elf-objcopy \
    -O binary \
    target/riscv64gc-unknown-none-elf/release/libkernel.elf \
    target/riscv64gc-unknown-none-elf/release/libkernel.bin

KERNEL_SIZE=$(stat -c %s target/riscv64gc-unknown-none-elf/release/libkernel.bin)

riscv64-unknown-elf-ld \
    --gc-sections \
    --no-print-gc-sections \
    -T boot/linker.ld \
    --defsym=_kernel_size=$KERNEL_SIZE \
    -o target/riscv64gc-unknown-none-elf/release/libboot.elf \
    target/riscv64gc-unknown-none-elf/release/libboot.a

riscv64-unknown-elf-objcopy \
    -O binary \
    target/riscv64gc-unknown-none-elf/release/libboot.elf \
    target/riscv64gc-unknown-none-elf/release/libboot.bin

cat target/riscv64gc-unknown-none-elf/release/libboot.bin \
    target/riscv64gc-unknown-none-elf/release/libkernel.bin \
    > target/riscv64gc-unknown-none-elf/release/kernel.bin

echo "BUILD SUCCESSFUL"