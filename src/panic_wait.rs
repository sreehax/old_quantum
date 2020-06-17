use crate::{bsp, cpu};
use core::{fmt, panic::PanicInfo};

fn _panic_print(args: fmt::Arguments) {
    use fmt::Write;
    unsafe { bsp::console::panic_console_out().write_fmt(args).unwrap() };
}
#[macro_export]
macro_rules! panic_println {
    ($($arg:tt)*) => ({
        _panic_print(format_args_nl!($($arg)*));
    })
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(args) = info.message() {
        panic_println!("\nFatal error: {}", args);
    } else {
        panic_println!("\nFatal error!");
    }
    cpu::wait_forever()
}
