#![no_std]

mod sbi;

use core::{arch::global_asm, panic::PanicInfo};

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    debug_println!("\nWelcome to the kernel!\n");

    loop {}
}

#[panic_handler]
fn panic(_panic: &PanicInfo) -> ! {
    loop {}
}

global_asm!(
    "
    .global _kernel_entrypoint

    .extern kernel_main

    .section .text.kernel_entrypoint
    
    _kernel_entrypoint:        
        jal kernel_main

    infinite:   // Infinite loop if kernel_main returns.
        wfi
        j infinite
    "
);
