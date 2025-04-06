//! Physical memory bump allocator implementation.
//!
//! This module provides a simple bump allocator for physical memory pages. It
//! does not support deallocation of memory pages.

use common_lib::memory::MemoryRegion;
use core::iter::Iterator;

/// Trait defining the interface for physical memory allocators.
///
/// This trait abstracts the allocation of physical memory pages, allowing for
/// different allocation strategies and making testing with mock allocators
/// easier.
pub trait PhysicalMemoryAllocator {
    /// Allocates a single page of physical memory.
    ///
    /// # Returns
    ///
    /// * `Some(*mut u8)` - If a page was successfully allocated, returns a
    ///   pointer to the page.
    /// * `None` - If there is no more memory available to allocate.
    fn allocate_page(&mut self) -> Option<*mut u8>;

    /// Returns the total amount of memory available for allocation, in bytes.
    ///
    /// # Returns
    ///
    /// The total size of all memory regions in bytes.
    fn total_memory_size(&self) -> usize;

    /// Returns the amount of memory that has been allocated so far, in bytes.
    ///
    /// # Returns
    ///
    /// The total amount of memory that has been allocated, in bytes.
    fn allocated_memory_size(&self) -> usize;

    /// Returns the amount of memory that is still available for allocation, in
    /// bytes.
    ///
    /// # Returns
    ///
    /// The total amount of memory that is still available for allocation, in
    /// bytes.
    fn available_memory_size(&self) -> usize {
        self.total_memory_size() - self.allocated_memory_size()
    }

    /// Returns an iterator over all memory regions available to the allocator.
    ///
    /// # Returns
    ///
    /// An iterator yielding all memory regions registered with this allocator.
    fn memory_regions(&self) -> impl Iterator<Item = MemoryRegion> + '_;

    /// Returns an iterator over the memory regions that have been allocated.
    ///
    /// This provides memory regions representing portions that have been
    /// allocated from the available memory.
    ///
    /// # Returns
    ///
    /// An iterator yielding memory regions representing allocated memory.
    fn allocated_regions(&self) -> impl Iterator<Item = MemoryRegion> + '_;
}

/// A simple bump allocator for physical memory.
///
/// This allocator allows allocation of physical memory pages (PPNs) using a
/// bump allocation strategy. It maintains a list of memory regions and
/// allocates pages sequentially from these regions. Deallocation is not
/// supported.
#[derive(Debug, Clone)]
pub struct PhysicalBumpAllocator {
    /// The memory regions available for allocation.
    memory_regions: [MemoryRegion; 128],

    /// The number of valid memory regions.
    region_count: usize,

    /// The current region being allocated from.
    current_region_index: usize,

    /// The next address to allocate within the current region.
    next_allocation_address: usize,
}

impl PhysicalBumpAllocator {
    pub const fn new() -> PhysicalBumpAllocator {
        PhysicalBumpAllocator {
            memory_regions: [MemoryRegion::new(0, 0); 128],
            region_count: 0,
            current_region_index: 0,
            next_allocation_address: 0,
        }
    }

    /// Resets the physical bump allocator with the provided memory regions. All
    /// current state is lost.
    ///
    /// # Parameters
    ///
    /// * `regions` - A slice of memory regions available for allocation.
    ///
    /// # Returns
    ///
    /// A new instance of PhysicalBumpAllocator.
    pub fn reset(&mut self, regions: &[MemoryRegion], region_count: usize) {
        // Copy regions into our internal array.
        let copy_count = core::cmp::min(region_count, self.memory_regions.len());
        for i in 0..copy_count {
            self.memory_regions[i] = regions[i];
        }

        self.region_count = copy_count;

        // Initialize the next allocation address if we have regions which is
        // the start of the first region.
        if copy_count > 0 {
            self.next_allocation_address = self.memory_regions[0].start;
        }
    }
}

impl PhysicalMemoryAllocator for PhysicalBumpAllocator {
    /// Allocates a single page of physical memory.
    ///
    /// This function attempts to allocate a single 4KiB page from the available
    /// memory regions. It advances through regions as needed when a region is
    /// exhausted.
    ///
    /// # Returns
    ///
    /// * `Some(*mut u8)` - If a page was successfully allocated, returns a
    ///   pointer to the page.
    /// * `None` - If there is no more memory available to allocate.
    fn allocate_page(&mut self) -> Option<*mut u8> {
        // Check if we have any regions to allocate from.
        if self.region_count == 0 {
            return None;
        }

        // Keep trying until we find a valid allocation or run out of regions.
        while self.current_region_index < self.region_count {
            let current_region = self.memory_regions[self.current_region_index];

            // Check if we've reached the end of the current region.
            let region_end_address = current_region.start + current_region.size;
            if self.next_allocation_address + 4096 > region_end_address {
                // Move to the next region.
                self.current_region_index += 1;

                // If there is another region, update the next allocation
                // address.
                if self.current_region_index < self.region_count {
                    self.next_allocation_address =
                        self.memory_regions[self.current_region_index].start;

                    continue;
                } else {
                    // No more regions available.
                    return None;
                }
            }

            // We have a valid allocation.
            let allocation_address = self.next_allocation_address;

            // Advance the allocation pointer.
            self.next_allocation_address += 4096;

            // If this allocation used the last available page in the region,
            // move to the next region.
            if self.next_allocation_address + 4096 > region_end_address {
                self.current_region_index += 1;

                if self.current_region_index < self.region_count {
                    self.next_allocation_address =
                        self.memory_regions[self.current_region_index].start;
                }
            }

            // Return the raw pointer to the allocated memory.
            return Some(allocation_address as *mut u8);
        }

        // No more memory available.
        None
    }

