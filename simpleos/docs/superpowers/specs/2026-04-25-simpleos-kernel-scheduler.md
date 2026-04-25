# Design Spec: Simple RISC-V Kernel Scheduler with SV39 Paging

**Date:** 2026-04-25
**Topic:** Preemptive Round-Robin Scheduler for RISC-V QEMU
**Status:** Approved

## 1. Goal
Implement a minimal RISC-V 64-bit kernel that supports:
- SV39 Page Table management.
- Preemptive Round-Robin scheduling.
- User-mode execution (U-mode).
- Two isolated example programs running at the same virtual address (`0x0`).

## 2. Architecture
- **Machine:** QEMU `virt` machine (`-M virt -cpu rv64`).
- **Privilege Levels:**
    - **S-mode (Supervisor):** Kernel code, interrupt handling, page table management.
    - **U-mode (User):** Example programs.
- **Memory Management (SV39):**
    - 3-level page table (L2 -> L1 -> L0).
    - **Kernel Mapping:** Identity mapped (VA = PA) for physical memory access.
    - **User Mapping:** Each process has a private root page table. Code/Data starts at virtual `0x0`.
- **Preemption:** RISC-V Timer Interrupts (via CLINT/MTIMER) triggering S-mode external interrupts.

## 3. Key Data Structures

### 3.1 `struct trap_frame`
Saved on the **Kernel Stack** of a task when a trap (interrupt/syscall) occurs.
- Contains all 31 general-purpose registers (x1-x31).
- Contains `sepc` (user program counter) and `sstatus`.

### 3.2 `struct context`
Used by the scheduler to switch between kernel threads.
- Contains callee-saved registers: `ra`, `sp`, `s0-s11`.

### 3.3 `struct task`
```c
struct task {
    enum { UNUSED, READY, RUNNING, ZOMBIE } state;
    int pid;
    uint64_t satp;           // Page table root (SATP value)
    uint64_t kstack;         // Pointer to top of kernel stack
    struct context context;  // Saved kernel context
};
```

## 4. Components

### 4.1 Memory Allocator
- A simple "Bump Allocator" or "Page Pool" that hands out 4KB physical pages starting after the kernel binary.

### 4.2 Page Table Manager
- Functions to create a new SV39 table.
- `map_page(root, va, pa, perm)`: Maps a 4KB virtual page to a physical one.

### 4.3 Trap Handler
- **Timer Interrupt:** Calls `scheduler()`.
- **Syscall (ecall):** Handles `print_char` (syscall #1) by writing to the UART.

### 4.4 Scheduler
- `scheduler()`: Finds the next `READY` task and calls `switch_to`.
- `switch_to(old_context, new_context)`: Assembly routine to swap stack pointers and callee-saved registers.

## 5. Example Programs
Two small functions hardcoded in the kernel image:
- **Task A:** `while(1) { syscall_print('A'); delay(); }`
- **Task B:** `while(1) { syscall_print('B'); delay(); }`
Both will be mapped to virtual address `0x0` in their respective address spaces.

## 6. Testing & Validation
1. **Compilation:** Use `riscv64-unknown-elf-gcc`.
2. **Execution:** `qemu-system-riscv64 -machine virt -nographic -kernel kernel.elf`.
3. **Success Criteria:** The terminal should output an alternating sequence of 'A' and 'B' (e.g., `AAABBBBAAA...`), proving that preemption and context switching are working.
