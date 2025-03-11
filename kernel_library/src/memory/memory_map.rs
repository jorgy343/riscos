#![allow(dead_code)]

#[derive(Debug, Clone, Copy)]
pub struct MemoryMap {
    regions: [MemoryRegion; 128],
    current_size: usize,
}

impl MemoryMap {
    /// Creates a new memory map with no regions.
    /// 
    /// # Returns
    /// 
    /// A new memory map instance.
    /// 
    /// # Safety
    /// 
    /// This function is safe to call.
    pub const fn new() -> Self {
        MemoryMap {
            regions: [MemoryRegion::new(0, 0); 128],
            current_size: 0,
        }
    }

    /// Adds a new memory region to the memory map.
    ///
    /// # Parameters
    ///
    /// * `start` - The start address of the memory region.
    /// * `size` - The size of the memory region in bytes.
    ///
    /// # Safety
    ///
    /// This function assumes that there is enough space in the memory map to
    /// add the region.
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

        let mut i = 0;
        while i < self.current_size {
            let region = self.regions[i];
            let region_end = region.end();

            // Check for any kind of intersection between the region and reserved area.
            if region_end >= reserved_start && region.start < reserved_start + reserved_size {
                // Case 1: The reserved region completely contains the current region.
                if reserved_start <= region.start && reserved_start + reserved_size > region_end {
                    // Remove the region by shifting all subsequent regions one slot to the left.
                    for j in i..self.current_size - 1 {
                        self.regions[j] = self.regions[j + 1];
                    }

                    self.current_size -= 1;

                    // Don't increment i as we need to process the newly shifted element at this position.
                    continue;
                }
                // Case 2: The reserved region cuts the beginning of the region.
                else if reserved_start <= region.start && reserved_start + reserved_size <= region_end {
                    let new_start = reserved_start + reserved_size;
                    let new_size = region.size - (new_start - region.start);

                    self.regions[i].start = new_start;
                    self.regions[i].size = new_size;

                    i += 1;
                }
                // Case 3: The reserved region cuts the end of the region.
                else if reserved_start > region.start && reserved_start + reserved_size > region_end {
                    let new_size = reserved_start - region.start;
                    self.regions[i].size = new_size;

                    i += 1;
                }
                // Case 4: The reserved region is in the middle of the region.
                else if reserved_start > region.start && reserved_start + reserved_size <= region_end {
                    // Create a new region for the end part.
                    let end_part_start = reserved_start + reserved_size;
                    let end_part_size = (region_end + 1) - end_part_start;
                    let end_region = MemoryRegion::new(end_part_start, end_part_size);

                    // Update the current region to be the beginning part.
                    self.regions[i].size = reserved_start - region.start;

                    // Add the new region if there's space.
                    if self.current_size < self.regions.len() {
                        // Add the new region by inserting it at the end and we'll process it later.
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
    fn test_carve_out_reserved_region_adjacent_to_start_nothing_happens() {
        let mut memory_map = MemoryMap::new();

        // Add a region starting at 0x1000 with a size of 0x2000.
        memory_map.add_region(0x1000, 0x2000);

        // Carve out a reserved region starting at 0x0 with a size of 0x1000.
        memory_map.carve_out_region(0x0, 0x1000);

        assert_eq!(memory_map.current_size, 1);
        assert_eq!(memory_map.regions[0].start, 0x1000);
        assert_eq!(memory_map.regions[0].size, 0x2000);
    }

    #[test]
    fn test_carve_out_reserved_region_adjacent_to_end_nothing_happens() {
        let mut memory_map = MemoryMap::new();

        // Add a region starting at 0x1000 with a size of 0x2000.
        memory_map.add_region(0x1000, 0x2000);

        // Carve out a reserved region starting at 0x3000 with a size of 0x1000.
        memory_map.carve_out_region(0x3000, 0x1000);

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
