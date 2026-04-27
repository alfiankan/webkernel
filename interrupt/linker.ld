OUTPUT_ARCH( "riscv" )
ENTRY( _start )

MEMORY
{
  /* QEMU virt machine RAM starts at 0x80000000 */
  ram (rwx) : ORIGIN = 0x80000000, LENGTH = 128M
}

SECTIONS
{
  /* Start of RAM */
  . = 0x80000000;

  .text : {
    *(.text)
  } > ram

  .rodata : {
    *(.rodata)
  } > ram

  .data : {
    *(.data)
  } > ram

  .bss : {
    *(.bss)
  } > ram
}

