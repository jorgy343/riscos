use super::sbi_call_3;

const DEBUG_CONSOLE_EXTENSION_ID: i32 = 0x4442434E;

const CONSOLE_WRITE_ID: i32 = 0x0;

#[inline(always)]
pub fn sbi_debug_console_write(buffer: &[u8]) -> (isize, usize) {
    let num_bytes = buffer.len();
    let buffer_addr = buffer.as_ptr() as usize;

    sbi_call_3(
        DEBUG_CONSOLE_EXTENSION_ID as isize,
        CONSOLE_WRITE_ID as isize,
        num_bytes,
        buffer_addr,
        0
    )
}
