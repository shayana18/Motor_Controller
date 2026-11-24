#![no_std]
use core::panic::PanicInfo;
use core::arch::global_asm;

pub union Vector {
    reserved : u32, 
    handler : unsafe extern "C" fn() -> (),
}

unsafe extern "C" {
    pub fn NMI();
    pub fn HardFault();
    pub fn MemManageFault();
    pub fn MemBusFault();
    pub fn UsageFault();
    pub fn SvCall();
    pub fn PendSV();
    pub fn SysTick();
}

#[unsafe(link_section = "vector_table.exceptions")]
#[unsafe(no_mangle)]
pub static EXCEPTIONS : [Vector; 14] = [
    Vector {handler : NMI},
    Vector {handler : HardFault},
    Vector {handler : MemManageFault},
    Vector {handler : MemBusFault},
    Vector {handler : UsageFault},
    Vector {reserved : 0},
    Vector {reserved : 0},
    Vector {reserved : 0},
    Vector {reserved : 0},
    Vector {handler : SvCall},
    Vector {reserved : 0},
    Vector {reserved : 0},
    Vector {handler : PendSV},
    Vector {handler : SysTick},
];

// this macro generates a function "__main" that interally uses a temp variable assignment to make a compile time check on the params and outputs of the passed in function which is also our main function. 
#[macro_export]
macro_rules! entry {
    ($path:path) => {
        #[unsafe(export_name = "main")] // on thing to note here is taht in the reset handler when we call main, it is actualling calling this function (__main) as we have specified the symbol of this to be "main". 
                                        // the main function in our main.c is not what is directly called by the invociation of this symbol
        pub unsafe fn __main() -> ! {
            // below we make a temp variable f that has a type hint of a function pointer that points to a function that 
            // takes no parameters and returns nothing. we try to assign the passed in parameter to this variable that has this enforced type 
            // thus if our passed in parameter doesnt match it becomes a compile time error
            let f: fn() -> ! = $path;

            f()
        }
    };
}

unsafe extern "C" {
    fn reset_handler() ->!; 
}

global_asm!(
    ".text

     .syntax unified
     .global _sbss
     .global _ebss

     .global _sdata
     .global _edata
     .global _sidata

     .global main
     .global reset_handler

     .type reset_handler,%function
     .thumb_func
     reset_handler:

     _init_bss:
         movs r2, #0
         ldr r0, =_sbss
         ldr r1, =_ebss

     1:
         cmp r1, r0
         beq _init_data
         strb r2, [r0]
         add r0, #1
         b 1b

     _init_data:
         ldr r0, =_sdata
         ldr r1, =_edata
         ldr r2, =_sidata

     1:
         cmp r0, r1
         beq _main_trampoline
         ldrb r3, [r2]
         strb r3, [r0]
         add r0, #1
         add r2, #1
         b 1b
     _main_trampoline:
         ldr r0, =main
         bx r0"
);


#[unsafe(link_section = ".vector_table.reset_vector")]
#[unsafe(no_mangle)]
pub static RESET_VECTOR: unsafe extern "C" fn() -> ! = reset_handler;

#[panic_handler]
#[inline(never)]
fn panic(_panic: &PanicInfo<'_>) -> !{
    loop{}
}

#[unsafe(no_mangle)]
pub extern "C" fn DefaultExceptionHandler() {
    loop {}
}
