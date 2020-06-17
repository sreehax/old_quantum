use crate::{bsp, cpu};
use cortex_a::{asm, regs::*};

#[naked]
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    use crate::runtime_init;

    if bsp::cpu::BOOT_CORE_ID == cpu::smp::core_id() {
        SP.set(bsp::cpu::BOOT_CORE_STACK_START);
        runtime_init::runtime_init()
    } else {
        wait_forever()
    }
}

pub use asm::nop;

#[inline(always)]
pub fn spin_for_cycles(n: usize) {
    for _ in 0..n {
        asm::nop();
    }
}

#[inline(always)]
pub fn wait_forever() -> ! {
    loop {
        asm::wfe();
    }
}