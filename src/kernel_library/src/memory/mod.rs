pub mod memory_map;
pub mod mmu;
pub mod physical_memory_allocator;

/// Represents a physical page number (PPN).
///
/// This is the top 44 bits of a 56-bit physical address. The structure stores
/// the PPN with bit 0 representing the start of the PPN (the address
/// right-shifted by 12 bits), as it does not include the 12-bit page offset.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysicalPageNumber(pub usize);

impl PhysicalPageNumber {
    /// Get the raw physical page number.
    ///
    /// # Returns
    ///
    /// The raw physical page number. That is, the 56-bit physical address
    /// right-shifted by 12 bits.
    pub const fn raw_ppn(&self) -> usize {
        self.0
    }

    /// Create a new `PhysicalPageNumber` from a physical address.
    ///
    /// # Arguments
    ///
    /// * `physical_address` - The physical address which will be right shifted
    ///   by 12 bits to get the PPN. The lower 12 bits are lost. This is
    ///   equivalent to rounding down the physical address to the nearest 4KiB
    ///   boundary.
    ///
    /// # Returns
    ///
    /// The `PhysicalPageNumber` representing the top 44 bits of the physical
    /// address.
    ///
    /// # Example
    ///
    /// ```
    /// let physical_address = 0x8020_0123;
    /// let ppn = PhysicalPageNumber::from(physical_address);
    ///
    /// assert_eq!(ppn.0, 0x0008_0200);
    /// ```
    pub const fn from_physical_address(physical_address: usize) -> Self {
        Self(physical_address >> 12)
    }

    /// Create a new `PhysicalPageNumber` from a raw physical page number
    /// typically coming from a page table entry.
    ///
    /// # Arguments
    ///
    /// * `ppn` - The 44-bit physical page number.
    ///
    /// # Returns
    ///
    /// The `PhysicalPageNumber` representing the top 44 bits of the physical
    /// address.
    pub const fn from_raw_physical_page_number(ppn: usize) -> Self {
        Self(ppn)
    }

