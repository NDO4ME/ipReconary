use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

static CTRL_C_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn setup_handler(stop_flag: Arc<AtomicBool>) {
    ctrlc::set_handler(move || {
        let count = CTRL_C_COUNT.fetch_add(1, Ordering::SeqCst) + 1;

        if count == 1 {
            eprintln!("\n[!] Stopping gracefully... Press Ctrl+C again to force exit.");
            stop_flag.store(true, Ordering::SeqCst);
        } else {
            eprintln!("\n[!] Force exit.");
            std::process::exit(1);
        }
    })
    .expect("Failed to set Ctrl+C handler");
}
