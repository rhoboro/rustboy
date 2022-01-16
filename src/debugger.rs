#[macro_export]
macro_rules! debug_log {
    () => (
        let debug_mode = false;
        if debug_mode {
            print!("\n")
        });
    ($fmt:expr) => (
        let debug_mode = false;
        if debug_mode {
            print!(concat!($fmt, "\n"))
        });
    ($fmt:expr, $($arg:tt)*) => (
        let debug_mode = false;
        if debug_mode {
            print!(concat!($fmt, "\n"), $($arg)*)
        });
}
