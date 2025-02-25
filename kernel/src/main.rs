#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_panic: &PanicInfo) -> ! {
    loop {}
}

global_asm!(
    "
    .global _start
    .extern _stack_start

    .section .text.boot

    _start:
        la sp, _stack_start

        la a7, 0x4442434E
        la a6, 0x0
        la a0, message
        li a1, 6
        ecall

        j .

    message:
        .asciz \"Hello!\"
    "
);
