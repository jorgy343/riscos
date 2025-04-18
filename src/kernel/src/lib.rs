#![no_std]

mod sbi;

use core::{arch::global_asm, panic::PanicInfo};

#[unsafe(no_mangle)]
pub fn kernel_main(
    hart_id: usize,
    dtb_physical_address: usize,
    root_page_table_physical_address: usize,
) -> ! {
    debug_println!("\nWelcome to the kernel! :)\n");

    debug_println!("Hart ID: {}", hart_id);
    debug_println!("DTB physical address: {:#x}", dtb_physical_address);
    debug_println!(
        "Root page table physical address: {:#x}",
        root_page_table_physical_address
    );

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    debug_println!("\n\n===== KERNEL PANIC =====");

    // Print location information if available.
    if let Some(location) = info.location() {
        debug_println!(
            "Panic occurred at {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    } else {
        debug_println!("Panic occurred at unknown location.");
    }

    // Print the panic message directly.
    debug_println!("Panic message: {}", info);

    debug_println!("=========================\n");

    // Halt the kernel.
    loop {}
}

global_asm!(
    "
    .global _kernel_entrypoint

    .extern kernel_main

    .section .text.kernel_entrypoint
    
    _kernel_entrypoint:
        // - a0 = hart_id
        // - a1 = dtb_physical_address
        // - a2 = root_page_table_physical_address
        jal kernel_main

    infinite:   // Infinite loop if kernel_main returns.
        wfi
        j infinite
    "
);
