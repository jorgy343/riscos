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

pub fn walk_structure_block(dtb_header_pointer: *const DtbHeader) {
    // Convert the DTB header pointer to a DtbHeader reference.
    let dtb_header = unsafe { &*dtb_header_pointer };

    // Calculate the structure block address. The DTB header fields are stored
    // in big-endian format, so we need to convert them.
    let structure_block_offset = u32::from_be(dtb_header.structure_block_offset);
    let structure_block_address = dtb_header_pointer as usize + structure_block_offset as usize;

    // Walk the structure block.
    let mut current_address = structure_block_address;
    let mut current_node_depth = 0;

    loop {
        let token_address = unsafe { &*(current_address as *const u32) };
        let token = u32::from_be(*token_address);

        current_address += core::mem::size_of::<u32>();

        if token == 1 {
            // The FDT_BEGIN_NODE token (1) is followed by the unit name as a null-terminated string.
            // The string is padded to a multiple of 4 bytes.
            let mut byte_address = current_address;
            let mut name_length = 0;

            // First, find the end of the null-terminated string
            loop {
                let byte = unsafe { *(byte_address as *const u8) };
                if byte == 0 {
                    break;
                }
                byte_address += 1;
                name_length += 1;
            }

            // Print the node name directly as bytes
            debug_print!("Node: ");
            for i in 0..name_length {
                let byte = unsafe { *((current_address + i) as *const u8) };
                debug_print!("{}", byte as char);
            }
            debug_println!(", depth: {}", current_node_depth);

            // Move the current address to the next 4-byte aligned position after the null-terminated string
            current_address += name_length + 1;
            current_address = (current_address + 3) & !3;
            
            current_node_depth += 1;
        } else if token == 2 {
            current_node_depth -= 1;

            debug_println!("end node, depth: {}", current_node_depth);
        } else if token == 3 {
            // FDT_PROP token
            // First read the property length and nameoff
            let prop_len_be = unsafe { *(current_address as *const u32) };
            let prop_len = u32::from_be(prop_len_be);
            current_address += core::mem::size_of::<u32>();

            let nameoff_be = unsafe { *(current_address as *const u32) };
            let nameoff = u32::from_be(nameoff_be); // Not used yet but read for future use
            current_address += core::mem::size_of::<u32>();

            // Get the property name from the strings block
            let strings_block_offset = u32::from_be(dtb_header.strings_block_offset);
            let strings_block_address = dtb_header_pointer as usize + strings_block_offset as usize;
            let prop_name_address = strings_block_address + nameoff as usize;

            // Read the property name as a null-terminated string
            let mut byte_address = prop_name_address;
            let mut name_length = 0;
            loop {
                let byte = unsafe { *(byte_address as *const u8) };
                if byte == 0 {
                    break;
                }
                byte_address += 1;
                name_length += 1;
            }

            // Print the property name
            debug_print!("  Property: ");
            for i in 0..name_length {
                let byte = unsafe { *((prop_name_address + i) as *const u8) };
                debug_print!("{}", byte as char);
            }

            // Skip the property data
            if prop_len > 0 {
                debug_println!("|Length: {}",  prop_len);
                current_address += prop_len as usize;
            } else {
                debug_println!("|Length: 0");
            }

            // Align to 4-byte boundary
            current_address = (current_address + 3) & !3;
        } else if token == 4 { // NOP token. Do nothing.
            
        } else if token == 9 { // End of block token.
            break;
        }
    }
}
