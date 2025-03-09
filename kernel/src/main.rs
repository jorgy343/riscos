#![no_std]
#![no_main]

mod sbi;
mod memory;
mod dtb;

use core::arch::global_asm;
use core::panic::PanicInfo;

use dtb::{walk_memory_reservation_entries, walk_structure_block};
use memory::bump_allocator::BumpAllocator;
use memory::mmu::PageTable;

static mut ROOT_PAGE_TABLE: PageTable = PageTable::new();
static mut BUMP_ALLOCATOR: Option<BumpAllocator> = None;

/// Main kernel entry point. This function is called as early as possible in the boot process.
/// 
/// # Arguments
/// * `hart_id` - The hardware thread ID.
/// * `dtb_address` - Pointer to the device tree blob.
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(_hart_id: usize, dtb_address: usize) -> ! {
    // Convert the DTB address to a DtbHeader reference.
    let dtb_header = unsafe { &*(dtb_address as *const dtb::DtbHeader) };

    debug_println!("DTB found at address: {:#x}", dtb_address);
    debug_println!("{:#?}", dtb_header);

    walk_memory_reservation_entries(dtb_header, |entry| {
        debug_println!("{:#?}", entry);
    });

    walk_structure_block(
        dtb_header,
        |node_name, depth| {
            for _ in 0..depth {
                debug_print!("  ");
            }

            debug_println!("Node: {}", node_name);
        },
        |property, _cell_info, depth| {
            for _ in 0..depth {
                debug_print!("  ");
            }

            if property.name == "#address-cells" {
                debug_println!("  Property: {} ({})", property.name, property.parse_u32_from_property());
            } else if property.name == "#size-cells" {
                debug_println!("  Property: {} ({})", property.name, property.parse_u32_from_property());
            } else {
                debug_println!("  Property: {}", property.name);
            }
        }
    );

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
