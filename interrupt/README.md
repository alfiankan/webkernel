# Simple RISC-V Assembly Kernel

A minimal 64-bit RISC-V kernel written entirely in assembly for the QEMU `virt` machine. This project demonstrates the fundamentals of bare-metal development: booting, serial I/O via UART, and handling hardware interrupts.

## Features
- **Pure Assembly:** No C runtime, just raw RV64 instructions.
- **Direct Hardware I/O:** Interacts directly with the 16550A UART.
- **Trap Handling:** Implements a trap vector to catch and process exceptions and interrupts.
- **Interrupt Echo:** Uses the PLIC (Platform Level Interrupt Controller) to receive keyboard input and echo it back.

## Project Structure
- `kernel.S`: The core kernel logic (boot, drivers, trap handler).
- `linker.ld`: Defines the memory layout (starting at `0x80000000`).
- `Makefile`: Automates the cross-compilation and QEMU execution.

## Technical Explanation

### 1. The Boot Process (`_start`)
When QEMU starts, the CPU jumps to `0x80000000`. 
- **Hart Management:** RISC-V systems can have multiple cores (harts). We use `csrr t0, mhartid` to check the core ID. Only core 0 is allowed to continue; others are put into a `wfi` (Wait For Interrupt) loop.
- **Stack Setup:** We initialize the `sp` register to a reserved memory area to allow for function calls and trap handling.

### 2. UART Driver
The `virt` machine maps a 16550A UART at address `0x10000000`.
- **`uart_putc`:** To send a character, we poll the Line Status Register (LSR). Bit 5 tells us if the "Transmit Holding Register" is empty and ready for a new byte.
- **UART Interrupts:** We enable the "Receiver Data Available" interrupt in the UART's Interrupt Enable Register (IER) so the hardware notifies us when you press a key.

### 3. Traps & The "IDT" (`mtvec`)
In RISC-V, interrupts and exceptions are handled through the `mtvec` (Machine Trap Vector) register.
- **Context Saving:** When a trap occurs, the CPU jumps to our `trap_vector`. We must immediately save all 32 general-purpose registers (`x1` to `x31`) to the stack so we can restore the exact state of the kernel after the handler finishes.
- **`mret`:** This special instruction returns from Machine-mode trap handling, restoring the program counter from `mepc`.

### 4. PLIC (Platform Level Interrupt Controller)
The PLIC manages external interrupts (like the UART).
- **Claim/Complete:** To handle an interrupt, we must "claim" it by reading from a specific memory-mapped address in the PLIC. This gives us the ID of the device that triggered the interrupt (IRQ 10 for UART). After processing, we write that ID back to "complete" the interrupt.

## Building and Running

### Prerequisites
- `riscv64-unknown-elf-gcc` toolchain.
- `qemu-system-riscv64`.

### Commands
```bash
# Compile the kernel
make

# Run in QEMU (Ctrl-A X to exit)
make run
```

When running, you should see:
```text
Hello, RISC-V Assembly World!
```
Any character you type will be echoed back by the interrupt handler.
