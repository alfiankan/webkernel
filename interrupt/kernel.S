# Constants
.equ UART0, 0x10000000
.equ UART_THR, 0
.equ UART_LSR, 5
.equ LSR_TX_EMPTY, 0x20

.section .text
.global _start

_start:
    # Only hart 0 continues
    csrr t0, mhartid
    bnez t0, park_hart

    # Setup stack
    la sp, _stack_top

    # Jump to kernel main
    j kmain

park_hart:
    wfi
    j park_hart

uart_putc:
    li t0, UART0
1:
    lbu t1, UART_LSR(t0)
    andi t1, t1, LSR_TX_EMPTY
    beqz t1, 1b
    sb a0, UART_THR(t0)
    ret

print_string:
    addi sp, sp, -16
    sd ra, 0(sp)
    sd s0, 8(sp)
    mv s0, a0
1:
    lbu a0, 0(s0)
    beqz a0, 2f
    call uart_putc
    addi s0, s0, 1
    j 1b
2:
    ld ra, 0(sp)
    ld s0, 8(sp)
    addi sp, sp, 16
    ret

kmain:
    la a0, hello_msg
    call print_string
1:
    wfi
    j 1b

.section .rodata
hello_msg:
    .string "Hello, RISC-V Assembly World!\n"
