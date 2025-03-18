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
    PhysicalPageNumber,
    memory_map::MemoryMap,
    mmu::{self, PageTable, PageTableEntry, PageTableEntryFlags},
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
    debug_println!("Setting up MMU with sv39 paging...");

    // Get a mutable reference to the root page table.
    let root_page_table = unsafe { &mut *&raw mut ROOT_PAGE_TABLE };

    // Clear the root page table to ensure all entries start as invalid.
    root_page_table.clear();

    // Create the recursive mapping for the root page table at index 511. This
    // allows the page tables to be accessed as virtual memory after paging is
    // enabled.
    let root_page_table_physical_address = &raw const ROOT_PAGE_TABLE as *const _ as usize;
    let root_physical_page_number =
        PhysicalPageNumber::from_physical_address(root_page_table_physical_address);

    debug_println!(
        "Root page table physical address is {:#x}.",
        root_page_table_physical_address
    );

    debug_println!(
        "Root physical page number is {:#x}.",
        root_physical_page_number.raw_ppn()
    );

    let mut recursive_entry = PageTableEntry::new();
    recursive_entry.set_valid(true);
    recursive_entry.set_ppn(root_physical_page_number);

    // Identity map the .text, .data, .bss, .rodata, and stack sections.
    unsafe extern "C" {
        static _text_begin: usize;
        static _text_end: usize;
        static _data_begin: usize;
        static _data_end: usize;
        static _bss_begin: usize;
        static _bss_end: usize;
        static _rodata_begin: usize;
        static _rodata_end: usize;
        static _stack_begin: usize;
        static _stack_end: usize;
    }

    let text_begin = unsafe { &_text_begin as *const _ as usize };
    let text_end = unsafe { &_text_end as *const _ as usize };
    let data_begin = unsafe { &_data_begin as *const _ as usize };
    let data_end = unsafe { &_data_end as *const _ as usize };
    let bss_begin = unsafe { &_bss_begin as *const _ as usize };
    let bss_end = unsafe { &_bss_end as *const _ as usize };
    let rodata_begin = unsafe { &_rodata_begin as *const _ as usize };
    let rodata_end = unsafe { &_rodata_end as *const _ as usize };
    let stack_begin_address = unsafe { &_stack_begin as *const _ as usize };
    let stack_end_address = unsafe { &_stack_end as *const _ as usize };

    // Identity map the .text section with the executable flag.
    let mut text_flags = PageTableEntryFlags::default();
    text_flags.set_executable(true);

    let text_start_ppn = PhysicalPageNumber::from_physical_address(text_begin);
    let text_end_ppn = PhysicalPageNumber::from_physical_address(text_end);

    mmu::identity_map_range(
        root_page_table,
        text_start_ppn,
        text_end_ppn,
        &text_flags,
        physical_memory_allocator,
    );

    // Identity map the .data section with readable and writable flags.
    let mut data_flags = PageTableEntryFlags::default();
    data_flags.set_readable(true);
    data_flags.set_writable(true);

    let data_start_ppn = PhysicalPageNumber::from_physical_address(data_begin);
    let data_end_ppn = PhysicalPageNumber::from_physical_address(data_end);

    mmu::identity_map_range(
        root_page_table,
        data_start_ppn,
        data_end_ppn,
        &data_flags,
        physical_memory_allocator,
    );

    // Identity map the .rodata section with the readable flag.
    let mut rodata_flags = PageTableEntryFlags::default();
    rodata_flags.set_readable(true);

    let rodata_start_ppn = PhysicalPageNumber::from_physical_address(rodata_begin);
    let rodata_end_ppn = PhysicalPageNumber::from_physical_address(rodata_end);

    mmu::identity_map_range(
        root_page_table,
        rodata_start_ppn,
        rodata_end_ppn,
        &rodata_flags,
        physical_memory_allocator,
    );

    // Identity map the .bss section with readable and writable flags.
    let mut bss_flags = PageTableEntryFlags::default();
    bss_flags.set_readable(true);
    bss_flags.set_writable(true);

    let bss_start_ppn = PhysicalPageNumber::from_physical_address(bss_begin);
    let bss_end_ppn = PhysicalPageNumber::from_physical_address(bss_end);

    mmu::identity_map_range(
        root_page_table,
        bss_start_ppn,
        bss_end_ppn,
        &bss_flags,
        physical_memory_allocator,
    );

    // Identity map the stack data with readable and writable flags.
    let mut stack_page_flags = PageTableEntryFlags::default();
    stack_page_flags.set_readable(true);
    stack_page_flags.set_writable(true);

    let stack_start_ppn = PhysicalPageNumber::from_physical_address(stack_begin_address);
    let stack_end_ppn = PhysicalPageNumber::from_physical_address(stack_end_address);

    mmu::identity_map_range(
        root_page_table,
        stack_start_ppn,
        stack_end_ppn,
        &stack_page_flags,
        physical_memory_allocator,
    );

    // Map the first 128GiB of physical memory to the top 128GiB of virtual
    // memory. This will give the kernel the ability to access any physical
    // memory address. Importantly, this will allow the kernel to access every
    // page table we have created and will create.

    debug_println!();
    print_page_table_entries(root_page_table, 0, 2, 0);
    debug_println!();

    // Set up the satp register to enable paging. Format for RV64 with sv39:
    // - MODE (bits 63:60) = 8 for sv39
    // - ASID (bits 59:44) = 0 for now (Address Space ID)
    // - PPN (bits 43:0) = physical page number of the root page table
    let satp_value = (8usize << 60) | root_physical_page_number.raw_ppn();

    debug_println!("Setting satp register to {:#x}.", satp_value);

    // Activate the MMU by writing to the satp register.
    unsafe {
        // Flush the TLB before activating the MMU, write to satp to enable
        // paging, and flush the TLB again after enabling paging.
        core::arch::asm!(
            "csrw satp, {}",
            "sfence.vma",
            in(reg) satp_value,
            options(nomem, nostack)
        );
    }

    debug_println!("MMU activated with sv39 paging.");
}

