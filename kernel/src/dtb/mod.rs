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

/// Parses a property node in the Device Tree Blob (DTB).
/// 
/// The FDT_PROP node structure in the DTB contains:
/// - A 4-byte length value (big-endian) indicating property data size.
/// - A 4-byte offset (big-endian) into the strings block for the property name
/// - The actual property data (of the specified length).
/// - Padding to align to a 4-byte boundary.
/// 
/// This function extracts all information for the property and calls the
/// provided callback with the relevant details.
///
/// # Parameters
///
/// * `dtb_header_pointer` - Pointer to the DTB header structure.
/// * `node_address` - Memory address where the property node data begins.
/// * `node_depth` - Current depth in the device tree hierarchy.
/// * `property_callback` - Function to call with the parsed property details:
///   - Property name as a string slice.
///   - Memory address of the property data.
///   - Length of the property data in bytes.
///   - Current node depth in the tree.
///
/// # Returns
///
/// The memory address immediately after this property entry, aligned to a
/// 4-byte boundary.
fn parse_property(
    dtb_header_pointer: *const DtbHeader,
    node_address: usize,
    node_depth: i32,
    property_callback: &impl Fn(&str, usize, usize, i32)
) -> usize {
    let mut current_address = node_address;
    
    // Read data length and name offset. Note that data length can be zero which
    // indicates a boolean property with implicit value of true.
    let data_length = u32::from_be(unsafe { *(current_address as *const u32) });
    current_address += core::mem::size_of::<u32>();
    
    let nameoff = u32::from_be(unsafe { *(current_address as *const u32) });
    current_address += core::mem::size_of::<u32>();
    
    // Get the strings block address.
    let strings_block_offset = u32::from_be(unsafe { &*dtb_header_pointer }.strings_block_offset_be);
    let strings_block_address = dtb_header_pointer as usize + strings_block_offset as usize;
    
    // Get the property name.
    let property_name_address = strings_block_address + nameoff as usize;
    let property_name = read_null_terminated_string(property_name_address);
    
    // Call the property callback.
    property_callback(property_name, current_address, data_length as usize, node_depth);
    
    // Skip property data and align to 4-byte boundary.
    current_address += data_length as usize;
    current_address = (current_address + 3) & !3;
    
    current_address
}

/// Parses a node in the Device Tree Blob (DTB).
/// 
/// This function recursively processes a node in the device tree, including its
/// name, properties, and child nodes. It calls the provided callbacks for each
/// node and property encountered during traversal.
///
/// # Parameters
///
/// * `dtb_header_pointer` - Pointer to the DTB header structure.
/// * `current_address` - Memory address where the node data begins (points to
///   node name).
/// * `node_depth` - Current depth in the device tree hierarchy.
/// * `node_callback` - Function to call with each node's name and depth.
///   - Node name as a string slice.
///   - Current node depth in the tree.
/// * `property_callback` - Function to call with the parsed property details:
///   - Property name as a string slice.
///   - Memory address of the property data.
///   - Length of the property data in bytes.
///   - Current node depth in the tree.
///
/// # Returns
///
/// The memory address immediately after this node and all its children, aligned
/// to a 4-byte boundary.
fn parse_node(
    dtb_header_pointer: *const DtbHeader,
    current_address: usize,
    node_depth: i32,
    node_callback: &impl Fn(&str, i32),
    property_callback: &impl Fn(&str, usize, usize, i32)
) -> usize {
    // Read the node name.
    let node_name = read_null_terminated_string(current_address);
    
    // Call the node callback.
    node_callback(node_name, node_depth);
    
    // Align to 4-byte boundary after the name.
    let mut next_address = current_address + node_name.len() + 1; // +1 for null terminator.
    next_address = (next_address + 3) & !3;
    
    // Start processing tokens after the node name.
    let mut current_address = next_address;
    
    loop {
        let token_address = unsafe { &*(current_address as *const u32) };
        let token = u32::from_be(*token_address);
        current_address += core::mem::size_of::<u32>();
        
        match token {
            FDT_PROP => {
                // Parse property and update address.
                current_address = parse_property(dtb_header_pointer, current_address, node_depth, property_callback);
            },
            FDT_BEGIN_NODE => {
                // Recursively parse a child node.
                current_address = parse_node(
                    dtb_header_pointer,
                    current_address,
                    node_depth + 1,
                    node_callback,
                    property_callback
                );
            },
            FDT_END_NODE => {
                // End of current node.
                return current_address;
            },
            FDT_NOP => {
                // Nothing to do for NOP tokens.
            },
            FDT_END => {
                // End of entire tree - should not happen within a node.
                debug_println!("Unexpected FDT_END token within node.");
                return current_address;
            },
            _ => {
                debug_println!("Unexpected token: {}", token);
            }
        }
    }
}

/// Traverses the structure block of a Device Tree Blob (DTB).
/// 
/// This function walks through the structure block in a DTB, which contains
/// nodes and their properties arranged in a hierarchical tree structure. It
/// processes FDT_BEGIN_NODE tokens to parse nodes and their children
/// recursively, FDT_NOP tokens which are ignored, and stops when encountering
/// an FDT_END token.
///
/// The function invokes the provided callbacks for each node and property
/// encountered during traversal, allowing the caller to process the device tree
/// information as needed in an allocation free way.
///
/// # Parameters
///
/// * `dtb_header_pointer` - Pointer to the DTB header structure.
/// * `node_callback` - Function to call with each node's name and depth:
///   - Node name as a string slice.
///   - Current node depth in the tree.
/// * `property_callback` - Function to call with the parsed property details:
///   - Property name as a string slice.
///   - Memory address of the property data.
///   - Length of the property data in bytes.
///   - Current node depth in the tree.
///
/// # Examples
///
/// ```
/// walk_structure_block(
///     dtb_header_ptr,
///     |name, depth| println!("Node: {} at depth {}", name, depth),
///     |name, data_adress, data_length, depth| println!("Property: {} at depth {}", name, depth)
/// );
/// ```
pub fn walk_structure_block(
    dtb_header_pointer: *const DtbHeader,
    node_callback: impl Fn(&str, i32),
    property_callback: impl Fn(&str, usize, usize, i32)
) {
    // Convert the DTB header pointer to a DtbHeader reference.
    let dtb_header = unsafe { &*dtb_header_pointer };

    // Calculate the structure block address.
    let structure_block_offset = u32::from_be(dtb_header.structure_block_offset_be);
    let structure_block_address = dtb_header_pointer as usize + structure_block_offset as usize;

    // Walk the structure block.
    let mut current_address = structure_block_address;

    loop {
        let token_address = unsafe { &*(current_address as *const u32) };
        let token = u32::from_be(*token_address);

        current_address += core::mem::size_of::<u32>();

        match token {
            FDT_BEGIN_NODE => {
                // Parse this node and all its children.
                current_address = parse_node(
                    dtb_header_pointer, 
                    current_address, 
                    0, 
                    &node_callback, 
                    &property_callback
                );
            },
            FDT_NOP => {
                // Nothing to do for NOP tokens.
            },
            FDT_END => {
                // End of the structure block.
                break;
            },
            _ => {
                debug_println!("Unexpected token at structure block root: {}", token);
                break;
            }
        }
    }
}
