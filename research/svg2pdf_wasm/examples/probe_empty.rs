//! Size probe baseline: an empty cdylib with the same profile.

#[unsafe(no_mangle)]
pub extern "C" fn probe() -> usize {
    0
}