    /// Get the physical address this `PhysicalPageNumber` represents. The
    /// physical address represents the address pointing to the first byte of a
    /// 4KiB page.
    ///
    /// # Returns
    ///
    /// The physical address with the PPN shifted left by 12 bits. The resultant
    /// physical address is guaranteed to be aligned to a 4KiB boundary.
    pub const fn to_physical_address(&self) -> usize {
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtualPageNumber(pub usize);

impl VirtualPageNumber {
    /// Get the raw virtual page number.
    ///
    /// # Returns
    ///
    /// The raw virtual page number. That is, the 39-bit virtual address
    /// right-shifted by 12 bits.
    pub const fn raw_vpn(&self) -> usize {
        self.0
    }

    /// Create a new `VirtualPageNumber` from a virtual address.
    ///
    /// # Arguments
    ///
    /// * `virtual_address` - The virtual address which is right shifted by 12
    ///   bits to get the VPN. The lower 12 bits are lost. This is equivalent to
    ///   rounding down the virtual address to the nearest 4KiB boundary.
    ///
    /// # Returns
    ///
    /// The `VirtualPageNumber` representing the top 27 bits of the virtual
    /// address.
    pub const fn from_virtual_address(virtual_address: usize) -> Self {
        Self(virtual_address >> 12)
    }

    /// Create a new `VirtualPageNumber` from a raw virtual page number
    /// typically coming from a page table entry.
    ///
    /// # Arguments
    ///
    /// * `vpn` - The 27-bit virtual page number.
    ///
    /// # Returns
    ///
    /// The `VirtualPageNumber` representing the top 27 bits of the virtual
    /// address.
    pub const fn from_raw_virtual_page_number(vpn: usize) -> Self {
        Self(vpn)
    }

    /// Get the virtual address this `VirtualPageNumber` represents. The virtual
    /// address represents the address pointing to the first byte of a 4KiB
    /// page.
    ///
    /// # Returns
    ///
    /// The virtual address with the VPN shifted left by 12 bits. The resultant
    /// virtual address is guaranteed to be aligned to a 4KiB boundary.
    pub const fn to_virtual_address(&self) -> usize {
        self.0 << 12
    }

    /// Get the index for the level 2 page table (root page table).
    ///
    /// In sv39 paging mode, virtual addresses have 27 bits for the VPN split
    /// into 3 levels of 9 bits each. This method extracts the highest 9 bits
    /// (bits 26-18) which represent the index into the level 2 page table.
    ///
    /// # Returns
    ///
    /// The 9-bit index for the level 2 page table, suitable for indexing into a
    /// page table array.
    pub const fn get_level_2_index(&self) -> usize {
        ((self.0 >> 18) & 0x1FF) as usize
    }

    /// Get the index for the level 1 page table (middle page table).
    ///
    /// In sv39 paging mode, this method extracts the middle 9 bits (bits 17-9)
    /// which represent the index into the level 1 page table.
    ///
    /// # Returns
    ///
    /// The 9-bit index for the level 1 page table, suitable for indexing into a
    /// page table array.
    pub const fn get_level_1_index(&self) -> usize {
        ((self.0 >> 9) & 0x1FF) as usize
    }

    /// Get the index for the level 0 page table (lowest page table).
    ///
    /// In sv39 paging mode, this method extracts the lowest 9 bits (bits 8-0)
    /// which represent the index into the level 0 page table.
    ///
    /// # Returns
    ///
    /// The 9-bit index for the level 0 page table, suitable for indexing into a
    /// page table array.
    pub const fn get_level_0_index(&self) -> usize {
        (self.0 & 0x1FF) as usize
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    pub start: usize,
    pub size: usize,
}

impl MemoryRegion {
    /// Creates a new memory region with the specified start address and size.
    ///
    /// # Parameters
    ///
    /// * `start` - The start address of the memory region.
    /// * `size` - The size of the memory region in bytes.
    ///
    /// # Returns
    ///
    /// A new memory region instance.
    pub const fn new(start: usize, size: usize) -> Self {
        MemoryRegion { start, size }
    }

    /// Returns the inclusive end address of the memory region.
    ///
    /// # Returns
    ///
    /// The inclusive end address of the memory region. If the size is zero,
    /// returns zero.
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

    mod physical_page_number_tests {
        use super::*;

        #[test]
        fn test_raw_ppn() {
            // Standard case.
            let ppn = PhysicalPageNumber(0x1234_5678);
            assert_eq!(ppn.raw_ppn(), 0x1234_5678);

            // Zero value.
            let ppn = PhysicalPageNumber(0);
            assert_eq!(ppn.raw_ppn(), 0);

            // Maximum 44-bit value.
            let max_ppn = 0x0FFF_FFFF_FFFF; // 44 bits all set to 1.
            let ppn = PhysicalPageNumber(max_ppn);
            assert_eq!(ppn.raw_ppn(), max_ppn);
        }

        #[test]
        fn test_from_physical_address() {
            // Standard case.
            let physical_addr = 0x8020_1000;
            let ppn = PhysicalPageNumber::from_physical_address(physical_addr);
            assert_eq!(ppn.0, physical_addr >> 12);

            // Address with page offset.
            let physical_addr_with_offset = 0x8020_1ABC;
            let ppn = PhysicalPageNumber::from_physical_address(physical_addr_with_offset);
            assert_eq!(ppn.0, 0x8020_1);

            // Zero address.
            let ppn = PhysicalPageNumber::from_physical_address(0);
            assert_eq!(ppn.0, 0);

            // Maximum physical address (56 bits).
            let max_physical_addr = 0x00FF_FFFF_FFFF_FFFF;
            let ppn = PhysicalPageNumber::from_physical_address(max_physical_addr);
            assert_eq!(ppn.0, max_physical_addr >> 12);
        }

        #[test]
        fn test_from_raw_physical_page_number() {
            // Standard case.
            let raw_ppn = 0x1234_5678;
            let ppn = PhysicalPageNumber::from_raw_physical_page_number(raw_ppn);
            assert_eq!(ppn.0, raw_ppn);

            // Zero value.
            let ppn = PhysicalPageNumber::from_raw_physical_page_number(0);
            assert_eq!(ppn.0, 0);

            // Maximum 44-bit PPN.
            let max_ppn = 0x0FFF_FFFF_FFFF; // 44 bits all set to 1.
            let ppn = PhysicalPageNumber::from_raw_physical_page_number(max_ppn);
            assert_eq!(ppn.0, max_ppn);
        }

        #[test]
        fn test_to_physical_address() {
            // Standard case.
            let ppn = PhysicalPageNumber(0x1234);
            assert_eq!(ppn.to_physical_address(), 0x1234 << 12);

            // Zero PPN.
            let ppn = PhysicalPageNumber(0);
            assert_eq!(ppn.to_physical_address(), 0);

            // Maximum 44-bit PPN.
            let max_ppn = 0x0FFF_FFFF_FFFF; // 44 bits all set to 1.
            let ppn = PhysicalPageNumber(max_ppn);
            assert_eq!(ppn.to_physical_address(), max_ppn << 12);

            // Verify round-trip conversion.
            let original_addr = 0x8000_0000;
            let page_aligned_addr = original_addr & !0xFFF; // Clear the lower 12 bits.
            let ppn = PhysicalPageNumber::from_physical_address(original_addr);
            assert_eq!(ppn.to_physical_address(), page_aligned_addr);
        }

        #[test]
        fn test_conversions_round_trip() {
            // Test a round trip conversion from physical address to PPN and
            // back.
            let addresses = [
                0x0000_1000,
                0x8020_3000,
                0xFFFF_F000,
                0x00FF_FFFF_FFFF_F000, // Maximum 56-bit address aligned to page.
            ];

            for addr in addresses.iter() {
                let ppn = PhysicalPageNumber::from_physical_address(*addr);
                let recovered_addr = ppn.to_physical_address();
                // The recovered address should match the original with cleared
                // lower 12 bits.
                assert_eq!(recovered_addr, *addr & !0xFFF);
            }
        }
    }

    mod virtual_page_number_tests {
        use super::*;

        #[test]
        fn test_raw_vpn() {
            // Standard case.
            let vpn = VirtualPageNumber(0x1234_5678);
            assert_eq!(vpn.raw_vpn(), 0x1234_5678);

            // Zero value.
            let vpn = VirtualPageNumber(0);
            assert_eq!(vpn.raw_vpn(), 0);

            // Maximum 27-bit value for sv39.
            let max_vpn = 0x0777_FFFF; // 27 bits all set to 1.
            let vpn = VirtualPageNumber(max_vpn);
            assert_eq!(vpn.raw_vpn(), max_vpn);
        }

        #[test]
        fn test_from_virtual_address() {
            // Standard case.
            let virtual_addr = 0x8020_1000;
            let vpn = VirtualPageNumber::from_virtual_address(virtual_addr);
            assert_eq!(vpn.0, virtual_addr >> 12);

            // Address with page offset.
            let virtual_addr_with_offset = 0x8020_1ABC;
            let vpn = VirtualPageNumber::from_virtual_address(virtual_addr_with_offset);
            assert_eq!(vpn.0, 0x8020_1);

            // Zero address.
            let vpn = VirtualPageNumber::from_virtual_address(0);
            assert_eq!(vpn.0, 0);

            // Maximum sv39 virtual address (sign extended 39 bits).
            let max_virtual_addr = 0x0000_007F_FFFF_FFFF;
            let vpn = VirtualPageNumber::from_virtual_address(max_virtual_addr);
            assert_eq!(vpn.0, max_virtual_addr >> 12);
        }

        #[test]
        fn test_from_raw_virtual_page_number() {
            // Standard case.
            let raw_vpn = 0x0123_4567;
            let vpn = VirtualPageNumber::from_raw_virtual_page_number(raw_vpn);
            assert_eq!(vpn.0, raw_vpn);

            // Zero value.
            let vpn = VirtualPageNumber::from_raw_virtual_page_number(0);
            assert_eq!(vpn.0, 0);

            // Maximum 27-bit VPN for sv39.
            let max_vpn = 0x0777_FFFF; // 27 bits all set to 1.
            let vpn = VirtualPageNumber::from_raw_virtual_page_number(max_vpn);
            assert_eq!(vpn.0, max_vpn);
        }

        #[test]
        fn test_to_virtual_address() {
            // Standard case.
            let vpn = VirtualPageNumber(0x1234);
            assert_eq!(vpn.to_virtual_address(), 0x1234 << 12);

            // Zero VPN.
            let vpn = VirtualPageNumber(0);
            assert_eq!(vpn.to_virtual_address(), 0);

            // Maximum 27-bit VPN for sv39.
            let max_vpn = 0x0777_FFFF; // 27 bits all set to 1.
            let vpn = VirtualPageNumber(max_vpn);
            assert_eq!(vpn.to_virtual_address(), max_vpn << 12);
        }

        #[test]
        fn test_page_table_indices() {
            // Test case 1: VPN with maximum values in each field.
            let vpn = VirtualPageNumber((0x1FF << 18) | (0x1FF << 9) | 0x1FF);
            // All levels should have index 0x1FF (511).
            assert_eq!(vpn.get_level_2_index(), 0x1FF);
            assert_eq!(vpn.get_level_1_index(), 0x1FF);
            assert_eq!(vpn.get_level_0_index(), 0x1FF);

            // Test case 2: Zero VPN.
            let vpn = VirtualPageNumber(0);
            assert_eq!(vpn.get_level_2_index(), 0);
            assert_eq!(vpn.get_level_1_index(), 0);
            assert_eq!(vpn.get_level_0_index(), 0);

            // Test case 3: VPN with specific bit patterns.
            let vpn = VirtualPageNumber(0b110_101010_111000111_101010101);
            assert_eq!(vpn.get_level_2_index(), 0b110_101010);
            assert_eq!(vpn.get_level_1_index(), 0b111000111);
            assert_eq!(vpn.get_level_0_index(), 0b101010101);
        }

        #[test]
        fn test_conversions_round_trip() {
            // Test a round trip conversion from virtual address to VPN and
            // back.
            let addresses = [
                0x0000_1000,
                0x8020_3000,
                0x7FFF_F000, // Maximum sv39 address aligned to page.
            ];

            for addr in addresses.iter() {
                let vpn = VirtualPageNumber::from_virtual_address(*addr);
                let recovered_addr = vpn.to_virtual_address();
                // The recovered address should match the original with cleared
                // lower 12 bits.
                assert_eq!(recovered_addr, *addr & !0xFFF);
            }
        }
    }
}
