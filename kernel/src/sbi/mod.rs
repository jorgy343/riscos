pub mod debug_console;

#[inline(always)]
fn sbi_call_1(extension_id: isize, function_id: isize, arg0: usize) -> (isize, usize) {
    let error: isize;
    let value: usize;
    
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") arg0,
            in("a6") function_id,
            in("a7") extension_id,
            lateout("a0") error,
            lateout("a1") value,
        );
    }
    
    (error, value)
}

#[inline(always)]
fn sbi_call_2(extension_id: isize, function_id: isize, arg0: usize, arg1: usize) -> (isize, usize) {
    let error: isize;
    let value: usize;
    
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") arg0,
            in("a1") arg1,
            in("a6") function_id,
            in("a7") extension_id,
            lateout("a0") error,
            lateout("a1") value,
        );
    }
    
    (error, value)
}

#[inline(always)]
fn sbi_call_3(extension_id: isize, function_id: isize, arg0: usize, arg1: usize, arg2: usize) -> (isize, usize) {
    let error: isize;
    let value: usize;
    
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a6") function_id,
            in("a7") extension_id,
            lateout("a0") error,
            lateout("a1") value,
        );
    }
    
    (error, value)
}