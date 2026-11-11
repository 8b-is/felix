use core::fmt::Write;

/// Entry point called from the Felix VM bootstrap.
#[no_mangle]
pub extern "C" fn felix_demo_entry() -> i32 {
    let (n, d) = crate::CURRENT_ROOT_WEIGHT;
    let _ = write!(core::fmt::Buffer::new(),
        "Root weight = {}/{} → {:.3}\n", n, d, n as f64 / d as f64);
    0
}