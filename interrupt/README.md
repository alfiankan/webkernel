# RISC-V Bare-Metal Kernel

This project is a minimal "Hello World" kernel written for the RISC-V architecture. It bypasses all operating system abstractions to talk directly to the hardware.

## High-Level Overview: The "Raw" Handshake

If you usually write in Python, JavaScript, or even C++, you're used to a **Environment** (a Runtime or an OS) that provides tools like `console.log` or `std::cout`. 

In this project, there is no environment. We are building the environment.

*   **`kernel.S` (The Assembly)** is the **Instruction Manual**. It tells the CPU exactly which registers to move data into and which hardware ports to poke.
*   **`linker.ld` (The Linker Script)** is the **Blueprint**. It tells the hardware where our code and data "live" in the physical memory chips.

---

## 1. The Assembly (`kernel.S`): Talking to the Silicon

### The Abstraction: "Manual Variable Management"
Think of registers (`a0`, `a1`, `a2`) as the only three variables you are allowed to have. You don't have names like `user_count`; you just have "Slot A" and "Slot B". You have to manually move data from RAM into these slots to do any math or logic.

### Deep Explanation
This code performs a classic "Hello World" by talking to a **UART** (a hardware component that sends text to your terminal).

1.  **`_start`**: This is the global entry point. When the CPU powers on, it's hardwired to look for the first instruction here.
2.  **Hardware Addresses**: We define `UART_BASE` as `0x10000000`. In the bare-metal world, hardware components are "mapped" to memory addresses. Writing a byte to this specific address doesn't save it to RAM; it sends it to your screen.
3.  **The Print Loop**:
    *   `lb a2, 0(a1)`: Load a byte from our string into register `a2`.
    *   `sb a2, 0(a0)`: "Store" (send) that byte to the UART address.
    *   `addi a1, a1, 1`: Manually move the pointer to the next character in memory.
4.  **`wfi` (Wait For Interrupt)**: This tells the CPU to sleep until something happens. Since nothing else is happening, we loop here forever to prevent the CPU from executing random "junk" memory.

---

## 2. The Linker Script (`linker.ld`): The Map of the World

### The Abstraction: "The Floor Plan"
Imagine you are building a house. You have a pile of wood (Code) and a pile of bricks (Data). The Linker Script is the floor plan that says: "The kitchen (Code) goes on the North side, and the storage room (Data) goes in the basement." 

Without this, the CPU wouldn't know where the "Kitchen" starts, and it might try to execute a "Brick" as if it were a "Piece of Wood," causing a crash.

### Deep Explanation
The linker script organizes the final binary file (`kernel.elf`) into sections that the hardware expects:

1.  **`MEMORY` block**: Defines that our RAM starts at `0x80000000`. This is a physical requirement of the QEMU emulator we are using.
2.  **`.text`**: This is where the actual executable instructions from `kernel.S` are placed.
3.  **`.rodata` (Read-Only Data)**: This is where our "Hello World" string is stored. It's separated from the code because the CPU doesn't need to "execute" a string; it just needs to read it.
4.  **`ENTRY(_start)`**: Explicitly tells the linker that the very first instruction the CPU should run is located at the `_start` label in our assembly.

---

## How they work together

1.  The **Assembly** defines the logic (What to do).
2.  The **Linker Script** defines the location (Where to be).
3.  The **Makefile** uses a compiler to smash them together into `kernel.elf`, which is a complete, standalone package that can be "booted" by a RISC-V processor or emulator.
