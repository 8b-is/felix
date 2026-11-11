#![no_std]
/// Global weight cell (p, q) used by the Felix VM.
/// SAFETY: This mutable static is only accessed from the single‑threaded
/// Felix VM, so the usual Sync restriction does not apply.
static mut CURRENT_ROOT_WEIGHT: (u32, u32) = (7, 1);

/// Read the current root weight as a tuple (numerator, denominator).
pub fn current_root_weight() -> (u32, u32) {
    unsafe { CURRENT_ROOT_WEIGHT }
}

/// Write a new reduced root weight (p, q). The fraction is reduced by
/// dividing by the greatest common divisor.
pub fn set_root_weight(p: u32, q: u32) {
    // Simple GCD for u32 – no std required.
    fn gcd(mut a: u32, mut b: u32) -> u32 {
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a
    }
    let g = gcd(p, q);
    unsafe {
        CURRENT_ROOT_WEIGHT = (p / g, q / g);
    }
}