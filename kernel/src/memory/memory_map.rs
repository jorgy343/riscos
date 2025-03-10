#![allow(dead_code)]

use crate::dtb::{DtbHeader, CellInfo, walk_structure_block};

#[derive(Debug, Clone, Copy)]
pub struct MemoryMap {
    regions: [MemoryRegion; 128],
    current_size: usize,
}

impl MemoryMap {
    pub const fn new() -> Self {
        MemoryMap {
            regions: [MemoryRegion::new(0, 0); 128],
            current_size: 0,
        }
    }

    pub const fn add_region(&mut self, start: usize, end: usize) {
        self.regions[self.current_size] = MemoryRegion { start, end };
        self.current_size += 1;
    }

    pub fn walk_regions(&self, callback: impl Fn(&MemoryRegion))
    {
        for i in 0..self.current_size {
            callback(&self.regions[i]);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: usize,
    pub end: usize,
}

impl MemoryRegion {
    pub const fn new(start: usize, end: usize) -> Self {
        MemoryRegion { start, end }
    }

    pub const fn size(&self) -> usize {
        self.end - self.start
    }
}

/// Populates a memory map with memory regions described in the Device Tree
/// Blob.
///
/// This function walks through the DTB structure, looking for memory nodes and
/// their "reg" properties which describe available memory ranges. Each memory
/// region is aligned to 4KiB boundaries before being added to the memory map:
/// - Start addresses are rounded up to the nearest 4KiB boundary.
/// - End addresses are rounded down to the nearest 4KiB boundary.
/// - Regions that become smaller than 4KiB after alignment are discarded.
///
/// # Parameters
///
/// * `memory_map` - The memory map to populate with memory regions.
/// * `dtb_header` - Reference to the Device Tree Blob header.
pub fn populate_memory_map_from_dtb(memory_map: &mut MemoryMap, dtb_header: &DtbHeader) {
    // Constants for 4KiB alignment in the Sv39 paging scheme.
    const PAGE_SIZE: usize = 4096;
    const PAGE_MASK: usize = !(PAGE_SIZE - 1);
    
    walk_structure_block(
        dtb_header,
        |_, _| {},
        |node, property, cells_info, _depth| {
            if node.name != "memory" && !node.name.starts_with("memory@") {
                return;
            }
                        
            // Only process "reg" properties that are inside memory nodes.
            if property.name == "reg" {
                // Extract memory regions from the reg property.
                property.get_property_data_as_reg(&cells_info, |address, size| {
                    let region_start_address = address as usize;
                    let region_end_address = region_start_address + size as usize;
                    
                    // Align the start address up to the next 4KiB boundary.
                    let aligned_start_address = (region_start_address + PAGE_SIZE - 1) & PAGE_MASK;
                    
                    // Align the end address down to the previous 4KiB boundary.
                    let aligned_end_address = region_end_address & PAGE_MASK;
                    
                    // Only add regions that are at least 4KiB in size after
                    // alignment.
                    if aligned_end_address > aligned_start_address {
                        let region_size = aligned_end_address - aligned_start_address;
                        
                        if region_size >= PAGE_SIZE {
                            memory_map.add_region(aligned_start_address, aligned_end_address);
                        }
                    }
                });
            }
        }
    );
}
