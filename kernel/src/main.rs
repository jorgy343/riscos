#![no_std]
#![no_main]

mod sbi;
mod memory;
mod dtb;

use core::arch::global_asm;
use core::panic::PanicInfo;

use dtb::walk_memory_reservation_entries;
use memory::bump_allocator::BumpAllocator;
use memory::mmu::PageTable;

static mut ROOT_PAGE_TABLE: PageTable = PageTable::new();
static mut BUMP_ALLOCATOR: Option<BumpAllocator> = None;

/// Main kernel entry point. This function is called as early as possible in the boot process.
/// 
/// # Arguments
/// * `hart_id` - The hardware thread ID.
/// * `dtb_addr` - Pointer to the device tree blob.
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(_hart_id: usize, dtb_addr: *const u8) -> ! {
    debug_println!("Hello, world!");

    // Convert the DTB address to a DtbHeader pointer
    let dtb_header = dtb_addr as *const dtb::DtbHeader;

    // Safely access the DTB header
    if !dtb_header.is_null() {
        // Access fields via the pointer
        let header = unsafe { &*dtb_header };

        debug_println!("DTB found at address: {:#x}", dtb_addr as usize);
        debug_println!("{:#?}", header);

        walk_memory_reservation_entries(dtb_header, |entry| {
            debug_println!("{:#?}", entry);
        });
    } else {
        debug_println!("Invalid DTB address provided.");
    }

    loop {}
}

#[panic_handler]
fn panic(_panic: &PanicInfo) -> ! {
    loop {}
}

global_asm!(
    "
    .global _start

    .extern _bss_begin
    .extern _bss_end
    .extern _stack_end
    .extern kernel_main

    .section .text.kernel_boot
    
    _start:
        // For now, all secondary harts (hart ID != 0) will loop forever. The riscv spec,
        // requires that there be at least one hart that has hart ID 0.
        bnez a0, secondary_hart

        // Disable all supervisor level interrupts globally.
        csrci sstatus, 2

        // Load stack pointer from the linker script symbol.
        la sp, _stack_end

        // Zero out the .bss section.
        la t0, _bss_begin
        la t1, _bss_end
    
        bss_clear_loop:
            bgeu t0, t1, bss_clear_end  // If t0 >= t1, exit the loop.
            sd zero, (t0)               // Write 8 bytes of zeros at address t0.
            addi t0, t0, 8              // Increment t0 by 8 bytes.
            j bss_clear_loop            // Repeat the loop.

        bss_clear_end:
        
        // a0 = hart_id
        // a1 = Device Tree Blob address
        jal kernel_main

    infinite:   // Infinite loop if kernel_main returns.
        wfi
        j infinite

    secondary_hart:
        wfi
        j secondary_hart
    "
);
