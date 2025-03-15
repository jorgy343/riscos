#![allow(dead_code)]

use super::{
    PhysicalPageNumber, VirtualPageNumber, physical_memory_allocator::PhysicalMemoryAllocator,
};

#[derive(Clone)]
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    /// Create a new page table with all entries cleared to zero (invalid).
    ///
    /// # Returns
    ///
    /// A new `PageTable` with all entries cleared to zero.
    pub const fn new() -> Self {
        Self {
            entries: [const { PageTableEntry::new() }; 512],
        }
    }

    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }

    pub const fn get_entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    pub const fn get_entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }

    pub const fn set_entry(&mut self, index: usize, entry: PageTableEntry) {
        self.entries[index] = entry;
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    const FLAG_VALID: u64 = 1 << 0; // V bit - entry is valid
    const FLAG_READ: u64 = 1 << 1; // R bit - readable
    const FLAG_WRITE: u64 = 1 << 2; // W bit - writable
    const FLAG_EXECUTE: u64 = 1 << 3; // X bit - executable
    const FLAG_USER: u64 = 1 << 4; // U bit - accessible to user mode
    const FLAG_GLOBAL: u64 = 1 << 5; // G bit - global mapping
    const FLAG_ACCESSED: u64 = 1 << 6; // A bit - page was accessed
    const FLAG_DIRTY: u64 = 1 << 7; // D bit - page was written to

    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn clear(&mut self) {
        self.0 = 0;
    }

    pub const fn get_ppn(&self) -> PhysicalPageNumber {
        PhysicalPageNumber::from_raw_physical_page_number(
            ((self.0 >> 10) & 0x0000_0FFF_FFFF_FFFF) as usize,
        )
    }

    pub const fn set_ppn(&mut self, ppn: PhysicalPageNumber) {
        // Clear the old PPN and set the new one.
        self.0 = (self.0 & !0x0000_003F_FFFF_FFF0)
            | ((ppn.raw_ppn() as u64 & 0x0000_0FFF_FFFF_FFFF) << 10);
    }

    pub const fn is_valid(&self) -> bool {
        self.0 & Self::FLAG_VALID != 0
    }

    pub const fn set_valid(&mut self, valid: bool) {
        if valid {
            self.0 |= Self::FLAG_VALID;
        } else {
            self.0 &= !Self::FLAG_VALID;
        }
    }

    pub const fn is_readable(&self) -> bool {
        self.0 & Self::FLAG_READ != 0
    }

    pub const fn set_readable(&mut self, readable: bool) {
        if readable {
            self.0 |= Self::FLAG_READ;
        } else {
            self.0 &= !Self::FLAG_READ;
        }
    }

    pub const fn is_writable(&self) -> bool {
        self.0 & Self::FLAG_WRITE != 0
    }

    pub const fn set_writable(&mut self, writable: bool) {
        if writable {
            self.0 |= Self::FLAG_WRITE;
        } else {
            self.0 &= !Self::FLAG_WRITE;
        }
    }

    pub const fn is_executable(&self) -> bool {
        self.0 & Self::FLAG_EXECUTE != 0
    }

    pub const fn set_executable(&mut self, executable: bool) {
        if executable {
            self.0 |= Self::FLAG_EXECUTE;
        } else {
            self.0 &= !Self::FLAG_EXECUTE;
        }
    }

    pub const fn is_user(&self) -> bool {
        self.0 & Self::FLAG_USER != 0
    }

    pub const fn set_user(&mut self, user: bool) {
        if user {
            self.0 |= Self::FLAG_USER;
        } else {
            self.0 &= !Self::FLAG_USER;
        }
    }

    pub const fn is_global(&self) -> bool {
        self.0 & Self::FLAG_GLOBAL != 0
    }

    pub const fn set_global(&mut self, global: bool) {
        if global {
            self.0 |= Self::FLAG_GLOBAL;
        } else {
            self.0 &= !Self::FLAG_GLOBAL;
        }
    }

    pub const fn is_accessed(&self) -> bool {
        self.0 & Self::FLAG_ACCESSED != 0
    }

    pub const fn set_accessed(&mut self, accessed: bool) {
        if accessed {
            self.0 |= Self::FLAG_ACCESSED;
        } else {
            self.0 &= !Self::FLAG_ACCESSED;
        }
    }

    pub const fn is_dirty(&self) -> bool {
        self.0 & Self::FLAG_DIRTY != 0
    }

    pub const fn set_dirty(&mut self, dirty: bool) {
        if dirty {
            self.0 |= Self::FLAG_DIRTY;
        } else {
            self.0 &= !Self::FLAG_DIRTY;
        }
    }

    pub const fn is_leaf(&self) -> bool {
        // An entry is a leaf if it's valid and has at least one of R, W, or X
        // bits set.
        self.is_valid() && (self.is_readable() || self.is_writable() || self.is_executable())
    }
}

