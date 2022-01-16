#[macro_export]
macro_rules! debug_log {
    () => (
        let DEBUG_MODE = false;
        if DEBUG_MODE {
            print!("\n")
        });
    ($fmt:expr) => (
        let DEBUG_MODE = false;
        if DEBUG_MODE {
            print!(concat!($fmt, "\n"))
        });
    ($fmt:expr, $($arg:tt)*) => (
        let DEBUG_MODE = false;
        if DEBUG_MODE {
            print!(concat!($fmt, "\n"), $($arg)*)
        });
}
