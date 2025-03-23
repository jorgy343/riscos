use super::sbi_calls::sbi_call_3;
use core::fmt::{self, Write};

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
        0,
    )
}

/// A formatter that writes directly to the SBI debug console.
pub struct DebugConsoleWriter;

impl Write for DebugConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        sbi_debug_console_write(s.as_bytes());
        Ok(())
    }
}

/// Prints formatted text to the SBI debug console without heap allocations.
///
/// This macro works similar to `format!` but writes directly to the debug
/// console.
///
/// # Examples
///
/// ```
/// debug_print!("Hello, {}!", "world");
/// debug_println!("Value = {}", 42);
/// ```
#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        use $crate::sbi::debug_console::DebugConsoleWriter;
        let _ = write!(DebugConsoleWriter, $($arg)*);
    }};
}

/// Prints formatted text to the SBI debug console, followed by a newline.
///
/// This macro works similar to `format!` but writes directly to the debug
/// console.
///
/// # Examples
///
/// ```
/// debug_println!("Hello, {}!", "world");
/// debug_println!("Value = {}", 42);
/// ```
#[macro_export]
macro_rules! debug_println {
    () => {
        $crate::debug_print!("\n")
    };
    ($($arg:tt)*) => {{
        $crate::debug_print!($($arg)*);
        $crate::debug_print!("\n");
    }};
}
