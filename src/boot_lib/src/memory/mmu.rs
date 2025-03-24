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

    pub const fn get_entries(&self) -> &[PageTableEntry] {
        &self.entries
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

    pub const fn set_flags(&mut self, flags: &PageTableEntryFlags) {
        self.set_readable(flags.readable);
        self.set_writable(flags.writable);
        self.set_executable(flags.executable);
        self.set_user(flags.user);
        self.set_global(flags.global);
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

    pub const fn is_leaf(&self) -> bool {
        // An entry is a leaf if it's valid and has at least one of R, W, or X
        // bits set.
        self.is_valid() && (self.is_readable() || self.is_writable() || self.is_executable())
    }
}

#[derive(Debug, Clone, Default)]
pub struct PageTableEntryFlags {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub user: bool,
    pub global: bool,
}

impl PageTableEntryFlags {
    pub const fn get_readable(&self) -> bool {
        self.readable
    }

    pub const fn set_readable(&mut self, readable: bool) {
        self.readable = readable;
    }

    pub const fn get_writable(&self) -> bool {
        self.writable
    }

    pub const fn set_writable(&mut self, writable: bool) {
        self.writable = writable;
    }

    pub const fn get_executable(&self) -> bool {
        self.executable
    }

    pub const fn set_executable(&mut self, executable: bool) {
        self.executable = executable;
    }

    pub const fn get_user(&self) -> bool {
        self.user
    }

    pub const fn set_user(&mut self, user: bool) {
        self.user = user;
    }

    pub const fn get_global(&self) -> bool {
        self.global
    }

    pub const fn set_global(&mut self, global: bool) {
        self.global = global;
    }
}

/// Assigns a new physical page to the specified virtual page number in the page
/// table. A new physical page is allocated if the provided physical page number
/// is None.
///
/// This function walks the page table hierarchy starting from the root page
/// table, creating intermediate page tables as needed. It maps the requested
/// virtual page number to a physical page, either by using the provided
/// physical page number or by allocating a new page when needed. The resulting
/// leaf entry's valid, readable, writable, and executable permissions are set
/// based on the flags argument. The accessed and dirty flags are initially
/// cleared. If the page is already allocated, the function returns the existing
/// physical page.
///
/// # Arguments
///
/// * `page_table_root` - A mutable reference to the root page table.
/// * `vpn` - The virtual page number to allocate and map.
/// * `ppn` - An optional physical page number to use for mapping. If `None`, a
///   new physical page is allocated if needed.
/// * `physical_memory_allocator` - A mutable reference to a physical memory
///   allocator.
///
/// # Returns
///
/// * `Some(PhysicalPageNumber)` - The physical page number that was mapped
///   (either newly allocated or previously mapped).
/// * `None` - If the allocation failed due to a lack of physical memory.
pub fn allocate_vpn(
    page_table_root: &mut PageTable,
    vpn: VirtualPageNumber,
    ppn: Option<PhysicalPageNumber>,
    flags: &PageTableEntryFlags,
    physical_memory_allocator: &mut impl PhysicalMemoryAllocator,
) -> Option<PhysicalPageNumber> {
    // Extract the 9-bit indices for each level of the page table.
    let vpn2 = vpn.get_level_2_index();
    let vpn1 = vpn.get_level_1_index();
    let vpn0 = vpn.get_level_0_index();

    // Get the level 2 (root) entry.
    let mut page_table_level_2_entry = *page_table_root.get_entry(vpn2);

    // If the level 2 entry is not valid, allocate a new level 1 page table.
    if !page_table_level_2_entry.is_valid() {
        let page_table_level_1_ptr = physical_memory_allocator.allocate_page()?;
        let page_table_level_1_ppn =
            PhysicalPageNumber::from_physical_address(page_table_level_1_ptr as usize);
        let page_table_level_1 = unsafe { &mut *(page_table_level_1_ptr as *mut PageTable) };

        // Initialize the new page table to all zeros.
        page_table_level_1.clear();

        // Set up the level 2 entry to point to the new level 1 page table.
        page_table_level_2_entry.set_valid(true);
        page_table_level_2_entry.set_ppn(page_table_level_1_ppn);

        // Write the updated entry back to the root page table.
        page_table_root.set_entry(vpn2, page_table_level_2_entry);
    }

    // Access the level 1 page table.
    let page_table_level_1_ptr =
        page_table_level_2_entry.get_ppn().to_physical_address() as *mut PageTable;
    let page_table_level_1 = unsafe { &mut *page_table_level_1_ptr };

    // Get the level 1 entry.
    let mut page_table_level_1_entry = *page_table_level_1.get_entry(vpn1);

    // If the level 1 entry is not valid, allocate a new level 0 page table.
    if !page_table_level_1_entry.is_valid() {
        let page_table_level_0_ptr = physical_memory_allocator.allocate_page()?;
        let page_table_level_0_ppn =
            PhysicalPageNumber::from_physical_address(page_table_level_0_ptr as usize);
        let page_table_level_0 = unsafe { &mut *(page_table_level_0_ptr as *mut PageTable) };

        // Initialize the new page table to all zeros.
        page_table_level_0.clear();

        // Set up the level 1 entry to point to the new level 0 page table.
        page_table_level_1_entry.set_valid(true);
        page_table_level_1_entry.set_ppn(page_table_level_0_ppn);

        // Write the updated entry back to the level 1 page table.
        page_table_level_1.set_entry(vpn1, page_table_level_1_entry);
    }

    // Access the level 0 page table.
    let page_table_level_0_ptr =
        page_table_level_1_entry.get_ppn().to_physical_address() as *mut PageTable;
    let page_table_level_0 = unsafe { &mut *page_table_level_0_ptr };

    // Get the level 0 entry.
    let mut page_table_level_0_entry = *page_table_level_0.get_entry(vpn0);

    // Check if the page is already allocated.
    if page_table_level_0_entry.is_valid() && page_table_level_0_entry.is_leaf() {
        // Page already allocated, return the physical page number.
        return Some(page_table_level_0_entry.get_ppn());
    }

    // Determine the physical page to map.
    let physical_page_ppn = if let Some(some_ppn) = ppn {
        // Use the provided physical page number.
        some_ppn
    } else {
        // Allocate a new physical page for the actual memory.
        let physical_page_ptr = physical_memory_allocator.allocate_page()?;
        PhysicalPageNumber::from_physical_address(physical_page_ptr as usize)
    };

    // Clear the entry to zeroes.
    page_table_level_0_entry.clear();

    // Set up the level 0 entry as a leaf entry.
    page_table_level_0_entry.set_valid(true);
    page_table_level_0_entry.set_flags(flags);
    page_table_level_0_entry.set_ppn(physical_page_ppn);

    // Write the updated entry back to the level 0 page table.
    page_table_level_0.set_entry(vpn0, page_table_level_0_entry);

    // Return the physical page number that was allocated or provided.
    Some(physical_page_ppn)
}

/// Maps a virtual page number directly to a physical page number using a level
/// 2 (1 GiB) gigapage mapping in the sv39 paging mode.
///
/// This function creates a single page table entry at the level 2 page table
/// (the root) that maps an entire 1 GiB region of virtual memory to a
/// corresponding 1 GiB region of physical memory. This is more efficient than
/// using 4 KiB mappings for large memory regions as it requires fewer page
/// table entries and TLB entries.
///
/// This function does not allocate memory to back the page table entry. It is
/// assumed that the caller has already allocated the physical page number and
/// ensured it is aligned to a 1 GiB boundary.
///
/// # Arguments
///
/// * `page_table_root` - A mutable reference to the root page table.
/// * `vpn` - The virtual page number to map. Only the level 2 index (bits
///   26-18) is used.
/// * `ppn` - The physical page number to map to. This should be aligned to a 1
///   GiB boundary.
/// * `flags` - Page table entry flags to apply (readable, writable, executable,
///   etc.).
///
/// # Returns
///
/// * `true` - If the mapping was successfully created.
/// * `false` - If the mapping could not be created because:
///   - The entry already exists as a leaf entry.
///   - The entry already points to a level 1 page table (has child pages).
///
/// # Notes
///
/// * This function creates a 1 GiB mapping (gigapage), so the physical page
///   number should be aligned to a 1 GiB boundary for proper operation.
/// * When using this function, the caller must ensure the provided physical
///   page number is correctly aligned, as this function does not perform
///   alignment checks.
/// * In sv39 mode, this maps a single entry in the level 2 page table, covering
///   the entire address range for that index (1 GiB).
pub fn allocate_level_2_vpn(
    page_table_root: &mut PageTable,
    vpn: VirtualPageNumber,
    ppn: PhysicalPageNumber,
    flags: &PageTableEntryFlags,
) -> bool {
    let vpn2 = vpn.get_level_2_index();

    // Get the current level 2 entry.
    let mut page_table_level_2_entry = *page_table_root.get_entry(vpn2);

    // Check if the entry is already valid and is a leaf entry.
    if page_table_level_2_entry.is_valid() && page_table_level_2_entry.is_leaf() {
        // Entry is already allocated as a leaf, return the physical page
        // number.
        return false;
    }

    // If the entry is already valid but not a leaf (points to a level 1 page
    // table), we cannot convert it to a leaf as it would invalidate existing
    // mappings.
    if page_table_level_2_entry.is_valid() {
        return false;
    }

    // Clear the entry.
    page_table_level_2_entry.clear();

    // Set up the level 2 entry as a leaf entry.
    page_table_level_2_entry.set_valid(true);
    page_table_level_2_entry.set_flags(flags);
    page_table_level_2_entry.set_ppn(ppn);

    // Write the updated entry back to the root page table.
    page_table_root.set_entry(vpn2, page_table_level_2_entry);

    true
}

/// Maps a range of physical pages to the same virtual addresses in the page
/// table.
///
/// This function performs identity mapping, meaning that physical addresses are
/// mapped to the same virtual addresses. It iterates from the start page number
/// through the end page number (inclusive) and creates a mapping for each page
/// with the specified flags.
///
/// # Arguments
///
/// * `page_table_root` - A mutable reference to the root page table where
///   mappings will be added.
/// * `start_ppn_inclusive` - The starting physical page number (inclusive) of
///   the range to map.
/// * `end_ppn_inclusive` - The ending physical page number (inclusive) of the
///   range to map.
/// * `flags` - Page table entry flags to apply to each mapping (readable,
///   writable, executable, etc.).
/// * `physical_memory_allocator` - A mutable reference to a physical memory
///   allocator used for creating page tables if needed.
///
/// # Notes
///
/// * If the start page number is greater than the end page number, the function
///   returns without doing anything.
/// * This function may create intermediate page table entries as necessary.
/// * Errors in allocation are silently ignored - if a page mapping fails, the
///   function continues with the next page.
pub fn identity_map_range(
    page_table_root: &mut PageTable,
    start_ppn_inclusive: PhysicalPageNumber,
    end_ppn_inclusive: PhysicalPageNumber,
    flags: &PageTableEntryFlags,
    physical_memory_allocator: &mut impl PhysicalMemoryAllocator,
) {
    if start_ppn_inclusive > end_ppn_inclusive {
        return;
    }

    let mut current_ppn = start_ppn_inclusive;
    while current_ppn <= end_ppn_inclusive {
        let vpn = VirtualPageNumber::from_raw_virtual_page_number(current_ppn.raw_ppn());
        allocate_vpn(
            page_table_root,
            vpn,
            Some(current_ppn),
            flags,
            physical_memory_allocator,
        );

        current_ppn = PhysicalPageNumber::from_raw_physical_page_number(current_ppn.raw_ppn() + 1);
    }
}

/// Maps a range of physical pages to a specified range of virtual pages in the
/// page table.
///
/// This function maps physical pages starting at `start_ppn_inclusive` to
/// virtual pages starting at `start_vpn_inclusive` for the specified number of
/// pages. It creates mappings with the specified flags for each page in the
/// range.
///
/// # Arguments
///
/// * `page_table_root` - A mutable reference to the root page table where
///   mappings will be added.
/// * `start_ppn_inclusive` - The starting physical page number (inclusive) to
///   map from.
/// * `start_vpn_inclusive` - The starting virtual page number (inclusive) to
///   map to.
/// * `number_of_pages_inclusive` - The number of pages to map (inclusive
///   count).
/// * `flags` - Page table entry flags to apply to each mapping (readable,
///   writable, executable, etc.).
/// * `physical_memory_allocator` - A mutable reference to a physical memory
///   allocator used for creating page tables if needed.
///
/// # Notes
///
/// * This function creates a separate mapping for each page in the range.
/// * If the number of pages to map is zero, the function returns without doing.
/// * This function may create intermediate page table entries as necessary.
/// * Errors in allocation are silently ignored - if a page mapping fails, the
///   function continues with the next page.
pub fn map_range(
    page_table_root: &mut PageTable,
    start_ppn_inclusive: PhysicalPageNumber,
    start_vpn_inclusive: VirtualPageNumber,
    number_of_pages_inclusive: usize,
    flags: &PageTableEntryFlags,
    physical_memory_allocator: &mut impl PhysicalMemoryAllocator,
) {
    let mut current_ppn = start_ppn_inclusive;
    let mut current_vpn = start_vpn_inclusive;

    for _ in 0..=number_of_pages_inclusive {
        allocate_vpn(
            page_table_root,
            current_vpn,
            Some(current_ppn),
            flags,
            physical_memory_allocator,
        );

        current_ppn = PhysicalPageNumber::from_raw_physical_page_number(current_ppn.raw_ppn() + 1);
        current_vpn = VirtualPageNumber::from_raw_virtual_page_number(current_vpn.raw_vpn() + 1);
    }
}

/// Translates a virtual address to its corresponding physical address using the
/// provided root page table.
///
/// This function walks the three-level page table hierarchy to perform the
/// address translation. It returns None if any page table entry in the
/// translation path is invalid.
///
/// # Arguments
///
/// * `page_table_root` - A reference to the root (level 2) page table.
/// * `virtual_address` - The virtual address to translate.
///
/// # Returns
///
/// * `Some(usize)` - The physical address if translation succeeds.
/// * `None` - If translation fails due to any invalid page table entries.
pub fn translate_virtual_address(
    page_table_root: &PageTable,
    virtual_address: usize,
) -> Option<usize> {
    let vpn2: usize = ((virtual_address >> 30) & 0x1FF) as usize;
    let vpn1: usize = ((virtual_address >> 21) & 0x1FF) as usize;
    let vpn0: usize = ((virtual_address >> 12) & 0x1FF) as usize;
    let offset: usize = virtual_address & 0x0000_0000_0000_0FFF;

    let page_table_level_2_entry = page_table_root.get_entry(vpn2);
    if !page_table_level_2_entry.is_valid() {
        return None;
    }

    let page_table_level_1 =
        unsafe { &*(page_table_level_2_entry.get_ppn().to_physical_address() as *const PageTable) };

    let page_table_level_1_entry = page_table_level_1.get_entry(vpn1);
    if !page_table_level_1_entry.is_valid() {
        return None;
    }

    let page_table_level_0 =
        unsafe { &*(page_table_level_1_entry.get_ppn().to_physical_address() as *const PageTable) };

    let page_table_level_0_entry = page_table_level_0.get_entry(vpn0);
    if !page_table_level_0_entry.is_valid() {
        return None;
    }

    let ppn = page_table_level_0_entry.get_ppn();
    let physical_address = ppn.to_physical_address() | offset;

    Some(physical_address)
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
        assert_eq!(result, Some(expected_physical_address));
    }

    #[test]
    fn test_translate_invalid_root_entry() {
        let root = PageTable::new();
        // Entry 0x0123 is not set to valid.

        let virtual_address = (0x0123 << 30) | (0x0056 << 21) | (0x0056 << 12) | 0x0ABC;

        let result = translate_virtual_address(&root, virtual_address);
        assert_eq!(
            result, None,
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
            result, None,
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
            result, None,
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
            result_1,
            Some(expected_physical_address_1),
            "Translation with zero offset failed."
        );
        assert_eq!(
            result_2,
            Some(expected_physical_address_2),
            "Translation with maximum offset failed."
        );
    }
}
