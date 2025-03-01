#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

use sbi::debug_console::sbi_debug_console_write;

pub mod sbi;

#[panic_handler]
fn panic(_panic: &PanicInfo) -> ! {
    loop {}
}

/// Main kernel entry point. This function is called as early as possible in the boot process.
/// 
/// # Arguments
/// * `hart_id` - The hardware thread ID.
/// * `dtb_addr` - Pointer to the device tree blob.
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(_hart_id: usize, _dtb_addr: *const u8) -> ! {
    sbi_debug_console_write(b"Hello, world!\n");

    loop {}
}

global_asm!(
    "
    .global _start
    .extern _stack_start
    .extern kernel_main

    .section .text.boot
    
    _start:
        // Load stack pointer from the linker script symbol.
        la sp, _stack_start
        
        // a0 = hart_id
        // a1 = Device Tree Blob address
        jal kernel_main

    infinite:   // Infinite loop if kernel_main returns.
        j infinite
    "
);
