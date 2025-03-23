use crate::dtb;
use crate::{
    debug_println,
    dtb::{adjust_memory_map_from_reserved_regions_in_dtb, populate_memory_map_from_dtb},
};
use boot_lib::memory::{
    memory_map::MemoryMap,
    physical_memory_allocator::{PhysicalBumpAllocator, PhysicalMemoryAllocator},
};

pub fn create_memory_map(dtb_header: &dtb::DtbHeader) -> MemoryMap {
    unsafe extern "C" {
        static _boot_start: usize;
        static _boot_end: usize;
        static _kernel_size: usize;
    }

    let boot_start = unsafe { &_boot_start as *const _ as usize };
    let boot_end = unsafe { &_boot_end as *const _ as usize };
    let kernel_size = unsafe { &_kernel_size as *const _ as usize };

    let boot_size = boot_end - boot_start + 1;

    // Populate the memory map using information from the device tree blob.
    let mut memory_map = MemoryMap::new();

    populate_memory_map_from_dtb(&mut memory_map, dtb_header);
    adjust_memory_map_from_reserved_regions_in_dtb(&mut memory_map, dtb_header);

    // Carve out the kernel memory region from the memory map. The boot part of
    // the kernel and the kernel itself are loaded sequentially in physical
    // memory.
    memory_map.carve_out_region(boot_start, boot_size + kernel_size);

    memory_map
}

pub fn print_memory_regions(memory_map: &mut MemoryMap) {
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

pub fn create_physical_memory_allocator(
    memory_map: &mut MemoryMap,
) -> impl PhysicalMemoryAllocator {
    let mut physical_memory_allocator = PhysicalBumpAllocator::new();
    physical_memory_allocator.reset(memory_map.get_regions(), memory_map.get_region_count());

    debug_println!(
        "Created a physical memory allocator with {:#x} free memory.\n",
        physical_memory_allocator.total_memory_size()
    );

    physical_memory_allocator
}
