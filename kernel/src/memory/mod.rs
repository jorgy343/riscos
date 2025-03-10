#![allow(dead_code)]

pub mod bump_allocator;
pub mod mmu;
pub mod memory_map;

/// Represents a physical page number (PPN).
/// 
/// This is the top 44 bits of a 56-bit physical address. The structure stores
/// the PPN with bit 0 representing the start of the PPN (the address
/// right-shifted by 12 bits), as it does not include the 12-bit page offset.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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