    /// Returns the total amount of memory available for allocation, in bytes.
    ///
    /// # Returns
    ///
    /// The total size of all memory regions in bytes.
    fn total_memory_size(&self) -> usize {
        let mut total_size = 0;
        for i in 0..self.region_count {
            total_size += self.memory_regions[i].size;
        }

        total_size
    }

    /// Returns the amount of memory that has been allocated so far, in bytes.
    ///
    /// # Returns
    ///
    /// The total amount of memory that has been allocated, in bytes.
    fn allocated_memory_size(&self) -> usize {
        let mut allocated_size = 0;

        // Sum up completely consumed regions.
        for i in 0..self.current_region_index {
            allocated_size += self.memory_regions[i].size;
        }

        // Add the partially consumed current region.
        if self.current_region_index < self.region_count {
            let current_region = self.memory_regions[self.current_region_index];
            allocated_size += self.next_allocation_address - current_region.start;
        }

        allocated_size
    }

    /// Returns an iterator over all memory regions available to the allocator.
    ///
    /// # Returns
    ///
    /// An iterator yielding all memory regions registered with this allocator.
    fn memory_regions(&self) -> impl Iterator<Item = MemoryRegion> + '_ {
        self.memory_regions.iter().take(self.region_count).copied()
    }

    /// Returns an iterator over the memory regions that have been allocated.
    ///
    /// This provides memory regions representing portions that have been
    /// allocated from the available memory.
    ///
    /// # Returns
    ///
    /// An iterator yielding memory regions representing allocated memory.
    fn allocated_regions(&self) -> impl Iterator<Item = MemoryRegion> {
        self.memory_regions
            .iter()
            .take(self.current_region_index)
            .copied()
            .chain(
                // Only include the current region if some memory has been
                // allocated from it.
                self.memory_regions
                    .get(self.current_region_index)
                    .filter(|_| self.current_region_index < self.region_count)
                    .map(|region| {
                        let allocated_size = self.next_allocation_address - region.start;
                        if allocated_size > 0 {
                            Some(MemoryRegion::new(region.start, allocated_size))
                        } else {
                            None
                        }
                    })
                    .flatten(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_allocator() {
        let regions = [
            MemoryRegion::new(0x1000, 0x4000),
            MemoryRegion::new(0x10000, 0x8000),
        ];

        let mut allocator = PhysicalBumpAllocator::new();
        allocator.reset(&regions, regions.len());

        assert_eq!(allocator.region_count, 2);
        assert_eq!(allocator.current_region_index, 0);
        assert_eq!(allocator.next_allocation_address, 0x1000);
        assert_eq!(allocator.total_memory_size(), 0x4000 + 0x8000);
    }

    #[test]
    fn test_allocate_single_page() {
        let regions = [MemoryRegion::new(0x1000, 0x4000)];

        let mut allocator = PhysicalBumpAllocator::new();
        allocator.reset(&regions, regions.len());

        let ptr = allocator.allocate_page().unwrap();
        assert_eq!(ptr as usize, 0x1000);
        assert_eq!(allocator.next_allocation_address, 0x2000);
        assert_eq!(allocator.allocated_memory_size(), 0x1000);
    }

    #[test]
    fn test_allocate_multiple_pages() {
        let regions = [MemoryRegion::new(0x1000, 0x3000)];

        let mut allocator = PhysicalBumpAllocator::new();
        allocator.reset(&regions, regions.len());

        let ptr1 = allocator.allocate_page().unwrap();
        let ptr2 = allocator.allocate_page().unwrap();
        let ptr3 = allocator.allocate_page().unwrap();

        assert_eq!(ptr1 as usize, 0x1000);
        assert_eq!(ptr2 as usize, 0x2000);
        assert_eq!(ptr3 as usize, 0x3000);

        // The region should now be exhausted.
        assert_eq!(allocator.current_region_index, 1);
    }

    #[test]
    fn test_allocate_across_regions() {
        let regions = [
            MemoryRegion::new(0x1000, 0x1000),  // Just one page.
            MemoryRegion::new(0x10000, 0x2000), // Two pages.
        ];

        let mut allocator = PhysicalBumpAllocator::new();
        allocator.reset(&regions, regions.len());

        // Allocate from the first region.
        let ptr1 = allocator.allocate_page().unwrap();
        assert_eq!(ptr1 as usize, 0x1000);

        // The first region is now exhausted, next allocation should come from
        // the second region.
        let ptr2 = allocator.allocate_page().unwrap();
        assert_eq!(ptr2 as usize, 0x10000);

        let ptr3 = allocator.allocate_page().unwrap();
        assert_eq!(ptr3 as usize, 0x11000);

        // The second region should now be exhausted.
        assert_eq!(allocator.current_region_index, 2);
    }

    #[test]
    fn test_allocate_until_exhausted() {
        let regions = [
            MemoryRegion::new(0x1000, 0x1000), // One page.
        ];

        let mut allocator = PhysicalBumpAllocator::new();
        allocator.reset(&regions, regions.len());

        // Allocate the only page.
        let ptr = allocator.allocate_page().unwrap();
        assert_eq!(ptr as usize, 0x1000);

        // Try to allocate again, should be None.
        assert!(allocator.allocate_page().is_none());
    }

    #[test]
    fn test_available_memory_size_new_allocator() {
        let regions = [
            MemoryRegion::new(0x1000, 0x4000),
            MemoryRegion::new(0x10000, 0x8000),
        ];

        let mut allocator = PhysicalBumpAllocator::new();
        allocator.reset(&regions, regions.len());

        // Total memory should be 0x4000 + 0x8000 = 0xC000.
        //
        // No memory allocated yet, so available should equal total.
        assert_eq!(allocator.total_memory_size(), 0xC000);
        assert_eq!(allocator.allocated_memory_size(), 0);
        assert_eq!(allocator.available_memory_size(), 0xC000);
    }

    #[test]
    fn test_available_memory_size_after_allocation() {
        let regions = [MemoryRegion::new(0x1000, 0x4000)];

        let mut allocator = PhysicalBumpAllocator::new();
        allocator.reset(&regions, regions.len());

        // Total memory is 0x4000, nothing allocated yet.
        assert_eq!(allocator.available_memory_size(), 0x4000);

        // Allocate one page (0x1000).
        let _ptr = allocator.allocate_page().unwrap();
        assert_eq!(allocator.allocated_memory_size(), 0x1000);
        assert_eq!(allocator.available_memory_size(), 0x3000);

        // Allocate two more pages (0x2000).
        let _ptr2 = allocator.allocate_page().unwrap();
        let _ptr3 = allocator.allocate_page().unwrap();
        assert_eq!(allocator.allocated_memory_size(), 0x3000);
        assert_eq!(allocator.available_memory_size(), 0x1000);
    }

    #[test]
    fn test_available_memory_size_across_regions() {
        let regions = [
            MemoryRegion::new(0x1000, 0x1000),  // Just one page.
            MemoryRegion::new(0x10000, 0x2000), // Two pages.
        ];

        let mut allocator = PhysicalBumpAllocator::new();
        allocator.reset(&regions, regions.len());

        // Total memory is 0x1000 + 0x2000 = 0x3000.
        assert_eq!(allocator.total_memory_size(), 0x3000);
        assert_eq!(allocator.available_memory_size(), 0x3000);

        // Allocate from the first region.
        let _ptr1 = allocator.allocate_page().unwrap();
        assert_eq!(allocator.allocated_memory_size(), 0x1000);
        assert_eq!(allocator.available_memory_size(), 0x2000);

        // Allocate from the second region.
        let _ptr2 = allocator.allocate_page().unwrap();
        assert_eq!(allocator.allocated_memory_size(), 0x2000);
        assert_eq!(allocator.available_memory_size(), 0x1000);

        // Allocate the final page.
        let _ptr3 = allocator.allocate_page().unwrap();
        assert_eq!(allocator.allocated_memory_size(), 0x3000);
        assert_eq!(allocator.available_memory_size(), 0);
    }

    #[test]
    fn test_available_memory_size_when_exhausted() {
        let regions = [
            MemoryRegion::new(0x1000, 0x1000), // One page.
        ];

        let mut allocator = PhysicalBumpAllocator::new();
        allocator.reset(&regions, regions.len());

        // Initially 0x1000 bytes available.
        assert_eq!(allocator.available_memory_size(), 0x1000);

        // Allocate the only page.
        let _ptr = allocator.allocate_page().unwrap();

        // No more memory available.
        assert_eq!(allocator.allocated_memory_size(), 0x1000);
        assert_eq!(allocator.available_memory_size(), 0);
        assert!(allocator.allocate_page().is_none());
    }
}
