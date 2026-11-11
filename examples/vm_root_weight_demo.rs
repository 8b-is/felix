use std::io::{self, Write};

/// Entry point called from the Felix VM bootstrap.
#[no_mangle]
pub extern "C" fn felix_demo_entry() -> i32 {
    let (n, d) = felix_vm::current_root_weight();
    let scaled_milli = (u64::from(n) * 1000) / u64::from(d.max(1));
    let whole = scaled_milli / 1000;
    let frac = scaled_milli % 1000;
    let _ = writeln!(
        io::stderr(),
        "Root weight = {n}/{d} -> {whole}.{frac:03} (p/q exact)"
    );
    0
}

fn main() {
    std::process::exit(felix_demo_entry());
}
