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

    pub const fn add_region(&mut self, start: usize, size: usize) {
        self.regions[self.current_size] = MemoryRegion::new(start, size);
        self.current_size += 1;
    }

    /// Removes or adjusts memory regions in this memory map that overlap with a
    /// specified reserved memory region.
    ///
    /// This function handles four possible overlap cases between a memory
    /// region and the reserved area:
    /// 1. Complete containment - The reserved region fully contains a memory
    ///    region, in which case the memory region is completely removed.
    /// 2. Start overlap - The reserved region overlaps the start of a memory
    ///    region, in which case the memory region is trimmed to begin after the
    ///    reserved area.
    /// 3. End overlap - The reserved region overlaps the end of a memory
    ///    region, in which case the memory region is trimmed to end before the
    ///    reserved area.
    /// 4. Middle overlap - The reserved region is in the middle of a memory
    ///    region, in which case the memory region is split into two separate
    ///    regions.
    ///
    /// The function aligns the reserved region boundaries to 4KiB page
    /// boundaries, rounding the start address down and the end address up to
    /// ensure all reserved memory is properly excluded.
    ///
    /// # Parameters
    ///
    /// * `reserved_start` - The start address of the reserved memory region.
    /// * `reserved_size` - The size of the reserved memory region in bytes.
    ///
    /// # Side Effects
    ///
    /// This function modifies this memory map by potentially removing regions,
    /// adjusting region boundaries, or adding new regions when splitting is required.
    pub fn carve_out_region(&mut self, reserved_start: usize, reserved_size: usize) {
        // Skip if the reserved region is invalid.
        if reserved_size == 0 {
            return;
        }

        // Calculate the end address (exclusive) from start and size.
        let reserved_end = reserved_start + reserved_size;

        // Constants for 4KiB alignment in the Sv39 paging scheme.
        const PAGE_SIZE: usize = 4096;
        const PAGE_MASK: usize = !(PAGE_SIZE - 1);

        // Round start down and end up to be generous with the boundary.
        let aligned_reserved_start = reserved_start & PAGE_MASK;
        let aligned_reserved_end = (reserved_end + PAGE_SIZE - 1) & PAGE_MASK;

        let mut i = 0;
        while i < self.current_size {
            let region = self.regions[i];
            let region_end = region.end();

            // Check for any kind of intersection between the region and
            // reserved area. Since region_end is now inclusive, we need to use
            // >= for the comparison with aligned_reserved_start.
            if region_end >= aligned_reserved_start && region.start < aligned_reserved_end {
                // Case 1: The reserved region completely contains the current
                // region.
                if aligned_reserved_start <= region.start && aligned_reserved_end >= region_end + 1
                {
                    // Remove the region by shifting all subsequent regions one
                    // slot to the left.
                    for j in i..self.current_size - 1 {
                        self.regions[j] = self.regions[j + 1];
                    }

                    self.current_size -= 1;

                    // Don't increment i as we need to process the newly shifted
                    // element at this position.
                    continue;
                }
                // Case 2: The reserved region cuts the beginning of the region.
                else if aligned_reserved_start <= region.start
                    && aligned_reserved_end < region_end + 1
                {
                    let new_start = aligned_reserved_end;

                    // Add 1 to region_end because it's now inclusive.
                    let new_size = (region_end + 1) - new_start;

                    self.regions[i].start = new_start;
                    self.regions[i].size = new_size;

                    i += 1;
                }
                // Case 3: The reserved region cuts the end of the region.
                else if aligned_reserved_start > region.start
                    && aligned_reserved_end >= region_end + 1
                {
                    let new_size = aligned_reserved_start - region.start;
                    self.regions[i].size = new_size;

                    i += 1;
                }
                // Case 4: The reserved region is in the middle of the region.
                else if aligned_reserved_start > region.start
                    && aligned_reserved_end < region_end + 1
                {
                    // Create a new region for the end part.
                    let end_part_start = aligned_reserved_end;
                    // Add 1 to region_end because it's now inclusive
                    let end_part_size = (region_end + 1) - aligned_reserved_end;
                    let end_region = MemoryRegion::new(end_part_start, end_part_size);

                    // Update the current region to be the beginning part.
                    self.regions[i].size = aligned_reserved_start - region.start;

                    // Add the new region if there's space.
                    if self.current_size < self.regions.len() {
                        // Add the new region by inserting it at the end and
                        // we'll process it later.
                        self.regions[self.current_size] = end_region;
                        self.current_size += 1;
                    }

                    i += 1;
                }
            } else {
                // No intersection, continue to the next region.
                i += 1;
            }
        }
    }

    pub fn walk_regions(&self, callback: impl Fn(&MemoryRegion)) {
        for i in 0..self.current_size {
            callback(&self.regions[i]);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: usize,
    pub size: usize,
}

impl MemoryRegion {
    pub const fn new(start: usize, size: usize) -> Self {
        MemoryRegion { start, size }
    }

    // Returns the inclusive end address of the memory region.
    // If the size is zero, returns zero.
    pub const fn end(&self) -> usize {
        if self.size == 0 {
            return 0;
        }

        // Subtract 1 from start + size to get the inclusive end address.
        self.start + self.size - 1
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
                    let original_start = address as usize;
                    let original_size = size as usize;

                    // Align the start address up to the next 4KiB boundary.
                    let aligned_start = (original_start + PAGE_SIZE - 1) & PAGE_MASK;

                    // Calculate how much the alignment changed the start
                    // position.
                    let start_adjustment = aligned_start - original_start;

                    // Adjust the size by subtracting the start adjustment.
                    let adjusted_size = if start_adjustment < original_size {
                        original_size - start_adjustment
                    } else {
                        // If start adjustment exceeded original size, region
                        // vanishes.
                        0
                    };

                    // Align the size down to a multiple of 4KiB.
                    let aligned_size = adjusted_size & PAGE_MASK;

                    // Only add regions that are at least 4KiB in size after
                    // alignment.
                    if aligned_size >= PAGE_SIZE {
                        memory_map.add_region(aligned_start, aligned_size);
                    }
                });
            }
        },
    );
}

