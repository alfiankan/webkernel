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

.align 4
trap_vector:
    # Save context (32 regs * 8 bytes = 256 bytes)
    addi sp, sp, -256
    sd ra, 0(sp)
    sd sp, 8(sp)
    sd gp, 16(sp)
    sd tp, 24(sp)
    sd t0, 32(sp)
    sd t1, 40(sp)
    sd t2, 48(sp)
    sd s0, 56(sp)
    sd s1, 64(sp)
    sd a0, 72(sp)
    sd a1, 80(sp)
    sd a2, 88(sp)
    sd a3, 96(sp)
    sd a4, 104(sp)
    sd a5, 112(sp)
    sd a6, 120(sp)
    sd a7, 128(sp)
    sd s2, 136(sp)
    sd s3, 144(sp)
    sd s4, 152(sp)
    sd s5, 160(sp)
    sd s6, 168(sp)
    sd s7, 176(sp)
    sd s8, 184(sp)
    sd s9, 192(sp)
    sd s10, 200(sp)
    sd s11, 208(sp)
    sd t3, 216(sp)
    sd t4, 224(sp)
    sd t5, 232(sp)
    sd t6, 240(sp)

    call handle_trap

    # Restore context
    ld ra, 0(sp)
    # Skip sp
    ld gp, 16(sp)
    ld tp, 24(sp)
    ld t0, 32(sp)
    ld t1, 40(sp)
    ld t2, 48(sp)
    ld s0, 56(sp)
    ld s1, 64(sp)
    ld a0, 72(sp)
    ld a1, 80(sp)
    ld a2, 88(sp)
    ld a3, 96(sp)
    ld a4, 104(sp)
    ld a5, 112(sp)
    ld a6, 120(sp)
    ld a7, 128(sp)
    ld s2, 136(sp)
    ld s3, 144(sp)
    ld s4, 152(sp)
    ld s5, 160(sp)
    ld s6, 168(sp)
    ld s7, 176(sp)
    ld s8, 184(sp)
    ld s9, 192(sp)
    ld s10, 200(sp)
    ld s11, 208(sp)
    ld t3, 216(sp)
    ld t4, 224(sp)
    ld t5, 232(sp)
    ld t6, 240(sp)
    addi sp, sp, 256
    mret

handle_trap:
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
