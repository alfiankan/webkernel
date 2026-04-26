# Design Spec: RISC-V Assembly Kernel (Hello World + Keyboard Interrupts)

## Overview
A minimal 64-bit RISC-V kernel written entirely in assembly for the QEMU `virt` machine. The kernel runs in Machine Mode (M-Mode) and demonstrates basic UART I/O, trap handling, and interrupt management.

## Architecture
- **Target:** RISC-V 64-bit (RV64)
- **Platform:** QEMU `virt` machine
- **Privilege Level:** Machine Mode (M-Mode)
- **Memory Map:**
  - UART0: `0x10000000` (16550A compatible)
  - PLIC (Platform Level Interrupt Controller): `0x0c000000`
  - RAM Base: `0x80000000`

## Components

### 1. Bootloader & Initialization (`kernel.S`)
- Entry point at `_start` (linked to `0x80000000`).
- Hart Check: Only hart 0 continues; others enter a `wfi` (Wait For Interrupt) loop.
- Stack Setup: `sp` initialized to a 4KB internal buffer.
- Global Pointer: `gp` initialized for relaxed addressing (if applicable).

### 2. UART Driver
- **`uart_init`**: Configures the 16550A UART (baud rate, LCR, FCR).
- **`uart_putc(char)`**: Polls LSR bit 5 and writes to THR.
- **`uart_getc() -> char`**: Reads from RBR when data is available.

### 3. Trap Handling (The "IDT")
- **`mtvec`**: Set to `trap_vector` address (Direct mode).
- **`trap_vector`**: 
  - Saves all 32 registers to the stack.
  - Calls `handle_trap`.
  - Restores registers.
  - Returns with `mret`.
- **`handle_trap`**:
  - Reads `mcause`.
  - If `mcause` indicates a Machine External Interrupt (bit 63 set, code 11):
    - Polls PLIC to claim the interrupt.
    - If from UART (IRQ 10), calls `uart_getc` and echoes the character.
    - Completes PLIC interrupt.

### 4. Main Loop
- Calls `uart_init`.
- Prints "Hello, RISC-V World!\n".
- Configures PLIC to enable UART interrupts (IRQ 10).
- Enables interrupts in `mstatus` and `mie`.
- Enters an infinite `wfi` loop waiting for keyboard input.

## Build & Run
- **Toolchain:** `riscv64-unknown-elf-gcc` (or similar)
- **QEMU Command:**
  ```bash
  qemu-system-riscv64 -M virt -cpu rv64 -nographic -kernel kernel.elf
  ```