/// Calculates the recursive virtual page number for the page table containing
/// the given virtual page number.
///
/// In a recursive page table mapping, the page tables themselves are mapped
/// into virtual memory. This function computes the virtual page number where
/// the page table containing the given VPN would be mapped in a recursive page
/// table configuration.
///
/// # Arguments
///
/// * `vpn` - The virtual page number for which we want to find the containing
///   page table's VPN.
///
/// # Returns
///
/// A `VirtualPageNumber` representing where the containing page table is mapped
/// in virtual memory.
fn get_recursive_vpn_for_page_table_containing_vpn(vpn: VirtualPageNumber) -> VirtualPageNumber {
    // In sv39, there are 9 bits per level, with 3 levels total. For recursive
    // mapping, we use a fixed index in the root page table to point to itself.
    // By convention, we'll use the last entry (index 511) for the recursive
    // mapping.

    // Extract the VPN indices for each level.
    let vpn2 = vpn.get_level_2_index();
    let vpn1 = vpn.get_level_1_index();

    // For a recursive mapping to the level-0 page table (which directly
    // contains our target VPN), we need to construct a virtual address where:
    // - The level-2 index points to the recursive entry (511).
    // - The level-1 index points to the level-2 entry of our original VPN.
    // - The level-0 index points to the level-1 entry of our original VPN.

    // Construct a new VPN with these indices.
    let recursive_vpn_raw = (511 << 18) | (vpn2 << 9) | (vpn1 << 0);

    // Create and return the new VPN.
    VirtualPageNumber::from_raw_virtual_page_number(recursive_vpn_raw)
}

