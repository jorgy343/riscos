#![no_std]
#![no_main]

mod dtb;
mod sbi;

use core::arch::global_asm;
use core::panic::PanicInfo;
use dtb::{
    adjust_memory_map_from_reserved_regions_in_dtb, populate_memory_map_from_dtb,
    walk_memory_reservation_entries, walk_structure_block,
};
use kernel_library::memory::{
    PhysicalPageNumber, VirtualPageNumber,
    memory_map::MemoryMap,
    mmu::{self, PageTable, PageTableEntry},
    physical_memory_allocator::{PhysicalBumpAllocator, PhysicalMemoryAllocator},
};

static mut MEMORY_MAP: MemoryMap = MemoryMap::new();
static mut PHYSICAL_MEMORY_ALLOCATOR: PhysicalBumpAllocator = PhysicalBumpAllocator::new();
static mut ROOT_PAGE_TABLE: PageTable = PageTable::new();

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

    let physical_memory_allocator = create_physical_memory_allocator(memory_map);

    setup_mmu(physical_memory_allocator);

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

    debug_println!();
}

fn create_physical_memory_allocator(memory_map: &mut MemoryMap) -> &mut PhysicalBumpAllocator {
    // Create a physical memory allocator.
    let physical_memory_allocator = unsafe { &mut *&raw mut PHYSICAL_MEMORY_ALLOCATOR };
    physical_memory_allocator.reset(memory_map.get_regions(), memory_map.get_region_count());

    debug_println!(
        "Created a physical memory allocator with {:#x} free memory.",
        physical_memory_allocator.total_memory_size()
    );

    debug_println!();

    physical_memory_allocator
}

fn setup_mmu(physical_memory_allocator: &mut impl PhysicalMemoryAllocator) {
    unsafe extern "C" {
        static _kernel_begin: usize;
        static _kernel_end_exclusive: usize;
    }

    debug_println!("Setting up MMU with sv39 paging...");

    // Get a mutable reference to the root page table.
    let root_page_table = unsafe { &mut *&raw mut ROOT_PAGE_TABLE };

    // Clear the root page table to ensure all entries start as invalid.
    root_page_table.clear();

    // Get physical addresses of kernel start and end.
    let kernel_start_physical_address = unsafe { &_kernel_begin as *const _ as usize };
    let kernel_end_exclusive_physical_address =
        unsafe { &_kernel_end_exclusive as *const _ as usize };
    let kernel_size_bytes = kernel_end_exclusive_physical_address - kernel_start_physical_address;

    // Create the recursive mapping for the root page table at index 511. This
    // allows the page tables to be accessed as virtual memory after paging is
    // enabled.
    let root_page_table_physical_address = &raw const root_page_table as usize;
    let root_physical_page_number =
        PhysicalPageNumber::from_physical_address(root_page_table_physical_address);

    let mut recursive_entry = PageTableEntry::new();
    recursive_entry.set_valid(true);
    recursive_entry.set_readable(true);
    recursive_entry.set_writable(true);
    recursive_entry.set_ppn(root_physical_page_number);

    // Install the recursive mapping at index 511 (last entry).
    root_page_table.set_entry(511, recursive_entry);

    debug_println!(
        "Created recursive mapping at index 511 with PPN: {:#x}",
        root_physical_page_number.raw_ppn()
    );

    // Identity map the kernel memory region. This ensures the kernel keeps
    // working when we activate the MMU.
    let mut bytes_mapped = 0;
    let mut page_count = 0;

    while bytes_mapped < kernel_size_bytes {
        let current_physical_address = kernel_start_physical_address + bytes_mapped;

        // For identity mapping, virtual address equals physical address.
        let virtual_page_number = VirtualPageNumber::from_virtual_address(current_physical_address);
        let physical_page_number =
            PhysicalPageNumber::from_physical_address(current_physical_address);

        // Map a single page using the allocate_vpn function from the mmu
        // module.
        if mmu::allocate_vpn(
            root_page_table,
            virtual_page_number,
            Some(physical_page_number),
            physical_memory_allocator,
        )
        .is_none()
        {
            panic!("Failed to set up memory mapping for kernel.");
        }

        // Move to the next 4 KiB page.
        bytes_mapped += 4096;
        page_count += 1;
    }

    debug_println!(
        "Identity mapped kernel memory: {} pages ({} bytes).",
        page_count,
        page_count * 4096
    );

    // TODO: Map additional required regions (e.g., MMIO regions, device
    // memory).

    // Set up the satp register to enable paging. Format for RV64 with sv39:
    // - MODE (bits 63:60) = 8 for sv39
    // - ASID (bits 59:44) = 0 for now (Address Space ID)
    // - PPN (bits 43:0) = physical page number of the root page table
    let satp_value = (8usize << 60) | root_physical_page_number.raw_ppn();

    debug_println!("Setting satp register to {:#x}.", satp_value);

    // Activate the MMU by writing to the satp register.
    unsafe {
        // Flush the TLB before activating the MMU.
        core::arch::asm!("sfence.vma", options(nomem, nostack));

        // Write to satp to enable paging with sv39 mode.
        core::arch::asm!("csrw satp, {}", in(reg) satp_value);

        // Flush the TLB again after enabling paging.
        core::arch::asm!("sfence.vma", options(nomem, nostack));
    }

    debug_println!("MMU activated with sv39 paging.");
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
