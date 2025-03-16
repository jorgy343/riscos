#![no_std]
#![no_main]

mod dtb;
mod sbi;

use core::panic::PanicInfo;
use core::sync::atomic::Ordering;
use core::{arch::global_asm, sync::atomic::fence};
use dtb::{
    adjust_memory_map_from_reserved_regions_in_dtb, populate_memory_map_from_dtb,
    walk_memory_reservation_entries, walk_structure_block,
};
use kernel_library::memory::{
    memory_map::MemoryMap,
    mmu::PageTable,
    physical_memory_allocator::{PhysicalBumpAllocator, PhysicalMemoryAllocator},
};

static mut ROOT_PAGE_TABLE: PageTable = PageTable::new();
static mut MEMORY_MAP: MemoryMap = MemoryMap::new();

/// Main kernel entry point. This function is called as early as possible in the
/// boot process.
///
/// # Arguments
/// * `hart_id` - The hardware thread ID.
/// * `dtb_address` - Pointer to the device tree blob.
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(hart_id: usize, dtb_address: usize) -> ! {
    debug_println!("\nKernel booting on hart ID: {}\n", hart_id);

    let dtb_header = get_dtb_header(dtb_address);

    print_reserved_memory_regions(dtb_header);
    print_dtb_structure(dtb_header);

    let memory_map = create_memory_map(dtb_header);
    print_memory_regions(memory_map);

    create_physical_memory_allocator(memory_map);

    loop {}
}

fn get_dtb_header(dtb_address: usize) -> &'static dtb::DtbHeader {
    // Convert the DTB address to a DtbHeader reference.
    let dtb_header = unsafe { &*(dtb_address as *const dtb::DtbHeader) };

    debug_println!("DTB found at address: {:#x}", dtb_address);
    debug_println!("{:#?}", dtb_header);
    debug_println!();

    dtb_header
}

fn print_reserved_memory_regions(dtb_header: &dtb::DtbHeader) {
    debug_println!("Reserved Memory Regions:");
    walk_memory_reservation_entries(dtb_header, |entry| {
        debug_println!("  {:#?}", entry);
    });

    debug_println!();
}

fn print_dtb_structure(dtb_header: &dtb::DtbHeader) {
    walk_structure_block(
        dtb_header,
        |node, depth| {
            for _ in 0..depth {
                debug_print!("  ");
            }

            debug_println!("Node: {}", node.name);
        },
        |_, property, cell_info, depth| {
            for _ in 0..depth {
                debug_print!("  ");
            }

            if property.name == "#address-cells" {
                debug_println!(
                    "  Property: {} ({})",
                    property.name,
                    property.get_property_data_as_u32()
                );
            } else if property.name == "#size-cells" {
                debug_println!(
                    "  Property: {} ({})",
                    property.name,
                    property.get_property_data_as_u32()
                );
            } else if property.name == "reg" {
                debug_println!("  Property: {}", property.name);

                property.get_property_data_as_reg(cell_info, |address, size| {
                    for _ in 0..depth {
                        debug_print!("  ");
                    }

                    debug_println!(
                        "    Reg entry: address {:#x}-{:#x}, size {:#x}",
                        address,
                        address + size,
                        size
                    );
                });
            } else {
                debug_println!("  Property: {}", property.name);
            }
        },
    );

    debug_println!();
}

fn create_memory_map(dtb_header: &dtb::DtbHeader) -> &mut MemoryMap {
    unsafe extern "C" {
        static _kernel_begin: usize;
        static _kernel_end_exclusive: usize;
    }

    let kernel_start = unsafe { &_kernel_begin as *const _ as usize };
    let kernel_end_exclusive = unsafe { &_kernel_end_exclusive as *const _ as usize };

    // Populate the memory map using information from the device tree blob.
    let memory_map = unsafe { &mut *&raw mut MEMORY_MAP };

    populate_memory_map_from_dtb(memory_map, dtb_header);
    adjust_memory_map_from_reserved_regions_in_dtb(memory_map, dtb_header);

    let kernel_size = kernel_end_exclusive - kernel_start;
    debug_println!(
        "Kernel memory region: {:#x}-{:#x}, size: {:#x}",
        kernel_start,
        kernel_end_exclusive - 1,
        kernel_size
    );

    // Carve out the kernel memory region from the memory map.
    memory_map.carve_out_region(kernel_start, kernel_size);

    memory_map
}

fn print_memory_regions(memory_map: &mut MemoryMap) {
    debug_println!("Usable memory regions:");

    memory_map.walk_regions(|region| {
        debug_println!(
            "  Memory region: {:#x}-{:#x}, size: {:#x}",
            region.start,
            region.end(),
            region.size
        );
    });
}

fn create_physical_memory_allocator(memory_map: &mut MemoryMap) {
    // Create a physical memory allocator.
    let mut physical_memory_allocator =
        PhysicalBumpAllocator::new(memory_map.get_regions(), memory_map.get_region_count());

    let allocated_memory = physical_memory_allocator.total_memory_size();

    debug_println!();
    debug_println!(
        "Created a physical memory allocator with {:#x} bytes of memory.",
        allocated_memory
    );
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
        // For now, all secondary harts (hart ID != 0) will loop forever. The
        // riscv spec, requires that there be at least one hart that has hart ID
        // 0.
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
        
        // a0 = hart_id a1 = Device Tree Blob address
        jal kernel_main

    infinite:   // Infinite loop if kernel_main returns.
        wfi
        j infinite

    secondary_hart:
        wfi
        j secondary_hart
    "
);
