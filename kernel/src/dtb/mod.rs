#![allow(dead_code)]

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
