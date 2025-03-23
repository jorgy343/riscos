use crate::{debug_print, debug_println};
use boot_lib::memory::{
    PhysicalPageNumber, VirtualPageNumber,
    mmu::{PageTable, PageTableEntryFlags, allocate_level_2_vpn, identity_map_range},
    physical_memory_allocator::PhysicalMemoryAllocator,
};

pub fn setup_mmu(
    root_page_table_physical_address: usize,
    root_page_table: &mut PageTable,
    physical_memory_allocator: &mut impl PhysicalMemoryAllocator,
) {
    debug_println!("Setting up MMU with sv39 paging...");

    // Create the recursive mapping for the root page table at index 511. This
    // allows the page tables to be accessed as virtual memory after paging is
    // enabled.
    let root_page_table_ppn =
        PhysicalPageNumber::from_physical_address(root_page_table_physical_address);

    debug_println!(
        "Root page table physical address is {:#x}.",
        root_page_table_physical_address
    );

    debug_println!(
        "Root physical page number is {:#x}.",
        root_page_table_ppn.raw_ppn()
    );

    // Identity map the .text, .data, .bss, .rodata, and stack sections.
    unsafe extern "C" {
        static _boot_text_start: usize;
        static _boot_text_end: usize;
        static _boot_data_start: usize;
        static _boot_data_end: usize;
        static _boot_bss_start: usize;
        static _boot_bss_end: usize;
        static _boot_rodata_start: usize;
        static _boot_rodata_end: usize;
        static _boot_stack_start: usize;
        static _boot_stack_end: usize;
    }

    let boot_text_start = unsafe { &_boot_text_start as *const _ as usize };
    let boot_text_end = unsafe { &_boot_text_end as *const _ as usize };
    let boot_data_start = unsafe { &_boot_data_start as *const _ as usize };
    let boot_data_end = unsafe { &_boot_data_end as *const _ as usize };
    let boot_bss_start = unsafe { &_boot_bss_start as *const _ as usize };
    let boot_bss_end = unsafe { &_boot_bss_end as *const _ as usize };
    let boot_rodata_start = unsafe { &_boot_rodata_start as *const _ as usize };
    let boot_rodata_end = unsafe { &_boot_rodata_end as *const _ as usize };
    let boot_stack_start = unsafe { &_boot_stack_start as *const _ as usize };
    let boot_stack_end = unsafe { &_boot_stack_end as *const _ as usize };

    // Identity map the .text section with the executable flag.
    let mut text_flags = PageTableEntryFlags::default();
    text_flags.set_executable(true);

    let text_start_ppn = PhysicalPageNumber::from_physical_address(boot_text_start);
    let text_end_ppn = PhysicalPageNumber::from_physical_address(boot_text_end);

    debug_println!(
        "Mapping .text section: {:#x}-{:#x}",
        boot_text_start,
        boot_text_end
    );

    identity_map_range(
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

    let data_start_ppn = PhysicalPageNumber::from_physical_address(boot_data_start);
    let data_end_ppn = PhysicalPageNumber::from_physical_address(boot_data_end);

    identity_map_range(
        root_page_table,
        data_start_ppn,
        data_end_ppn,
        &data_flags,
        physical_memory_allocator,
    );

    // Identity map the .rodata section with the readable flag.
    let mut rodata_flags = PageTableEntryFlags::default();
    rodata_flags.set_readable(true);

    let rodata_start_ppn = PhysicalPageNumber::from_physical_address(boot_rodata_start);
    let rodata_end_ppn = PhysicalPageNumber::from_physical_address(boot_rodata_end);

    identity_map_range(
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

    let bss_start_ppn = PhysicalPageNumber::from_physical_address(boot_bss_start);
    let bss_end_ppn = PhysicalPageNumber::from_physical_address(boot_bss_end);

    identity_map_range(
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

    let stack_start_ppn = PhysicalPageNumber::from_physical_address(boot_stack_start);
    let stack_end_ppn = PhysicalPageNumber::from_physical_address(boot_stack_end);

    identity_map_range(
        root_page_table,
        stack_start_ppn,
        stack_end_ppn,
        &stack_page_flags,
        physical_memory_allocator,
    );

    map_physical_memory(root_page_table);

    debug_println!();
    print_page_table_entries(root_page_table, 2, 0);
    debug_println!();

    // Set up the satp register to enable paging. Format for RV64 with sv39:
    // - MODE (bits 63:60) = 8 for sv39
    // - ASID (bits 59:44) = 0 for now (Address Space ID)
    // - PPN (bits 43:0) = physical page number of the root page table
    let satp_value = (8usize << 60) | root_page_table_ppn.raw_ppn();

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

/// Map the first 128GiB of physical memory to the top 128GiB of virtual memory.
/// This will give the kernel the ability to access any physical memory address.
/// Importantly, this will allow the kernel to access every page table we have
/// created and will create.
fn map_physical_memory(root_page_table: &mut PageTable) {
    // Define the number of gigabytes to map (128GiB).
    const GIGABYTES_TO_MAP: usize = 128;

    // Create page table entry flags for this direct mapping section. These
    // pages should be readable and writable, but not executable. Also mark
    // these pages as global since they will be part of every address space.
    let mut direct_mapping_flags = PageTableEntryFlags::default();
    direct_mapping_flags.set_readable(true);
    direct_mapping_flags.set_writable(true);
    direct_mapping_flags.set_global(true);

    debug_println!(
        "Mapping first {}GiB of physical memory to top of virtual memory.",
        GIGABYTES_TO_MAP
    );

    // Map each gigabyte individually.
    for gib_index in 0..GIGABYTES_TO_MAP {
        // Calculate the virtual page number for this mapping. For the top
        // 128GiB, we start at index (512 - 128) = 384.
        let vpn2_index = 512 - GIGABYTES_TO_MAP + gib_index;
        let virtual_page_number = VirtualPageNumber::from_raw_virtual_page_number(vpn2_index << 18);

        // The physical page number for this mapping is just the index * 1GiB
        // since we're mapping 0..128GiB to the top of the address space.
        let physical_page_number =
            PhysicalPageNumber::from_raw_physical_page_number(gib_index << 18);

        // Create the mapping using the gigapage mapper.
        let mapping_result = allocate_level_2_vpn(
            root_page_table,
            virtual_page_number,
            physical_page_number,
            &direct_mapping_flags,
        );

        if !mapping_result {
            debug_println!(
                "  Failed to map 1GiB at Virtual [{:#x}] -> Physical [{:#x}]",
                virtual_page_number.to_virtual_address(),
                physical_page_number.to_physical_address()
            );
        }
    }

    debug_println!("  Direct mapping of physical memory complete.");
    debug_println!();
}

fn print_page_table_entries(page_table: &PageTable, level: u8, base_vpn: usize) {
    let indent = (2 - level) as usize * 2;
    let span = 512_usize.pow(level as u32);

    for i in 0..512 {
        let entry = page_table.get_entry(i);
        if !entry.is_valid() {
            continue;
        }

        let entry_vpn = base_vpn + i * span;

        debug_print!("{:1$}", "", indent);
        debug_print!(
            "L{} Entry {}: VPN {:#007x} (Virt: {:#016x}) -> PPN: {:#011x} (Phys: {:#016x}) Flags: [",
            level,
            i,
            entry_vpn,
            entry_vpn << 12,
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

        // If the entry is a pointer to another page table, recursively print its entries.
        if !entry.is_leaf() && level > 0 {
            let child_page_table =
                unsafe { &*(entry.get_ppn().to_physical_address() as *const PageTable) };

            print_page_table_entries(child_page_table, level - 1, entry_vpn);
        }
    }
}
