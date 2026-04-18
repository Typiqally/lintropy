pub fn unchecked_read(ptr: *const i32) -> i32 {
    unsafe { *ptr }
}