/// Allocates the necessary page tables and creates entries for the provided
/// virtual page number.
///
/// This function traverses the page table hierarchy for the given virtual page
/// number. It creates page tables as needed for each level (using the provided
/// allocator), but does not modify any existing valid leaf entry.
///
/// # Arguments
///
/// * `page_table_root` - A mutable reference to the root page table.
/// * `vpn` - The virtual page number to allocate page table entries for.
/// * `physical_memory_allocator` - The allocator used to get physical pages for
///   new page tables.
///
/// # Returns
///
/// * `true` - If new page tables were allocated or there was already a valid
///   entry.
/// * `false` - If allocation failed due to insufficient physical memory.
pub fn allocate_vpn(
    page_table_root: &mut PageTable,
    vpn: VirtualPageNumber,
    physical_memory_allocator: &mut impl PhysicalMemoryAllocator,
) -> bool {
    // Extract the indices for each level from the VPN.
    let level_2_index = vpn.get_level_2_index();
    let level_1_index = vpn.get_level_1_index();
    let level_0_index = vpn.get_level_0_index();

    // Check if the level 2 (root) entry exists.
    let level_2_entry = page_table_root.get_entry_mut(level_2_index);

    // If level 2 entry is not valid, we need to create a new level 1 page
    // table.
    if !level_2_entry.is_valid() {
        // Allocate a physical page for the level 1 page table.
        let level_1_ptr = match physical_memory_allocator.allocate_page() {
            Some(ptr) => ptr,
            None => return false, // Allocation failed.
        };

        // Convert the raw pointer to a PhysicalPageNumber.
        let level_1_ppn = PhysicalPageNumber::from_physical_address(level_1_ptr as usize);

        // Create a new level 1 page table at the allocated physical address.
        let level_1_page_table = unsafe { &mut *(level_1_ptr as *mut PageTable) };

        // Initialize the new page table by clearing all entries.
        level_1_page_table.clear();

        // Update the level 2 entry to point to the new level 1 page table.
        level_2_entry.set_valid(true);
        level_2_entry.set_ppn(level_1_ppn);
    }

    // If the level 2 entry is a leaf entry (has R/W/X permissions), then the
    // mapping is already done at a higher level.
    if level_2_entry.is_leaf() {
        return true;
    }

    // Get the level 1 page table.
    let level_1_page_table =
        unsafe { &mut *(level_2_entry.get_ppn().to_physical_address() as *mut PageTable) };

    // Check if the level 1 entry exists.
    let level_1_entry = level_1_page_table.get_entry_mut(level_1_index);

    // If level 1 entry is not valid, we need to create a new level 0 page
    // table.
    if !level_1_entry.is_valid() {
        // Allocate a physical page for the level 0 page table.
        let level_0_ptr = match physical_memory_allocator.allocate_page() {
            Some(ptr) => ptr,
            None => return false, // Allocation failed.
        };

        // Convert the raw pointer to a PhysicalPageNumber.
        let level_0_ppn = PhysicalPageNumber::from_physical_address(level_0_ptr as usize);

        // Create a new level 0 page table at the allocated physical address.
        let level_0_page_table = unsafe { &mut *(level_0_ptr as *mut PageTable) };

        // Initialize the new page table by clearing all entries.
        level_0_page_table.clear();

        // Update the level 1 entry to point to the new level 0 page table.
        level_1_entry.set_valid(true);
        level_1_entry.set_ppn(level_0_ppn);
    }

    // If the level 1 entry is a leaf entry, then the mapping is already done.
    if level_1_entry.is_leaf() {
        return true;
    }

    // Get the level 0 page table.
    let level_0_page_table =
        unsafe { &mut *(level_1_entry.get_ppn().to_physical_address() as *mut PageTable) };

    // Check if the level 0 entry exists.
    let level_0_entry = level_0_page_table.get_entry_mut(level_0_index);

    // If the level 0 entry is already a valid leaf entry, we don't need to do
    // anything.
    if level_0_entry.is_valid() && level_0_entry.is_leaf() {
        return true;
    }

    // If it's not a valid leaf entry, we allocate a physical page.
    let page_ptr = match physical_memory_allocator.allocate_page() {
        Some(ptr) => ptr,
        None => return false, // Allocation failed.
    };

    // Convert the raw pointer to a PhysicalPageNumber.
    let physical_page = PhysicalPageNumber::from_physical_address(page_ptr as usize);

    // Setup the level 0 entry as a leaf page.
    level_0_entry.set_valid(true);
    level_0_entry.set_ppn(physical_page);
    level_0_entry.set_readable(true);
    level_0_entry.set_writable(true);
    level_0_entry.set_executable(false); // By default, pages are not executable.

    true
}

