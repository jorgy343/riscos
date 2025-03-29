#![no_std]

mod dtb;
mod sbi;
mod startup;

use boot_lib::memory::{mmu::PageTable, physical_memory_allocator::PhysicalMemoryAllocator};
use core::arch::{asm, global_asm};
use core::panic::PanicInfo;
use startup::{
    dtb::{get_dtb_header, print_dtb_structure, print_reserved_memory_regions},
    memory::{create_memory_map, create_physical_memory_allocator, print_memory_regions},
    mmu::setup_mmu,
};

/// Primary entry point for the boot process after any low level assembly is
/// finished up. This function is called as early as possible in the boot
/// process.
///
/// # Arguments
///
/// * `hart_id` - The hardware thread ID that called this function.
/// * `dtb_address` - Pointer to the device tree blob.
#[unsafe(no_mangle)]
pub fn boot_main(hart_id: usize, dtb_physical_address: usize) -> ! {
    debug_println!("\nKernel booting on hart ID: {}\n", hart_id);

    let dtb_header = get_dtb_header(dtb_physical_address);

    print_reserved_memory_regions(dtb_header);
    print_dtb_structure(dtb_header);

    let mut memory_map = create_memory_map(dtb_header);
    print_memory_regions(&mut memory_map);

    let mut physical_memory_allocator = create_physical_memory_allocator(&mut memory_map);

    let root_page_table_pointer = physical_memory_allocator
        .allocate_page()
        .expect("Failed to allocate page for root page table.");

    let mut root_page_table = unsafe { &mut *(root_page_table_pointer as *mut PageTable) };
    root_page_table.clear();

    setup_mmu(
        root_page_table_pointer as usize,
        &mut root_page_table,
        &mut physical_memory_allocator,
    );

    // Jump to the kernel at virtual address 0xFFFF_FFC0_0000_0000.
    // Pass hart_id in a0, dtb_address in a1, and root_page_table_pointer in a2.
    unsafe {
        asm!(
            "
            mv a0, {0}
            mv a1, {1}
            mv a2, {2}
            li t0, 0xFFFFFFC000000000
            jr t0
            ",
            in(reg) hart_id,
            in(reg) dtb_physical_address,
            in(reg) root_page_table_pointer as usize,
            options(noreturn)
        );
    }
}

#[panic_handler]
fn panic(_panic: &PanicInfo) -> ! {
    loop {}
}

global_asm!(
    "
    .global _boot_entrypoint

    .extern _boot_bss_start
    .extern _boot_bss_end
    .extern _boot_stack_end
    .extern boot_main

    .section .text.boot_entrypoint
    
    _boot_entrypoint:
        // For now, all secondary harts (hart ID != 0) will loop forever. The
        // riscv spec requires that there be at least one hart that has hart ID
        // 0.
        bnez a0, secondary_hart

        // Disable all supervisor level interrupts globally.
        csrci sstatus, 2

        // Load stack pointer from the linker script symbol.
        la sp, _boot_stack_end

        // Zero out the .bss section.
        la t0, _boot_bss_start
        la t1, _boot_bss_end
    
        bss_clear_loop:
            bgeu t0, t1, bss_clear_end  // If t0 >= t1, exit the loop.
            sd zero, (t0)               // Write 8 bytes of zeros at address t0.
            addi t0, t0, 8              // Increment t0 by 8 bytes.
            j bss_clear_loop            // Repeat the loop.

        bss_clear_end:
        
        // - a0 = hart_id
        // - a1 = Device Tree Blob address
        jal boot_main

    infinite:   // Infinite loop if boot_main returns.
        wfi
        j infinite

    secondary_hart:
        wfi
        j secondary_hart
    "
);
