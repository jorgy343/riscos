#![allow(dead_code)]

use crate::{debug_print, debug_println};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DtbHeader {
    /// This field shall contain the value 0xd00dfeed (big-endian).
    pub magic_be: u32,

    /// This field shall contain the total size in bytes of the devicetree data
    /// structure, encompassing all sections: the header, memory reservation
    /// block, structure block, strings block, and any free space gaps between
    /// or after blocks.
    pub total_size_be: u32,

    /// This field shall contain the offset in bytes of the structure block from
    /// the beginning of the header.
    pub structure_block_offset_be: u32,

    /// This field shall contain the offset in bytes of the strings block from
    /// the beginning of the header.
    pub strings_block_offset_be: u32,

    /// This field shall contain the offset in bytes of the memory reservation
    /// block from the beginning of the header.
    pub memory_reservation_block_offset_be: u32,

    /// This field shall contain the version of the devicetree data structure.
    /// The version is 17 if using the structure as defined in this document.
    pub version_be: u32,

    /// This field shall contain the lowest version with which the current
    /// version is backwards compatible. For version 17, this field shall
    /// contain 16.
    pub last_compatible_version_be: u32,

    /// This field shall contain the physical ID of the system's boot CPU,
    /// identical to the physical ID given in the reg property of that CPU node
    /// within the devicetree.
    pub boot_physical_cpuid_be: u32,

    /// This field shall contain the length in bytes of the strings block
    /// section of the devicetree blob.
    pub strings_block_size_be: u32,

    /// This field shall contain the length in bytes of the structure block
    /// section of the devicetree blob.
    pub structure_block_size_be: u32,
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
    let memory_reservation_block_offset = u32::from_be(dtb_header.memory_reservation_block_offset_be);
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

// Constant values for FDT tokens.
const FDT_BEGIN_NODE: u32 = 1;
const FDT_END_NODE: u32 = 2;
const FDT_PROP: u32 = 3;
const FDT_NOP: u32 = 4;
const FDT_END: u32 = 9;

/// Reads a null-terminated string from the given address.
fn read_null_terminated_string(address: usize) -> &'static str {
    let mut byte_address = address;
    let mut name_length = 0;

    // Find the end of the null-terminated string.
    while unsafe { *(byte_address as *const u8) } != 0 {
        byte_address += 1;
        name_length += 1;
    }

    // Convert the byte sequence to a str.
    unsafe { 
        core::str::from_utf8_unchecked(
            core::slice::from_raw_parts(address as *const u8, name_length)
        )
    }
}

/// Read properties from a node in the device tree.
pub fn read_node_properties(
    dtb_header_pointer: *const DtbHeader,
    node_address: usize, 
    property_callback: impl Fn(&str, usize, usize)
) -> usize {
    let dtb_header = unsafe { &*dtb_header_pointer };
    
    // Get the address of the strings block.
    let strings_block_offset = u32::from_be(dtb_header.strings_block_offset_be);
    let strings_block_address = dtb_header_pointer as usize + strings_block_offset as usize;
    
    // Skip the node's FDT_BEGIN_NODE token and name.
    let mut current_address = node_address + core::mem::size_of::<u32>();
    let node_name = read_null_terminated_string(current_address);
    
    // Align to 4-byte boundary after the name.
    current_address += node_name.len() + 1; // +1 for null terminator
    current_address = (current_address + 3) & !3;
    
    loop {
        let token_address = unsafe { &*(current_address as *const u32) };
        let token = u32::from_be(*token_address);
        current_address += core::mem::size_of::<u32>();
        
        match token {
            FDT_PROP => {
                // Read property length and name offset.
                let prop_len = u32::from_be(unsafe { *(current_address as *const u32) });
                current_address += core::mem::size_of::<u32>();
                
                let nameoff = u32::from_be(unsafe { *(current_address as *const u32) });
                current_address += core::mem::size_of::<u32>();
                
                // Get the property name from the strings block.
                let prop_name_address = strings_block_address + nameoff as usize;
                let prop_name = read_null_terminated_string(prop_name_address);
                
                // Property data starts at current_address.
                let prop_data_address = current_address;
                
                // Call the callback with property information.
                property_callback(prop_name, prop_data_address, prop_len as usize);
                
                // Skip property data and align to 4-byte boundary.
                current_address += prop_len as usize;
                current_address = (current_address + 3) & !3;
            },
            FDT_BEGIN_NODE => {
                // We found a child node, return current position to allow caller to process it.
                return current_address - core::mem::size_of::<u32>();
            },
            FDT_END_NODE | FDT_END => {
                // End of this node's properties.
                return current_address - core::mem::size_of::<u32>();
            },
            FDT_NOP => {
                // Nothing to do for NOP tokens.
            },
            _ => {
                debug_println!("Unknown FDT token: {}", token);
            }
        }
    }
}

pub fn walk_structure_block(dtb_header_pointer: *const DtbHeader) {
    // Convert the DTB header pointer to a DtbHeader reference.
    let dtb_header = unsafe { &*dtb_header_pointer };

    // Calculate the structure block address. The DTB header fields are stored
    // in big-endian format, so we need to convert them.
    let structure_block_offset = u32::from_be(dtb_header.structure_block_offset_be);
    let structure_block_address = dtb_header_pointer as usize + structure_block_offset as usize;

    // Walk the structure block.
    let mut current_address = structure_block_address;
    let mut current_node_depth = 0;

    loop {
        let token_address = unsafe { &*(current_address as *const u32) };
        let token = u32::from_be(*token_address);

        current_address += core::mem::size_of::<u32>();

        if token == FDT_BEGIN_NODE {
            // Read the node name.
            let node_name = read_null_terminated_string(current_address);

            print_indent(current_node_depth);
            debug_println!("Node: {}", node_name);

            // Align to 4-byte boundary after the name.
            current_address += node_name.len() + 1; // +1 for null terminator.
            current_address = (current_address + 3) & !3;
            
            // Process all properties of this node.
            current_address = read_node_properties(
                dtb_header_pointer, 
                current_address - core::mem::size_of::<u32>() - (node_name.len() + 1),
                |prop_name, prop_data, prop_len| {
                    print_indent(current_node_depth);
                    debug_println!("  Property: {} | Length: {}", prop_name, prop_len);
                }
            );

            current_node_depth += 1;
        } else if token == FDT_END_NODE {
            current_node_depth -= 1;
        } else if token == FDT_END {
            break;
        }
    }
}

fn print_indent(node_depth: i32) {
    for _ in 0..node_depth {
        debug_print!("  ");
    }
}