pub fn translate_virtual_address(page_table_root: &PageTable, virtual_address: usize) -> usize {
    let offset: usize = virtual_address & 0x0000_0000_0000_0FFF;
    let vpn0: usize = ((virtual_address >> 12) & 0x1FF) as usize;
    let vpn1: usize = ((virtual_address >> 21) & 0x1FF) as usize;
    let vpn2: usize = ((virtual_address >> 30) & 0x1FF) as usize;

    let page_table_entry_2 = page_table_root.get_entry(vpn2);
    if !page_table_entry_2.is_valid() {
        return 0;
    }

    let page_table_level_1 =
        unsafe { &*(page_table_entry_2.get_ppn().to_physical_address() as *const PageTable) };

    let page_table_entry_1 = page_table_level_1.get_entry(vpn1);
    if !page_table_entry_1.is_valid() {
        return 0;
    }

    let page_table_level_0 =
        unsafe { &*(page_table_entry_1.get_ppn().to_physical_address() as *const PageTable) };

    let page_table_entry_0 = page_table_level_0.get_entry(vpn0);
    if !page_table_entry_0.is_valid() {
        return 0;
    }

    let ppn = page_table_entry_0.get_ppn();
    let physical_address = ppn.to_physical_address() | offset;

    physical_address
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::PhysicalPageNumber;

    /// Set up a basic three-level page table structure for testing translation.
    fn setup_page_tables() -> (PageTable, *const PageTable, *const PageTable) {
        let mut root = PageTable::new();
        let mut level1 = Box::new(PageTable::new());
        let mut level0 = Box::new(PageTable::new());

        // Create a mapping for virtual page 0x0012_3456 -> physical page
        // 0x00AB_CDEF. vpn2 = 0x0123 (291), vpn1 = 0x0056 (86), vpn0 = 0x0056
        // (86)

        // Set up level 0 page table (contains the leaf entry).
        let mut leaf_entry = PageTableEntry::new();
        leaf_entry.set_valid(true);
        leaf_entry.set_readable(true);
        leaf_entry.set_ppn(PhysicalPageNumber::from_raw_physical_page_number(
            0x00AB_CDEF,
        ));
        level0.set_entry(0x0056, leaf_entry);

        // Set up level 1 page table (points to level 0).
        let level0_ptr = Box::into_raw(level0);
        let level0_ppn = PhysicalPageNumber::from_physical_address(level0_ptr as usize);

        let mut l1_entry = PageTableEntry::new();
        l1_entry.set_valid(true);
        l1_entry.set_ppn(level0_ppn);
        level1.set_entry(0x0056, l1_entry);

        // Set up root page table (points to level 1).
        let level1_ptr = Box::into_raw(level1);
        let level1_ppn = PhysicalPageNumber::from_physical_address(level1_ptr as usize);

        let mut root_entry = PageTableEntry::new();
        root_entry.set_valid(true);
        root_entry.set_ppn(level1_ppn);
        root.set_entry(0x0123, root_entry);

        (root, level1_ptr, level0_ptr)
    }

    /// Clean up allocated page tables to prevent memory leaks.
    fn cleanup_page_tables(level1_ptr: *const PageTable, level0_ptr: *const PageTable) {
        unsafe {
            // Convert back to Box and drop.
            let _level1 = Box::from_raw(level1_ptr as *mut PageTable);
            let _level0 = Box::from_raw(level0_ptr as *mut PageTable);
        }
    }

    #[test]
    fn test_translate_valid_address() {
        let (root, level1_ptr, level0_ptr) = setup_page_tables();

        // Construct a virtual address with: vpn2 = 0x0123, vpn1 = 0x0056, vpn0
        // = 0x0056, offset = 0x0ABC
        let virtual_address: usize = (0x0123 << 30) | (0x0056 << 21) | (0x0056 << 12) | 0x0ABC;

        // Expected physical address: physical page 0x00AB_CDEF with offset
        // 0x0ABC.
        let expected_physical_address: usize = (0x00AB_CDEF << 12) | 0x0ABC;

        let result = translate_virtual_address(&root, virtual_address);

        cleanup_page_tables(level1_ptr, level0_ptr);
        assert_eq!(result, expected_physical_address);
    }

    #[test]
    fn test_translate_invalid_root_entry() {
        let root = PageTable::new();
        // Entry 0x0123 is not set to valid.

        let virtual_address = (0x0123 << 30) | (0x0056 << 21) | (0x0056 << 12) | 0x0ABC;

        let result = translate_virtual_address(&root, virtual_address);
        assert_eq!(
            result, 0,
            "Translation should fail with invalid root entry."
        );
    }

    #[test]
    fn test_translate_invalid_level1_entry() {
        let mut root = PageTable::new();
        let level1 = Box::new(PageTable::new());

        // Set up root to point to level1, but don't set up level1 entry.
        let level1_ptr = Box::into_raw(level1);
        let level1_ppn = PhysicalPageNumber::from_physical_address(level1_ptr as usize);

        let mut root_entry = PageTableEntry::new();
        root_entry.set_valid(true);
        root_entry.set_ppn(level1_ppn);
        root.set_entry(0x0123, root_entry);

        let virtual_address = (0x0123 << 30) | (0x0056 << 21) | (0x0056 << 12) | 0x0ABC;

        let result = translate_virtual_address(&root, virtual_address);

        unsafe {
            let _level1 = Box::from_raw(level1_ptr);
        }

        assert_eq!(
            result, 0,
            "Translation should fail with invalid level 1 entry."
        );
    }

    #[test]
    fn test_translate_invalid_level0_entry() {
        let mut root = PageTable::new();
        let mut level1 = Box::new(PageTable::new());
        let level0 = Box::new(PageTable::new());

        // Set up level1 to point to level0, but don't set up level0 entry.
        let level0_ptr = Box::into_raw(level0);
        let level0_ppn = PhysicalPageNumber::from_physical_address(level0_ptr as usize);

        let mut l1_entry = PageTableEntry::new();
        l1_entry.set_valid(true);
        l1_entry.set_ppn(level0_ppn);
        level1.set_entry(0x0056, l1_entry);

        // Set up root to point to level1.
        let level1_ptr = Box::into_raw(level1);
        let level1_ppn = PhysicalPageNumber::from_physical_address(level1_ptr as usize);

        let mut root_entry = PageTableEntry::new();
        root_entry.set_valid(true);
        root_entry.set_ppn(level1_ppn);
        root.set_entry(0x0123, root_entry);

        let virtual_address = (0x0123 << 30) | (0x0056 << 21) | (0x0056 << 12) | 0x0ABC;

        let result = translate_virtual_address(&root, virtual_address);

        unsafe {
            let _level0 = Box::from_raw(level0_ptr);
            let _level1 = Box::from_raw(level1_ptr);
        }

        assert_eq!(
            result, 0,
            "Translation should fail with invalid level 0 entry."
        );
    }

    #[test]
    fn test_translate_different_offsets() {
        let (root, level1_ptr, level0_ptr) = setup_page_tables();

        // Test with offset 0x0000.
        let virtual_address_1: usize = (0x0123 << 30) | (0x0056 << 21) | (0x0056 << 12) | 0x0000;
        let expected_physical_address_1: usize = (0x00AB_CDEF << 12) | 0x0000;
        let result_1 = translate_virtual_address(&root, virtual_address_1);

        // Test with offset 0x0FFF (maximum offset).
        let virtual_address_2 = (0x0123 << 30) | (0x0056 << 21) | (0x0056 << 12) | 0x0FFF;
        let expected_physical_address_2 = (0x00AB_CDEF << 12) | 0x0FFF;
        let result_2 = translate_virtual_address(&root, virtual_address_2);

        cleanup_page_tables(level1_ptr, level0_ptr);

        assert_eq!(
            result_1, expected_physical_address_1 as usize,
            "Translation with zero offset failed."
        );
        assert_eq!(
            result_2, expected_physical_address_2,
            "Translation with maximum offset failed."
        );
    }

    #[test]
    fn test_get_recursive_vpn_basic() {
        // Create a VPN with known indices: vpn2=123, vpn1=456, vpn0=289
        let vpn_raw = (123 << 18) | (456 << 9) | 289;
        let vpn = VirtualPageNumber::from_raw_virtual_page_number(vpn_raw);

        // Expected: recursive_vpn with indices: vpn2=511, vpn1=123, vpn0=456
        let expected_raw = (511 << 18) | (123 << 9) | 456;
        let expected = VirtualPageNumber::from_raw_virtual_page_number(expected_raw);

        let result = get_recursive_vpn_for_page_table_containing_vpn(vpn);

        assert_eq!(result, expected, "Recursive VPN calculation incorrect.");
        assert_eq!(
            result.get_level_2_index(),
            511,
            "Recursive VPN level 2 index should be 511."
        );
        assert_eq!(
            result.get_level_1_index(),
            123,
            "Recursive VPN level 1 index should match original vpn2."
        );
        assert_eq!(
            result.get_level_0_index(),
            456,
            "Recursive VPN level 0 index should match original vpn1."
        );
    }

    #[test]
    fn test_get_recursive_vpn_boundary_values() {
        // Test with minimum indices (all zeros).
        let min_vpn = VirtualPageNumber::from_raw_virtual_page_number(0);
        let min_result = get_recursive_vpn_for_page_table_containing_vpn(min_vpn);

        let expected_min_raw = 511 << 18;
        let expected_min = VirtualPageNumber::from_raw_virtual_page_number(expected_min_raw);
        assert_eq!(
            min_result, expected_min,
            "Recursive VPN calculation incorrect for minimum VPN."
        );
        assert_eq!(expected_min.get_level_2_index(), 511);
        assert_eq!(expected_min.get_level_1_index(), 0);
        assert_eq!(expected_min.get_level_0_index(), 0);

        // Test with maximum indices (all 0x1FF = 511).
        let max_vpn_raw = (511 << 18) | (511 << 9) | 511;
        let max_vpn = VirtualPageNumber::from_raw_virtual_page_number(max_vpn_raw);
        let max_result = get_recursive_vpn_for_page_table_containing_vpn(max_vpn);

        let expected_max_raw = (511 << 18) | (511 << 9) | 511;
        let expected_max = VirtualPageNumber::from_raw_virtual_page_number(expected_max_raw);
        assert_eq!(
            max_result, expected_max,
            "Recursive VPN calculation incorrect for maximum VPN."
        );
        assert_eq!(max_result.get_level_2_index(), 511);
        assert_eq!(max_result.get_level_1_index(), 511);
        assert_eq!(max_result.get_level_0_index(), 511);
    }
}
