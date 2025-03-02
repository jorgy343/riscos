#![allow(dead_code)]

/// Represents a physical page number (PPN).
/// 
/// This is the top 44 bits of a 56-bit physical address. The structure stores
/// the PPN with bit 0 representing the start of the PPN (the address
/// right-shifted by 12 bits), as it does not include the 12-bit page offset.
pub struct PhysicalPageNumber(u64);

impl PhysicalPageNumber {
    pub const fn new(ppn: u64) -> Self {
        Self(ppn)
    }
}

impl From<u64> for PhysicalPageNumber {
    /// Create a new `PhysicalPageNumber` from a physical address.
    /// 
    /// # Arguments
    /// * `physical_address` - The physical address which is right shifted by 12
    ///   bits to get the PPN. The lower 12 bits are lost. This is equivalent to
    ///   rounding down the physical address to the nearest 4KiB boundary.
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
    fn from(physical_address: u64) -> Self {
        Self(physical_address >> 12)
    }
}

impl Into<u64> for PhysicalPageNumber {
    /// Convert the `PhysicalPageNumber` into a physical address.
    /// 
    /// # Returns
    /// The physical address with the PPN shifted left by 12 bits. The resultant
    /// physical address is guaranteed to be aligned to a 4KiB boundary.
    /// 
    /// # Example
    /// ```
    /// let ppn = PhysicalPageNumber::new(0x0008_0200);
    /// let physical_address: u64 = ppn.into();
    /// 
    /// assert_eq!(physical_address, 0x8020_0000);
    /// ```
    fn into(self) -> u64 {
        self.0 << 12
    }
}

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

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn get_ppn(&self) -> u64 {
        (self.0 >> 10) & 0xFFF_FFFF_FFFF
    }

    pub fn set_ppn(&mut self, ppn: u64) {
        // Clear the old PPN and set the new one.
        self.0 = (self.0 & !0x003F_FFFF_FFF0) | ((ppn & 0xFFF_FFFF_FFFF) << 10);
    }

    pub fn is_valid(&self) -> bool {
        self.0 & Self::FLAG_VALID != 0
    }

    pub fn set_valid(&mut self, valid: bool) {
        if valid {
            self.0 |= Self::FLAG_VALID;
        } else {
            self.0 &= !Self::FLAG_VALID;
        }
    }

    pub fn is_readable(&self) -> bool {
        self.0 & Self::FLAG_READ != 0
    }

    pub fn set_readable(&mut self, readable: bool) {
        if readable {
            self.0 |= Self::FLAG_READ;
        } else {
            self.0 &= !Self::FLAG_READ;
        }
    }

    pub fn is_writable(&self) -> bool {
        self.0 & Self::FLAG_WRITE != 0
    }

    pub fn set_writable(&mut self, writable: bool) {
        if writable {
            self.0 |= Self::FLAG_WRITE;
        } else {
            self.0 &= !Self::FLAG_WRITE;
        }
    }

    pub fn is_executable(&self) -> bool {
        self.0 & Self::FLAG_EXECUTE != 0
    }

    pub fn set_executable(&mut self, executable: bool) {
        if executable {
            self.0 |= Self::FLAG_EXECUTE;
        } else {
            self.0 &= !Self::FLAG_EXECUTE;
        }
    }

    pub fn is_user(&self) -> bool {
        self.0 & Self::FLAG_USER != 0
    }

    pub fn set_user(&mut self, user: bool) {
        if user {
            self.0 |= Self::FLAG_USER;
        } else {
            self.0 &= !Self::FLAG_USER;
        }
    }

    pub fn is_global(&self) -> bool {
        self.0 & Self::FLAG_GLOBAL != 0
    }

    pub fn set_global(&mut self, global: bool) {
        if global {
            self.0 |= Self::FLAG_GLOBAL;
        } else {
            self.0 &= !Self::FLAG_GLOBAL;
        }
    }

    pub fn is_accessed(&self) -> bool {
        self.0 & Self::FLAG_ACCESSED != 0
    }

    pub fn set_accessed(&mut self, accessed: bool) {
        if accessed {
            self.0 |= Self::FLAG_ACCESSED;
        } else {
            self.0 &= !Self::FLAG_ACCESSED;
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.0 & Self::FLAG_DIRTY != 0
    }

    pub fn set_dirty(&mut self, dirty: bool) {
        if dirty {
            self.0 |= Self::FLAG_DIRTY;
        } else {
            self.0 &= !Self::FLAG_DIRTY;
        }
    }

    pub fn is_leaf(&self) -> bool {
        // An entry is a leaf if it's valid and has at least one of R, W, or X bits set.
        self.is_valid() && (self.is_readable() || self.is_writable() || self.is_executable())
    }
}

#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    pub const fn new() -> Self {
        Self {
            entries: [const { PageTableEntry::new() }; 512],
        }
    }

    pub fn get_entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    pub fn get_entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }

    pub fn set_entry(&mut self, index: usize, entry: PageTableEntry) {
        self.entries[index] = entry;
    }
}
