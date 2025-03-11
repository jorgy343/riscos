#![allow(dead_code)]

use kernel_library::memory::memory_map::MemoryMap;

use crate::dtb::{DtbHeader, walk_structure_block};
use core::cell::RefCell;

/// Populates a memory map with memory regions described in the Device Tree
/// Blob.
///
/// This function walks through the DTB structure, looking for memory nodes and
/// their "reg" properties which describe available memory ranges. Each memory
/// region is aligned to 4KiB boundaries before being added to the memory map:
/// - Start addresses are rounded up to the nearest 4KiB boundary.
/// - End addresses are rounded down to the nearest 4KiB boundary.
/// - Regions that become smaller than 4KiB after alignment are discarded.
///
/// # Parameters
///
/// * `memory_map` - The memory map to populate with memory regions.
/// * `dtb_header` - Reference to the Device Tree Blob header.
pub fn populate_memory_map_from_dtb(memory_map: &mut MemoryMap, dtb_header: &DtbHeader) {
    // Constants for 4KiB alignment in the Sv39 paging scheme.
    const PAGE_SIZE: usize = 4096;
    const PAGE_MASK: usize = !(PAGE_SIZE - 1);

    walk_structure_block(
        dtb_header,
        |_, _| {},
        |node, property, cells_info, _depth| {
            if node.name != "memory" && !node.name.starts_with("memory@") {
                return;
            }

            // Only process "reg" properties that are inside memory nodes.
            if property.name == "reg" {
                // Extract memory regions from the reg property.
                property.get_property_data_as_reg(&cells_info, |address, size| {
                    let original_start = address as usize;
                    let original_size = size as usize;

                    // Align the start address up to the next 4KiB boundary.
                    let aligned_start = (original_start + PAGE_SIZE - 1) & PAGE_MASK;

                    // Calculate how much the alignment changed the start
                    // position.
                    let start_adjustment = aligned_start - original_start;

                    // Adjust the size by subtracting the start adjustment.
                    let adjusted_size = if start_adjustment < original_size {
                        original_size - start_adjustment
                    } else {
                        // If start adjustment exceeded original size, region
                        // vanishes.
                        0
                    };

                    // Align the size down to a multiple of 4KiB.
                    let aligned_size = adjusted_size & PAGE_MASK;

                    // Only add regions that are at least 4KiB in size after
                    // alignment.
                    if aligned_size >= PAGE_SIZE {
                        memory_map.add_region(aligned_start, aligned_size);
                    }
                });
            }
        },
    );
}

/// Adjusts a memory map by removing regions marked as reserved in the Device
/// Tree Blob.
///
/// This function walks through the DTB structure looking for the
/// "reserved-memory" node and its children. For each child node with a "reg"
/// property, it extracts the address and size information of the reserved
/// memory region and removes it from the available memory map by calling
/// `remove_reserved_memory_region`.
///
/// Reserved memory regions are used by firmware, bootloaders, or other system
/// components and should not be used by the operating system. This ensures that
/// the memory map only contains memory that is safe to use.
///
/// # Parameters
///
/// * `memory_map` - The memory map to adjust by removing reserved regions.
/// * `dtb_header` - Reference to the Device Tree Blob header containing the
///   reserved memory information.
///
/// # Side Effects
///
/// This function modifies the provided memory map by potentially removing
/// regions, adjusting region boundaries, or adding new regions when splitting
/// is required.
pub fn adjust_memory_map_from_reserved_regions_in_dtb(
    memory_map: &mut MemoryMap,
    dtb_header: &DtbHeader,
) {
    // Track if we're inside a reserved-memory node to process its children
    let inside_reserved_memory = RefCell::new(false);

    walk_structure_block(
        dtb_header,
        |node, depth| {
            // Check if we're entering the reserved-memory node
            if node.name == "reserved-memory" || node.name.starts_with("reserved-memory@") {
                *inside_reserved_memory.borrow_mut() = true;
            }
            // Check if we're leaving the reserved-memory node (entering a different node at the same level)
            else if depth == 1 && *inside_reserved_memory.borrow() {
                *inside_reserved_memory.borrow_mut() = false;
            }
        },
        |_, property, cells_info, depth| {
            // Process reg properties in child nodes of reserved-memory
            if *inside_reserved_memory.borrow() && depth > 1 && property.name == "reg" {
                property.get_property_data_as_reg(&cells_info, |address, size| {
                    let reserved_start = address as usize;
                    let reserved_size = size as usize;

                    memory_map.carve_out_region(reserved_start, reserved_size);
                });
            }
        },
    );
}
