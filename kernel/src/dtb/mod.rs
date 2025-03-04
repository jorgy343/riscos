#![allow(dead_code)]

use crate::{debug_print, debug_println};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DtbHeader {
    /// This field shall contain the value 0xd00dfeed (big-endian).
    pub magic: u32,

    /// This field shall contain the total size in bytes of the devicetree data
    /// structure, encompassing all sections: the header, memory reservation
    /// block, structure block, strings block, and any free space gaps between
    /// or after blocks.
    pub total_size: u32,

    /// This field shall contain the offset in bytes of the structure block from
    /// the beginning of the header.
    pub structure_block_offset: u32,

    /// This field shall contain the offset in bytes of the strings block from
    /// the beginning of the header.
    pub strings_block_offset: u32,

    /// This field shall contain the offset in bytes of the memory reservation
    /// block from the beginning of the header.
    pub memory_reservation_block_offset: u32,

    /// This field shall contain the version of the devicetree data structure.
    /// The version is 17 if using the structure as defined in this document.
    pub version: u32,

    /// This field shall contain the lowest version with which the current
    /// version is backwards compatible. For version 17, this field shall
    /// contain 16.
    pub last_compatible_version: u32,

    /// This field shall contain the physical ID of the system's boot CPU,
    /// identical to the physical ID given in the reg property of that CPU node
    /// within the devicetree.
    pub boot_physical_cpuid: u32,

    /// This field shall contain the length in bytes of the strings block
    /// section of the devicetree blob.
    pub strings_block_size: u32,

    /// This field shall contain the length in bytes of the structure block
    /// section of the devicetree blob.
    pub structure_block_size: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DtbMemoryReservationEntry {
    /// This field shall contain the address of the memory region.
    pub address: u64,

    /// This field shall contain the size of the memory region.
    pub size: u64,
}

pub fn walk_memory_reservation_entries(dtb_header_pointer: *const DtbHeader, callback: impl Fn(&DtbMemoryReservationEntry)) {
    // Convert the DTB header pointer to a DtbHeader reference.
    let dtb_header = unsafe { &*dtb_header_pointer };

    // Calculate the memory reservation block address. The DTB header fields are
    // stored in big-endian format, so we need to convert them.
    let memory_reservation_block_offset = u32::from_be(dtb_header.memory_reservation_block_offset);
    let memory_reservation_block_address = dtb_header_pointer as usize + memory_reservation_block_offset as usize;

    let mut index = 0;
    loop {
        let memory_reservation_entry_address = memory_reservation_block_address + index * core::mem::size_of::<DtbMemoryReservationEntry>();
        let memory_reservation_entry = unsafe { &*(memory_reservation_entry_address as *const DtbMemoryReservationEntry) };

        // The last entry in the list will have an address and size of 0.
        if memory_reservation_entry.address == 0 && memory_reservation_entry.size == 0 {
            break;
        }

        callback(memory_reservation_entry);

        index += 1;
    }
}
