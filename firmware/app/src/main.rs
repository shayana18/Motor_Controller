#![no_std]
#![no_main]

use rt::entry; 
use core::arch::asm;

entry!(main);

// always note rust uses name mangling like C++, if we want a specific symbol name we specify it via export name or 
// if we want it to match the assigned name we use no_mangle 
pub fn main() -> ! {
    let _x = 42;
    unsafe { asm!("udf #0", options(noreturn)) };
    // loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn HardFault() -> !
{
    let _hard_fault = -1; 
    loop{};
}


