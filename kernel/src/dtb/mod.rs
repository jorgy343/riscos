//! Device Tree Blob (DTB) parser module.
//!
//! This module provides functionality to parse and traverse a Devicetree Blob
//! (DTB) in accordance with the Devicetree Specification without allocating
//! onto the heap. It includes capabilities to:
//! - Walk through memory reservation entries.
//! - Traverse the structure block containing nodes and properties.
//! - Parse individual nodes and properties.
//! - Extract and interpret cell values (address/size).

#![allow(dead_code)]

use core::cell::Cell;

use crate::debug_println;

//=============================================================================
// Constants
//=============================================================================

/// FDT token indicating the beginning of a node.
const FDT_BEGIN_NODE: u32 = 1;
/// FDT token indicating the end of a node.
const FDT_END_NODE: u32 = 2;
/// FDT token indicating a property definition.
const FDT_PROP: u32 = 3;
/// FDT token used for padding.
const FDT_NOP: u32 = 4;
/// FDT token indicating the end of the structure block.
const FDT_END: u32 = 9;

//=============================================================================
// Data Structures
//=============================================================================

/// Header of a Device Tree Blob.
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

impl DtbHeader {
    // Returns the memory reservation block address relative to the DTB header
    // base.
    pub fn memory_reservation_block_address(&self) -> usize {
        let base = self as *const _ as usize;
        base + u32::from_be(self.memory_reservation_block_offset_be) as usize
    }

    // Returns the structure block address relative to the DTB header base.
    pub fn structure_block_address(&self) -> usize {
        let base = self as *const _ as usize;
        base + u32::from_be(self.structure_block_offset_be) as usize
    }
    
    // Returns the strings block address relative to the DTB header base.
    pub fn strings_block_address(&self) -> usize {
        let base = self as *const _ as usize;
        base + u32::from_be(self.strings_block_offset_be) as usize
    }
}

/// Represents an entry in the memory reservation block of a Device Tree Blob.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DtbMemoryReservationEntry {
    /// This field shall contain the address of the memory region.
    pub address: u64,

    /// This field shall contain the size of the memory region.
    pub size: u64,
}

/// Represents property information from a Device Tree Blob.
#[derive(Debug, Clone, Copy)]
pub struct DtbProperty<'a> {
    /// Name of the property.
    pub name: &'a str,
    /// Memory address where the property data begins.
    pub data_address: usize,
    /// Length of the property data in bytes.
    pub data_length: usize,
}

impl<'a> DtbProperty<'a> {
    /// Parses a u32 value from the property data.
    pub fn parse_u32_from_property(&self) -> u32 {
        u32::from_be(unsafe { *(self.data_address as *const u32) })
    }
}

/// Represents the address and size cells information for a node.
#[derive(Debug, Clone, Copy)]
pub struct CellInfo {
    /// Number of 32-bit cells used to represent addresses in child nodes.
    pub address_cells: u32,
    /// Number of 32-bit cells used to represent sizes in child nodes.
    pub size_cells: u32,
}

impl Default for CellInfo {
    fn default() -> Self {
        // Default values according to the DTB specification.
        Self {
            address_cells: 2,
            size_cells: 1,
        }
    }
}

//=============================================================================
// Core Traversal Functions
//=============================================================================