/// Adjusts a memory map by removing regions marked as reserved in the Device
/// Tree Blob.
///
/// This function walks through the DTB structure looking for the
/// "reserved-memory" node and its children. For each child node with a "reg"
/// property, it extracts the address and size information of the reserved
/// memory region and removes it from the available memory map by calling
/// `remove_reserved_memory_region`.
///
/// Reserved memory regions are used by firmware, bootloaders, or other system
/// components and should not be used by the operating system. This ensures that
/// the memory map only contains memory that is safe to use.
///
/// # Parameters
///
/// * `memory_map` - The memory map to adjust by removing reserved regions.
/// * `dtb_header` - Reference to the Device Tree Blob header containing the
///   reserved memory information.
///
/// # Side Effects
///
/// This function modifies the provided memory map by potentially removing
/// regions, adjusting region boundaries, or adding new regions when splitting
/// is required.
pub fn adjust_memory_map_from_reserved_regions_in_dtb(
    memory_map: &mut MemoryMap,
    dtb_header: &DtbHeader,
) {
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
                    let reserved_size = size as usize;

                    memory_map.carve_out_region(reserved_start, reserved_size);
                });
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_region() {
        let mut memory_map = MemoryMap::new();

        // Add a region starting at 0x1000 with a size of 0x2000.
        memory_map.add_region(0x1000, 0x2000);

        assert_eq!(memory_map.current_size, 1);
        assert_eq!(memory_map.regions[0].start, 0x1000);
        assert_eq!(memory_map.regions[0].size, 0x2000);
    }

    #[test]
    fn test_carve_out_region_case_complete_containment() {
        let mut memory_map = MemoryMap::new();
        
        // Add a region that will be completely reserved.
        memory_map.add_region(4096, 4096);
        
        // Carve out a reserved region that completely covers the added region.
        memory_map.carve_out_region(4096, 4096);
        
        // Expect that the memory region is removed.
        assert_eq!(memory_map.current_size, 0);
    }

    #[test]
    fn test_carve_out_region_case_start_overlap() {
        let mut memory_map = MemoryMap::new();
        
        // Add a region from 4096 with size 8192.
        memory_map.add_region(4096, 8192);
        
        // Reserved region overlaps the start.
        // For a 4KiB page, aligned_reserved_start = 4096 and aligned_reserved_end = 8192.
        memory_map.carve_out_region(4096, 4096);
        
        // Expect the region now starts at 8192 and the new size is 4096.
        assert_eq!(memory_map.current_size, 1);
        assert_eq!(memory_map.regions[0].start, 8192);
        assert_eq!(memory_map.regions[0].size, 4096);
    }

    #[test]
    fn test_carve_out_region_case_end_overlap() {
        let mut memory_map = MemoryMap::new();
        
        // Add a region from 4096 with size 8192.
        memory_map.add_region(4096, 8192);
        
        // Reserved region overlaps the end.
        // With reserved_start = 8192 and reserved_size = 4096, aligned_reserved_start = 8192.
        memory_map.carve_out_region(8192, 4096);
        
        // Expect the region remains from 4096 to 8191 (size of 4096).
        assert_eq!(memory_map.current_size, 1);
        assert_eq!(memory_map.regions[0].start, 4096);
        assert_eq!(memory_map.regions[0].size, 4096);
    }

    #[test]
    fn test_carve_out_region_case_middle_overlap() {
        let mut memory_map = MemoryMap::new();
        
        // Add a region from 4096 with size 12288.
        memory_map.add_region(4096, 12288);
        
        // Reserved region is in the middle.
        // With reserved_start = 8192 and reserved_size = 4096, aligned_reserved_start = 8192, aligned_reserved_end = 12288.
        memory_map.carve_out_region(8192, 4096);
        
        // Expect the original region is split into two:
        // First region: from 4096 to 8191 (4096 bytes).
        // Second region: from 12288 to 16383 (4096 bytes).
        assert_eq!(memory_map.current_size, 2);
        
        // Verify the first region.
        assert_eq!(memory_map.regions[0].start, 4096);
        assert_eq!(memory_map.regions[0].size, 4096);
        
        // Verify the second region.
        assert_eq!(memory_map.regions[1].start, 12288);
        assert_eq!(memory_map.regions[1].size, 4096);
    }

    #[test]
    fn test_carve_out_region_no_reserved_size() {
        let mut memory_map = MemoryMap::new();
        
        // Add a region.
        memory_map.add_region(4096, 4096);
        
        // Call carve_out_region with reserved_size 0.
        memory_map.carve_out_region(4096, 0);
        
        // Expect no changes.
        assert_eq!(memory_map.current_size, 1);
        assert_eq!(memory_map.regions[0].start, 4096);
        assert_eq!(memory_map.regions[0].size, 4096);
    }
}
