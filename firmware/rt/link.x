/* Memory layout of the ST32G4 series. Uses arm Cortex -M4 with FPU */
/* Reference: 2.2.2 in G4 mcu data sheet */

MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 128K
  RAM : ORIGIN = 0x20000000, LENGTH = 32K
}

/* The entry point is the reset handler */
ENTRY(reset_handler);

EXTERN(RESET_VECTOR);
EXTERN(EXCEPTIONS);
/* Specifying how we want input memory sectiosn to be mapped over into the output memory */
SECTIONS
{
  .vector_table ORIGIN(FLASH) :
  {
    /* First entry: initial Stack Pointer value */
    LONG(ORIGIN(RAM) + LENGTH(RAM));

    /* Second entry: reset vector */
    KEEP(*(.vector_table.reset_vector));

    /* Third and on entries are vectors for exception handling and interrupt handling */
    KEEP(*(.vector_table.exceptions));

  } > FLASH

  .text :
  {
    *(.text .text.*);
  } > FLASH

    .rodata :
  {
    *(.rodata .rodata.*);
  } > FLASH

  .bss :
  {
    _sbss = .;
    *(.bss .bss.*);
    _ebss = .;
  } > RAM

  .data : AT(ADDR(.rodata) + SIZEOF(.rodata)) /* setting Load memory address (LMA) of .data to after .rodata */
  {
    _sdata = .; /* boundary marker for the VMA of .data as the actual initialized bytes live in flash but since its not a const it has a virtual addressing via RAM */
    *(.data .data.*);
    _edata = .; 
  } > RAM /* is the execution location */

  _sidata = LOADADDR(.data); /* boundary marking marking the actual mem location where the .data was initialized and through this boundary marker 
                              reset_handler can copy over initialized .data symbols to the VMA of RAM */

  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
  }
}

PROVIDE(NMI = DefaultExceptionHandler);
PROVIDE(HardFault = DefaultExceptionHandler);
PROVIDE(MemManageFault = DefaultExceptionHandler);
PROVIDE(MemBusFault = DefaultExceptionHandler);
PROVIDE(UsageFault = DefaultExceptionHandler);
PROVIDE(SvCall = DefaultExceptionHandler);
PROVIDE(PendSV = DefaultExceptionHandler);
PROVIDE(SysTick = DefaultExceptionHandler);