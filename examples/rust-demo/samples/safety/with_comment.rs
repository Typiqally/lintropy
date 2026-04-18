pub fn checked_read(ptr: *const i32) -> i32 {
    // SAFETY: this sample documents the invariant immediately above the block.
    unsafe { *ptr }
}
