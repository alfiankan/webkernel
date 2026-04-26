# RISC-V Assembly Kernel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a simple 64-bit RISC-V kernel in assembly that prints "Hello World", handles keyboard interrupts, and echoes input.

**Architecture:** Minimal M-mode kernel for QEMU `virt` machine. Direct UART I/O and PLIC-based interrupts.

**Tech Stack:** RISC-V Assembly (RV64), QEMU, GNU toolchain (`riscv64-unknown-elf-gcc`).

---

### Task 1: Linker Script

**Files:**
- Create: `linker.ld`

- [ ] **Step 1: Create the linker script**

```ld
OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
  . = 0x80000000;

  .text : {
    *(.text .text.*)
  }

  .rodata : {
    *(.rodata .rodata.*)
  }

  .data : {
    *(.data .data.*)
  }

  .bss : {
    *(.bss .bss.*)
  }

  PROVIDE(_stack_top = . + 0x1000);
}
```

- [ ] **Step 2: Commit**

```bash
git add linker.ld
git commit -m "feat: add linker script"
```

### Task 2: Minimal Boot & UART Print

**Files:**
- Create: `kernel.S`

- [ ] **Step 1: Write boot code and basic UART output**

```s
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
```

- [ ] **Step 2: Commit**

```bash
git add kernel.S
git commit -m "feat: add boot code and UART print"
```

### Task 3: Build & Run Infrastructure

**Files:**
- Create: `Makefile`

- [ ] **Step 1: Create Makefile**

```makefile
CC = riscv64-unknown-elf-gcc
AS = riscv64-unknown-elf-as
LD = riscv64-unknown-elf-ld
OBJCOPY = riscv64-unknown-elf-objcopy

CFLAGS = -Wall -Wextra -mcmodel=medany -ffreestanding -nostdlib -mabi=lp64 -march=rv64imafdc
LDFLAGS = -T linker.ld

kernel.elf: kernel.S linker.ld
	$(CC) $(CFLAGS) $(LDFLAGS) kernel.S -o kernel.elf

run: kernel.elf
	qemu-system-riscv64 -M virt -cpu rv64 -nographic -kernel kernel.elf

clean:
	rm -f kernel.elf
```

- [ ] **Step 2: Verify build and run**

Run: `make run`
Expected: Output "Hello, RISC-V Assembly World!" in the terminal. (Press Ctrl-A X to exit QEMU).

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "feat: add Makefile"
```

### Task 4: Trap Vector & Register Saving

**Files:**
- Modify: `kernel.S`

- [ ] **Step 1: Add register saving/restoring and trap vector**

```s
# Add to .text section
.align 4
trap_vector:
    # Save context (32 regs * 8 bytes = 256 bytes)
    addi sp, sp, -256
    sd ra, 0(sp)
    sd sp, 8(sp) # Note: saving old sp is tricky, usually done differently, but for M-mode it's okay
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
```

- [ ] **Step 2: Commit**

```bash
git add kernel.S
git commit -m "feat: add trap vector boilerplate"
```

### Task 5: PLIC & Interrupt Handling

**Files:**
- Modify: `kernel.S`

- [ ] **Step 1: Define PLIC constants and add trap handling logic**

```s
# Constants
.equ PLIC_BASE, 0x0c000000
.equ PLIC_PRIORITY_UART, (PLIC_BASE + 10*4)
.equ PLIC_PENDING, (PLIC_BASE + 0x1000)
.equ PLIC_ENABLE, (PLIC_BASE + 0x2000)
.equ PLIC_THRESHOLD, (PLIC_BASE + 0x200000)
.equ PLIC_CLAIM, (PLIC_BASE + 0x200004)

.equ UART_IER, 1
.equ UART_RBR, 0
.equ UART_LSR_RX_READY, 0x01

# Add to handle_trap
handle_trap:
    addi sp, sp, -16
    sd ra, 0(sp)

    csrr t0, mcause
    li t1, 0x800000000000000b # Machine external interrupt
    bne t0, t1, 2f

    # Claim interrupt
    li t0, PLIC_CLAIM
    lw t1, 0(t0)

    # Check if from UART (IRQ 10)
    li t2, 10
    bne t1, t2, 1f

    # Echo character
    li t3, UART0
    lbu a0, UART_RBR(t3)
    call uart_putc

1:
    # Complete interrupt
    li t0, PLIC_CLAIM
    sw t1, 0(t0)

2:
    ld ra, 0(sp)
    addi sp, sp, 16
    ret
```

- [ ] **Step 2: Update kmain to enable interrupts**

```s
kmain:
    # Init UART interrupts
    li t0, UART0
    li t1, 1
    sb t1, UART_IER(t0)

    # Init PLIC
    li t0, PLIC_PRIORITY_UART
    li t1, 1
    sw t1, 0(t0)

    li t0, PLIC_ENABLE
    li t1, (1 << 10)
    sw t1, 0(t0)

    li t0, PLIC_THRESHOLD
    sw zero, 0(t0)

    # Set trap vector
    la t0, trap_vector
    csrw mtvec, t0

    # Enable interrupts
    li t0, 0x800 # MEIE bit in mie
    csrs mie, t0
    li t0, 0x8   # MIE bit in mstatus
    csrs mstatus, t0

    la a0, hello_msg
    call print_string

1:
    wfi
    j 1b
```

- [ ] **Step 3: Verify full functionality**

Run: `make run`
Expected: "Hello, RISC-V Assembly World!" appears. Typing characters in the terminal should echo them back.

- [ ] **Step 4: Commit**

```bash
git add kernel.S
git commit -m "feat: implement interrupt handling and echoing"
```
