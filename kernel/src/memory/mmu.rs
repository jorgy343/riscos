#![allow(dead_code)]

/// Represents a physical page number (PPN).
/// 
/// This is the top 44 bits of a 56-bit physical address. The structure stores
/// the PPN with bit 0 representing the start of the PPN (the address
/// right-shifted by 12 bits), as it does not include the 12-bit page offset.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PhysicalPageNumber(u64);

impl PhysicalPageNumber {
    /// Create a new `PhysicalPageNumber` from a physical address.
    /// 
    /// # Arguments
    /// * `physical_address` - The physical address which will be right shifted
    ///   by 12 bits to get the PPN. The lower 12 bits are lost. This is
    ///   equivalent to rounding down the physical address to the nearest 4KiB
    ///   boundary.
    /// 
    /// # Returns
    /// The `PhysicalPageNumber` representing the top 44 bits of the physical
    /// address.
    /// 
    /// # Example
    /// ```
    /// let physical_address = 0x8020_0123;
    /// let ppn = PhysicalPageNumber::from(physical_address);
    /// 
    /// assert_eq!(ppn.0, 0x0008_0200);
    /// ```
    pub const fn from_physical_address(physical_address: u64) -> Self {
        Self(physical_address >> 12)
    }

    /// Create a new `PhysicalPageNumber` from a raw physical page number
    /// typically coming from a page table entry.
    /// 
    /// # Arguments
    /// * `ppn` - The 44-bit physical page number.
    ///
    /// # Returns
    /// The `PhysicalPageNumber` representing the top 44 bits of the physical
    /// address.
    pub const fn from_raw_physical_page_number(ppn: u64) -> Self {
        Self(ppn)
    }

    /// Get the physical address this `PhysicalPageNumber` represents. The
    /// physical address represents the address pointing to the first byte of a
    /// 4KiB page.
    /// 
    /// # Returns
    /// The physical address with the PPN shifted left by 12 bits. The resultant
    /// physical address is guaranteed to be aligned to a 4KiB boundary.
    pub const fn to_physical_address(&self) -> u64 {
        self.0 << 12
    }
}

/// Represents a virtual page number (VPN).
/// 
/// This is the top 27 bits of a 39-bit virtual address. The structure stores
/// the VPN with bit 0 representing the start of the VPN (the address
/// right-shifted by 12 bits), as it does not include the 12-bit page offset.
/// 
/// This virtual page number object only supports sv39 mode where virtual
/// addresses are a total of 39 bits (12-bit page offset + 27-bit VPN).
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct VirtualPageNumber(u64);

impl VirtualPageNumber {
    /// Create a new `VirtualPageNumber` from a virtual address.
    /// 
    /// # Arguments
    /// * `virtual_address` - The virtual address which is right shifted by 12
    ///   bits to get the VPN. The lower 12 bits are lost. This is equivalent to
    ///   rounding down the virtual address to the nearest 4KiB boundary.
    /// 
    /// # Returns
    /// The `VirtualPageNumber` representing the top 27 bits of the virtual
    /// address.
    pub const fn from_virtual_address(virtual_address: u64) -> Self {
        Self(virtual_address >> 12)
    }

    /// Create a new `VirtualPageNumber` from a raw virtual page number
    /// typically coming from a page table entry.
    /// 
    /// # Arguments
    /// * `vpn` - The 27-bit virtual page number.
    /// 
    /// # Returns
    /// The `VirtualPageNumber` representing the top 27 bits of the virtual
    /// address.
    pub const fn from_raw_virtual_page_number(vpn: u64) -> Self {
        Self(vpn)
    }

    /// Get the virtual address this `VirtualPageNumber` represents. The virtual
    /// address represents the address pointing to the first byte of a 4KiB
    /// page.
    /// 
    /// # Returns
    /// The virtual address with the VPN shifted left by 12 bits. The resultant
    /// virtual address is guaranteed to be aligned to a 4KiB boundary.
    pub const fn to_virtual_address(&self) -> u64 {
        self.0 << 12
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    const FLAG_VALID: u64 = 1 << 0;     // V bit - entry is valid
    const FLAG_READ: u64 = 1 << 1;      // R bit - readable
    const FLAG_WRITE: u64 = 1 << 2;     // W bit - writable
    const FLAG_EXECUTE: u64 = 1 << 3;   // X bit - executable
    const FLAG_USER: u64 = 1 << 4;      // U bit - accessible to user mode
    const FLAG_GLOBAL: u64 = 1 << 5;    // G bit - global mapping
    const FLAG_ACCESSED: u64 = 1 << 6;  // A bit - page was accessed
    const FLAG_DIRTY: u64 = 1 << 7;     // D bit - page was written to

    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn clear(&mut self) {
        self.0 = 0;
    }

    pub const fn get_ppn(&self) -> PhysicalPageNumber {
        PhysicalPageNumber::from_raw_physical_page_number((self.0 >> 10) & 0x0000_0FFF_FFFF_FFFF)
    }

    pub const fn set_ppn(&mut self, ppn: PhysicalPageNumber) {
        // Clear the old PPN and set the new one.
        self.0 = (self.0 & !0x0000_003F_FFFF_FFF0) | ((ppn.0 & 0x0000_0FFF_FFFF_FFFF) << 10);
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
        // An entry is a leaf if it's valid and has at least one of R, W, or X bits set.
        self.is_valid() && (self.is_readable() || self.is_writable() || self.is_executable())
    }
}

#[derive(Clone)]
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    /// Create a new page table with all entries cleared to zero (invalid).
    /// 
    /// # Returns
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

pub fn translate_virtual_address(page_table_root: &PageTable, virtual_address: u64) -> u64 {
    let offset: u64 = virtual_address & 0x0000_0000_0000_0FFF;
    let vpn0: usize = ((virtual_address >> 12) & 0x1FF) as usize;
    let vpn1: usize = ((virtual_address >> 21) & 0x1FF) as usize;
    let vpn2: usize = ((virtual_address >> 30) & 0x1FF) as usize;

    let page_table_entry_2 = page_table_root.get_entry(vpn2);
    if !page_table_entry_2.is_valid() {
        return 0;
    }

    let page_table_level_1 = unsafe { &*(page_table_entry_2.get_ppn().to_physical_address() as *const PageTable) };

    let page_table_entry_1 = page_table_level_1.get_entry(vpn1);
    if !page_table_entry_1.is_valid() {
        return 0;
    }

    let page_table_level_0 = unsafe { &*(page_table_entry_1.get_ppn().to_physical_address() as *const PageTable) };
    
    let page_table_entry_0 = page_table_level_0.get_entry(vpn0);
    if !page_table_entry_0.is_valid() {
        return 0;
    }

    let ppn = page_table_entry_0.get_ppn();
    let physical_address = ppn.to_physical_address() | offset;

    physical_address
}