fn print_page_table_entries(page_table: &PageTable, indent: usize, level: u8, base_vpn: usize) {
    let span = 512_usize.pow(level as u32);
    for i in 0..512 {
        // Skip index 511 only for the root page table.
        if level == 2 && i == 511 {
            continue;
        }
        let entry = page_table.get_entry(i);
        if !entry.is_valid() {
            continue;
        }
        let entry_vpn = base_vpn + i * span;
        debug_print!("{:1$} ", "", indent);
        debug_print!(
            "L{} Entry {}: VPN {:#x} -> PPN: {:#x} (Phys: {:#x}) Flags: [",
            level,
            i,
            entry_vpn,
            entry.get_ppn().raw_ppn(),
            entry.get_ppn().to_physical_address()
        );
        if entry.is_valid() {
            debug_print!("V");
        } else {
            debug_print!("-");
        }
        if entry.is_readable() {
            debug_print!("R");
        } else {
            debug_print!("-");
        }
        if entry.is_writable() {
            debug_print!("W");
        } else {
            debug_print!("-");
        }
        if entry.is_executable() {
            debug_print!("X");
        } else {
            debug_print!("-");
        }
        if entry.is_user() {
            debug_print!("U");
        } else {
            debug_print!("-");
        }
        if entry.is_global() {
            debug_print!("G");
        } else {
            debug_print!("-");
        }
        debug_println!("]");

        // If the entry is a pointer to another page table, recursively print
        // its entries.
        if !entry.is_leaf() && level > 0 {
            let child_pt = unsafe { &*(entry.get_ppn().to_physical_address() as *const PageTable) };
            print_page_table_entries(child_pt, indent + 2, level - 1, entry_vpn);
        }
    }
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
