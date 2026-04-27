.equ UART_BASE, 0x10000000

.section .text
.globl _start

_start:
    # Load the base address of the UART into register a0
    li a0, UART_BASE

    # Load the address of the "hello_msg" string into register a1
    la a1, hello_msg

print_loop:
    # Load a single byte (character) from the string into register a2
    lb a2, 0(a1)

    # If the byte is 0 (null terminator), branch to 'halt'
    beqz a2, halt

    # Store the byte into the UART memory address to print it
    sb a2, 0(a0)

    # Increment the string pointer to the next character
    addi a1, a1, 1

    # Jump back to the start of the loop
    j print_loop

halt:
    # Wait for interrupt (puts the CPU in a low-power state)
    wfi
    # Loop infinitely
    j halt

.section .rodata
hello_msg:
    .asciz "Boot, RISC-V Bare-Metal World!\n"
