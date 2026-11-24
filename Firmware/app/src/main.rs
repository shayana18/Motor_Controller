#![no_main]
#![no_std]
use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn reset_handler() -> ! {
    let _x = 42;
    loop{}
}

#[unsafe(link_section = ".vector_table.reset_vector")]
#[unsafe(no_mangle)]
pub static RESET_VECTOR: unsafe extern "C" fn() -> ! = reset_handler;

#[panic_handler]
#[inline(never)]
fn panic(_panic: &PanicInfo<'_>) -> !{
    loop{}
}
