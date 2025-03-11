#![allow(dead_code)]

use crate::dtb::{DtbHeader, walk_structure_block};
use core::cell::RefCell;

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

pub fn adjust_memory_map_from_reserved_regions_in_dtb(memory_map: &mut MemoryMap, dtb_header: &DtbHeader) {
    // Track if we're inside a reserved-memory node to process its children
    let inside_reserved_memory = RefCell::new(false);
    
    walk_structure_block(
        dtb_header,
        |node, depth| {
            // Check if we're entering the reserved-memory node
            if node.name == "reserved-memory" || node.name.starts_with("reserved-memory@") {
                *inside_reserved_memory.borrow_mut() = true;
            } 
            // Check if we're leaving the reserved-memory node (entering a different node at the same level)
            else if depth == 1 && *inside_reserved_memory.borrow() {
                *inside_reserved_memory.borrow_mut() = false;
            }
        },
        |_, property, cells_info, depth| {
            // Process reg properties in child nodes of reserved-memory
            if *inside_reserved_memory.borrow() && depth > 1 && property.name == "reg" {
                property.get_property_data_as_reg(&cells_info, |address, size| {
                    let reserved_start = address as usize;
                    let reserved_end = reserved_start + size as usize;
                    
                    remove_reserved_memory_region(memory_map, reserved_start, reserved_end);
                });
            }
        }
    );
}

pub fn remove_reserved_memory_region(memory_map: &mut MemoryMap, reserved_start: usize, reserved_end: usize) {
    // Skip if the reserved region is invalid.
    if reserved_end <= reserved_start {
        return;
    }

    // Constants for 4KiB alignment in the Sv39 paging scheme.
    const PAGE_SIZE: usize = 4096;
    const PAGE_MASK: usize = !(PAGE_SIZE - 1);

    // Round start down and end up to be generous with the boundary.
    let aligned_reserved_start = reserved_start & PAGE_MASK;
    let aligned_reserved_end = (reserved_end + PAGE_SIZE - 1) & PAGE_MASK;

    let mut i = 0;
    while i < memory_map.current_size {
        let region = memory_map.regions[i];
        
        // Check for any kind of intersection between the region and reserved area.
        if region.end > aligned_reserved_start && region.start < aligned_reserved_end {
            // Case 1: The reserved region completely contains the current region.
            if aligned_reserved_start <= region.start && aligned_reserved_end >= region.end {
                // Remove the region by shifting all subsequent regions one slot to the left.
                for j in i..memory_map.current_size - 1 {
                    memory_map.regions[j] = memory_map.regions[j + 1];
                }

                memory_map.current_size -= 1;
                
                // Don't increment i as we need to process the newly shifted element at this position.
                continue;
            }
            
            // Case 2: The reserved region cuts the beginning of the region.
            else if aligned_reserved_start <= region.start && aligned_reserved_end < region.end {
                memory_map.regions[i].start = aligned_reserved_end;
                i += 1;
            }
            
            // Case 3: The reserved region cuts the end of the region.
            else if aligned_reserved_start > region.start && aligned_reserved_end >= region.end {
                memory_map.regions[i].end = aligned_reserved_start;
                i += 1;
            }
            
            // Case 4: The reserved region is in the middle of the region.
            else if aligned_reserved_start > region.start && aligned_reserved_end < region.end {
                // Create a new region for the end part.
                let end_region = MemoryRegion::new(aligned_reserved_end, region.end);
                
                // Update the current region to be the beginning part.
                memory_map.regions[i].end = aligned_reserved_start;
                
                // Add the new region if there's space.
                if memory_map.current_size < memory_map.regions.len() {
                    // Add the new region by inserting it at the end and we'll process it later.
                    memory_map.regions[memory_map.current_size] = end_region;
                    memory_map.current_size += 1;
                }
                
                i += 1;
            }
        } else {
            // No intersection, continue to the next region.
            i += 1;
        }
    }
}