/// Traverses memory reservation entries in a Device Tree Blob.
///
/// Walks through all memory reservation entries in the DTB, calling the
/// provided callback function for each entry until the terminating entry (with
/// both address and size set to 0) is encountered.
///
/// # Parameters
///
/// * `dtb_header_pointer` - Pointer to the DTB header.
/// * `callback` - Function to call for each memory reservation entry.
pub fn walk_memory_reservation_entries(dtb_header: &DtbHeader, callback: impl Fn(&DtbMemoryReservationEntry)) {
    let memory_reservation_block_address = dtb_header.memory_reservation_block_address();

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
///     |name, data_address, data_length, depth| println!("Property: {} at depth {}", name, depth)
/// );
/// ```
pub fn walk_structure_block(
    dtb_header: &DtbHeader,
    node_callback: impl Fn(&str, i32),
    property_callback: impl Fn(&DtbProperty, &CellInfo, i32)
) {
    let structure_block_address = dtb_header.structure_block_address();

    // Walk the structure block with default cell info for the root.
    let mut current_address = structure_block_address;
    let default_cells_info = CellInfo::default();

    loop {
        let token_address = unsafe { &*(current_address as *const u32) };
        let token = u32::from_be(*token_address);

        current_address += core::mem::size_of::<u32>();

        match token {
            FDT_BEGIN_NODE => {
                // Parse this node and all its children.
                current_address = parse_node(
                    dtb_header, 
                    current_address, 
                    0, 
                    default_cells_info,
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

//=============================================================================
// Node and Property Parsing
//=============================================================================

/// Parses a property node in the Device Tree Blob (DTB).
/// 
/// The FDT_PROP node structure in the DTB contains:
/// - A 4-byte length value (big-endian) indicating property data size.
/// - A 4-byte offset (big-endian) into the strings block for the property name
/// - The actual property data (of the specified length).
/// - Padding to align to a 4-byte boundary.
/// 
/// This function extracts all information for the property and returns a
/// DtbProperty structure containing the details.
///
/// # Parameters
///
/// * `dtb_header` - Reference to the DTB header structure.
/// * `node_address` - Memory address where the property node data begins.
///
/// # Returns
///
/// A tuple containing:
/// - The DtbProperty struct with property information.
/// - The memory address immediately after this property entry, aligned to a
///   4-byte boundary.
fn parse_property(
    dtb_header: &DtbHeader,
    node_address: usize,
) -> (DtbProperty<'static>, usize) {
    let mut current_address = node_address;
    
    // Read data length and name offset. Note that data length can be zero which
    // indicates a boolean property with implicit value of true.
    let data_length = u32::from_be(unsafe { *(current_address as *const u32) });
    current_address += core::mem::size_of::<u32>();
    
    let nameoff = u32::from_be(unsafe { *(current_address as *const u32) });
    current_address += core::mem::size_of::<u32>();
    
    // Get the strings block address using the helper method
    let strings_block_address = dtb_header.strings_block_address();
    
    // Get the property name.
    let property_name_address = strings_block_address + nameoff as usize;
    let property_name = read_null_terminated_string(property_name_address);
    
    let property = DtbProperty {
        name: property_name,
        data_address: current_address,
        data_length: data_length as usize,
    };
    
    // Skip property data and align to 4-byte boundary.
    current_address += data_length as usize;
    current_address = (current_address + 3) & !3;
    
    (property, current_address)
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
/// * `parent_cells_info` - Address and size cells information from the parent node.
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
    dtb_header: &DtbHeader,
    current_address: usize,
    node_depth: i32,
    parent_cells_info: CellInfo,
    node_callback: &impl Fn(&str, i32),
    property_callback: &impl Fn(&DtbProperty, &CellInfo, i32)
) -> usize {
    // Read the node name.
    let node_name = read_null_terminated_string(current_address);
    
    // Initialize with parent's cell info, will be updated if this node has its
    // own values.
    let mut current_cells_info = parent_cells_info;
    
    // Call the node callback.
    node_callback(node_name, node_depth);
    
    // Align to 4-byte boundary after the name.
    let mut current_address = current_address + node_name.len() + 1; // +1 for null terminator.
    current_address = (current_address + 3) & !3;
    
    loop {
        let token_address = unsafe { &*(current_address as *const u32) };
        let token = u32::from_be(*token_address);
        current_address += core::mem::size_of::<u32>();
        
        match token {
            FDT_PROP => {
                let (property, next_address) = parse_property(dtb_header, current_address);

                if property.name == "#address-cells" {
                    current_cells_info.address_cells = property.parse_u32_from_property();
                } else if property.name == "#size-cells" {
                    current_cells_info.size_cells = property.parse_u32_from_property();
                }
                
                property_callback(&property, &current_cells_info, node_depth);
                
                current_address = next_address;
            },
            FDT_BEGIN_NODE => {
                // Recursively parse a child node with current node's cells
                // info.
                current_address = parse_node(
                    dtb_header,
                    current_address,
                    node_depth + 1,
                    current_cells_info,
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
                // End of entire tree - should not happen while node parsing.
                debug_println!("Unexpected FDT_END token within node.");
                return current_address;
            },
            _ => {
                debug_println!("Unexpected token: {}", token);

                // Try to recover by returning current address.
                return current_address;
            }
        }
    }
}

/// Reads a null-terminated string from the given address.
/// 
/// This function reads a null-terminated string from the provided memory
/// address and returns it as a string slice.
/// 
/// # Parameters
/// 
/// * `address` - Memory address where the string begins.
/// 
/// # Returns
/// 
/// A string slice containing the null-terminated string.
/// 
/// # Safety
/// 
/// This function is unsafe because it dereferences a raw pointer.
/// 
/// # Examples
/// 
/// ```
/// let string = read_null_terminated_string(address);
/// ```
fn read_null_terminated_string(address: usize) -> &'static str {
    // Find the string length by locating the null terminator.
    let mut length = 0;
    while unsafe { *((address + length) as *const u8) } != 0 {
        length += 1;
    }

    // Convert the byte sequence to a string slice.
    unsafe { 
        core::str::from_utf8_unchecked(
            core::slice::from_raw_parts(address as *const u8, length)
        )
    }
}
